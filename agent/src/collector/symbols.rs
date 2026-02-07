//! Symbol resolution
//!
//! Resolves instruction pointers to function names, file names, and line numbers

use anyhow::Result;
use aperture_shared::types::profile::{Frame, Profile, Stack};
use blazesym::symbolize::{Input, Symbolized, Symbolizer};
use blazesym::symbolize::source::{Kernel, Process, Source};
use blazesym::Pid;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Symbol resolver using blazesym
pub struct SymbolResolver {
    /// Blazesym symbolizer
    symbolizer: Symbolizer,

    /// Cache of resolved symbols: IP -> Frame
    cache: HashMap<u64, Frame>,
}

impl SymbolResolver {
    /// Create a new symbol resolver
    pub fn new() -> Self {
        Self {
            symbolizer: Symbolizer::new(),
            cache: HashMap::new(),
        }
    }

    /// Symbolize a profile by resolving all instruction pointers
    pub fn symbolize_profile(&mut self, profile: &mut Profile, pid: Option<i32>) -> Result<()> {
        debug!("Symbolizing {} unique stacks", profile.samples.len());

        // Collect all unique IPs from all stacks
        let mut all_ips: Vec<u64> = Vec::new();
        for stack in profile.samples.keys() {
            for frame in &stack.frames {
                if !self.cache.contains_key(&frame.ip) && !all_ips.contains(&frame.ip) {
                    all_ips.push(frame.ip);
                }
            }
        }

        debug!("Resolving {} unique instruction pointers", all_ips.len());

        // Symbolize IPs in batch
        if !all_ips.is_empty() {
            self.symbolize_ips(&all_ips, pid)?;
        }

        // Replace stacks with symbolized versions
        let mut new_samples = HashMap::new();
        for (stack, count) in profile.samples.drain() {
            let symbolized_stack = self.symbolize_stack(&stack);
            *new_samples.entry(symbolized_stack).or_insert(0) += count;
        }
        profile.samples = new_samples;

        Ok(())
    }

    /// Symbolize a stack by looking up each frame
    fn symbolize_stack(&self, stack: &Stack) -> Stack {
        let symbolized_frames: Vec<Frame> = stack
            .frames
            .iter()
            .map(|frame| {
                self.cache
                    .get(&frame.ip)
                    .cloned()
                    .unwrap_or_else(|| frame.clone())
            })
            .collect();

        Stack {
            frames: symbolized_frames,
        }
    }

    /// Symbolize a batch of instruction pointers
    fn symbolize_ips(&mut self, ips: &[u64], pid: Option<i32>) -> Result<()> {
        // Create input for symbolizer
        let input = Input::AbsAddr(ips);

        // Determine source (kernel vs userspace)
        let source = if let Some(pid) = pid {
            // For userspace processes
            Source::Process(Process::new(Pid::from(pid as u32)))
        } else {
            // For kernel addresses
            Source::Kernel(Kernel::default())
        };

        // Symbolize
        match self.symbolizer.symbolize(&source, input) {
            Ok(results) => {
                // Process results
                for (i, result) in results.iter().enumerate() {
                    let ip = ips[i];

                    let frame = match result {
                        Symbolized::Sym(sym) => {
                            // Successfully symbolized
                            Frame {
                                ip,
                                function: Some(sym.name.to_string()),
                                file: sym.module.as_ref().and_then(|m| m.to_str()).map(String::from),
                                line: None, // Line info not available in this API version
                                module: sym.module.as_ref().and_then(|m| m.to_str()).map(String::from),
                            }
                        }
                        Symbolized::Unknown(_) => {
                            // Could not symbolize - use hex address as function name
                            Frame {
                                ip,
                                function: Some(format!("0x{:x}", ip)),
                                file: None,
                                line: None,
                                module: None,
                            }
                        }
                    };

                    self.cache.insert(ip, frame);
                }
            }
            Err(e) => {
                warn!("Failed to symbolize batch: {}", e);
                // Add unresolved frames to cache
                for &ip in ips {
                    self.cache.entry(ip).or_insert_with(|| Frame {
                        ip,
                        function: Some(format!("0x{:x}", ip)),
                        file: None,
                        line: None,
                        module: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// Get cache size (number of resolved symbols)
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for SymbolResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = SymbolResolver::new();
        assert_eq!(resolver.cache_size(), 0);
    }
}
