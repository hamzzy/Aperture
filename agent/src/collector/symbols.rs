//! Symbol resolution
//!
//! Resolves instruction pointers to function names, file names, and line numbers

use anyhow::Result;
use aperture_shared::types::profile::{Frame, LockProfile, Profile, Stack};
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

    /// Symbolize a lock profile
    pub fn symbolize_lock_profile(&mut self, profile: &mut LockProfile, pid: Option<i32>) -> Result<()> {
        debug!("Symbolizing {} unique contention stacks", profile.contentions.len());

        // Collect all unique IPs
        let mut all_ips: Vec<u64> = Vec::new();
        for (_, stack) in profile.contentions.keys() {
            for frame in &stack.frames {
                if !self.cache.contains_key(&frame.ip) && !all_ips.contains(&frame.ip) {
                    all_ips.push(frame.ip);
                }
            }
        }

        if !all_ips.is_empty() {
            self.symbolize_ips(&all_ips, pid)?;
        }

        // Replace stacks
        let mut new_contentions = HashMap::new();
        for ((lock_addr, stack), stats) in profile.contentions.drain() {
            let symbolized_stack = self.symbolize_stack(&stack);
            new_contentions.insert((lock_addr, symbolized_stack), stats);
        }
        profile.contentions = new_contentions;

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

    /// Resolve symbols for a batch of ProfileEvents in-place.
    /// Populates `user_stack_symbols`/`kernel_stack_symbols` on CpuSample
    /// and `stack_symbols` on LockEvent so the aggregator receives symbolized data.
    pub fn symbolize_events(
        &mut self,
        events: &mut [aperture_shared::types::events::ProfileEvent],
        pid: Option<i32>,
    ) {
        use aperture_shared::types::events::ProfileEvent;

        // 1. Collect all unique IPs that need resolution, separated by address space
        let mut user_ips: Vec<u64> = Vec::new();
        let mut kernel_ips: Vec<u64> = Vec::new();
        for event in events.iter() {
            match event {
                ProfileEvent::CpuSample(s) => {
                    for &ip in &s.user_stack {
                        if !self.cache.contains_key(&ip) && !user_ips.contains(&ip) {
                            user_ips.push(ip);
                        }
                    }
                    for &ip in &s.kernel_stack {
                        if !self.cache.contains_key(&ip) && !kernel_ips.contains(&ip) {
                            kernel_ips.push(ip);
                        }
                    }
                }
                ProfileEvent::Lock(ev) => {
                    for &ip in &ev.stack_trace {
                        // Lock stacks combine user+kernel; classify by address range
                        if self.cache.contains_key(&ip) {
                            continue;
                        }
                        let is_kernel = ip >= 0xffff_0000_0000_0000;
                        if is_kernel {
                            if !kernel_ips.contains(&ip) {
                                kernel_ips.push(ip);
                            }
                        } else if !user_ips.contains(&ip) {
                            user_ips.push(ip);
                        }
                    }
                }
                _ => {}
            }
        }

        // 2. Batch-resolve user IPs (needs target PID for /proc/PID/maps)
        if !user_ips.is_empty() {
            if let Err(e) = self.symbolize_ips(&user_ips, pid) {
                warn!("Failed to symbolize user IPs: {}", e);
            }
        }
        // 3. Batch-resolve kernel IPs (uses /proc/kallsyms)
        if !kernel_ips.is_empty() {
            if let Err(e) = self.symbolize_ips(&kernel_ips, None) {
                warn!("Failed to symbolize kernel IPs: {}", e);
            }
        }

        // 4. Populate symbol fields on each event
        for event in events.iter_mut() {
            match event {
                ProfileEvent::CpuSample(s) => {
                    s.user_stack_symbols = s
                        .user_stack
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(|f| f.function.clone()))
                        .collect();
                    s.kernel_stack_symbols = s
                        .kernel_stack
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(|f| f.function.clone()))
                        .collect();
                }
                ProfileEvent::Lock(ev) => {
                    ev.stack_symbols = ev
                        .stack_trace
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(|f| f.function.clone()))
                        .collect();
                }
                _ => {}
            }
        }
    }

    /// Get cache size (number of resolved symbols)
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

}

impl Default for SymbolResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// A Send-safe symbol cache that can be held across `.await` points.
///
/// blazesym's `Symbolizer` uses `Rc` internally and is not `Send`, so it cannot
/// live inside a `tokio::spawn` future. This type holds only the `HashMap` cache
/// (which IS Send) and creates a temporary `Symbolizer` on each resolution call.
pub struct SymbolCache {
    cache: HashMap<u64, Frame>,
}

impl SymbolCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Resolve symbols for a batch of ProfileEvents in-place.
    ///
    /// Creates a temporary `Symbolizer` for each call (cheap — no persistent state
    /// worth keeping), resolves unknown IPs, and populates the symbol fields.
    pub fn symbolize_events(
        &mut self,
        events: &mut [aperture_shared::types::events::ProfileEvent],
        pid: Option<i32>,
    ) {
        use aperture_shared::types::events::ProfileEvent;

        // 1. Collect all unique IPs that need resolution, separated by address space
        let mut user_ips: Vec<u64> = Vec::new();
        let mut kernel_ips: Vec<u64> = Vec::new();
        for event in events.iter() {
            match event {
                ProfileEvent::CpuSample(s) => {
                    for &ip in &s.user_stack {
                        if !self.cache.contains_key(&ip) && !user_ips.contains(&ip) {
                            user_ips.push(ip);
                        }
                    }
                    for &ip in &s.kernel_stack {
                        if !self.cache.contains_key(&ip) && !kernel_ips.contains(&ip) {
                            kernel_ips.push(ip);
                        }
                    }
                }
                ProfileEvent::Lock(ev) => {
                    for &ip in &ev.stack_trace {
                        if self.cache.contains_key(&ip) {
                            continue;
                        }
                        let is_kernel = ip >= 0xffff_0000_0000_0000;
                        if is_kernel {
                            if !kernel_ips.contains(&ip) {
                                kernel_ips.push(ip);
                            }
                        } else if !user_ips.contains(&ip) {
                            user_ips.push(ip);
                        }
                    }
                }
                _ => {}
            }
        }

        // 2. Batch-resolve IPs using a temporary Symbolizer
        if !user_ips.is_empty() || !kernel_ips.is_empty() {
            let symbolizer = Symbolizer::new();
            if !user_ips.is_empty() {
                Self::resolve_ips(&symbolizer, &mut self.cache, &user_ips, pid);
            }
            if !kernel_ips.is_empty() {
                Self::resolve_ips(&symbolizer, &mut self.cache, &kernel_ips, None);
            }
            // symbolizer is dropped here — Rc freed before any .await
        }

        // 3. Populate symbol fields on each event
        for event in events.iter_mut() {
            match event {
                ProfileEvent::CpuSample(s) => {
                    s.user_stack_symbols = s
                        .user_stack
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(|f| f.function.clone()))
                        .collect();
                    s.kernel_stack_symbols = s
                        .kernel_stack
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(|f| f.function.clone()))
                        .collect();
                }
                ProfileEvent::Lock(ev) => {
                    ev.stack_symbols = ev
                        .stack_trace
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(|f| f.function.clone()))
                        .collect();
                }
                _ => {}
            }
        }
    }

    fn resolve_ips(
        symbolizer: &Symbolizer,
        cache: &mut HashMap<u64, Frame>,
        ips: &[u64],
        pid: Option<i32>,
    ) {
        let input = Input::AbsAddr(ips);
        let source = if let Some(pid) = pid {
            Source::Process(Process::new(Pid::from(pid as u32)))
        } else {
            Source::Kernel(Kernel::default())
        };

        match symbolizer.symbolize(&source, input) {
            Ok(results) => {
                for (i, result) in results.iter().enumerate() {
                    let ip = ips[i];
                    let frame = match result {
                        Symbolized::Sym(sym) => Frame {
                            ip,
                            function: Some(sym.name.to_string()),
                            file: sym.module.as_ref().and_then(|m| m.to_str()).map(String::from),
                            line: None,
                            module: sym.module.as_ref().and_then(|m| m.to_str()).map(String::from),
                        },
                        Symbolized::Unknown(_) => Frame {
                            ip,
                            function: Some(format!("0x{:x}", ip)),
                            file: None,
                            line: None,
                            module: None,
                        },
                    };
                    cache.insert(ip, frame);
                }
            }
            Err(e) => {
                warn!("Failed to symbolize batch: {}", e);
                for &ip in ips {
                    cache.entry(ip).or_insert_with(|| Frame {
                        ip,
                        function: Some(format!("0x{:x}", ip)),
                        file: None,
                        line: None,
                        module: None,
                    });
                }
            }
        }
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

    #[test]
    fn test_symbol_cache_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<SymbolCache>();
    }
}
