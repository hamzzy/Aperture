//! Symbol resolution
//!
//! Resolves instruction pointers to function names, file names, and line numbers

use anyhow::Result;
use aperture_shared::types::profile::{Frame, LockProfile, Profile, Stack};
use blazesym::symbolize::source::{Kernel, Process, Source};
use blazesym::symbolize::{Input, Symbolized, Symbolizer};
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

    /// Symbolize a profile by resolving all instruction pointers.
    ///
    /// Splits IPs into kernel (high addresses) vs user (low addresses) and resolves
    /// each set with the appropriate source.
    pub fn symbolize_profile(&mut self, profile: &mut Profile, pid: Option<i32>) -> Result<()> {
        debug!("Symbolizing {} unique stacks", profile.samples.len());

        let mut user_ips: Vec<u64> = Vec::new();
        let mut kernel_ips: Vec<u64> = Vec::new();
        for stack in profile.samples.keys() {
            for frame in &stack.frames {
                let ip = frame.ip;
                if self.cache.contains_key(&ip) {
                    continue;
                }
                if ip >= 0xffff_0000_0000_0000 {
                    if !kernel_ips.contains(&ip) {
                        kernel_ips.push(ip);
                    }
                } else if !user_ips.contains(&ip) {
                    user_ips.push(ip);
                }
            }
        }

        debug!(
            "Resolving {} user IPs + {} kernel IPs",
            user_ips.len(),
            kernel_ips.len(),
        );

        // Resolve kernel IPs using /proc/kallsyms
        if !kernel_ips.is_empty() {
            if let Err(e) = self.symbolize_ips(&kernel_ips, None) {
                warn!("Failed to symbolize kernel IPs: {}", e);
            }
        }

        // Resolve user IPs
        if !user_ips.is_empty() {
            if let Some(pid) = pid {
                if let Err(e) = self.symbolize_ips(&user_ips, Some(pid)) {
                    warn!("Failed to symbolize user IPs for PID {}: {}", pid, e);
                }
            } else {
                self.resolve_user_ips_systemwide(&user_ips);
            }
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

    /// Symbolize a lock profile (same kernel/user split as symbolize_profile)
    pub fn symbolize_lock_profile(
        &mut self,
        profile: &mut LockProfile,
        pid: Option<i32>,
    ) -> Result<()> {
        debug!(
            "Symbolizing {} unique contention stacks",
            profile.contentions.len()
        );

        let mut user_ips: Vec<u64> = Vec::new();
        let mut kernel_ips: Vec<u64> = Vec::new();
        for (_, stack) in profile.contentions.keys() {
            for frame in &stack.frames {
                let ip = frame.ip;
                if self.cache.contains_key(&ip) {
                    continue;
                }
                if ip >= 0xffff_0000_0000_0000 {
                    if !kernel_ips.contains(&ip) {
                        kernel_ips.push(ip);
                    }
                } else if !user_ips.contains(&ip) {
                    user_ips.push(ip);
                }
            }
        }

        if !kernel_ips.is_empty() {
            if let Err(e) = self.symbolize_ips(&kernel_ips, None) {
                warn!("Failed to symbolize kernel IPs: {}", e);
            }
        }
        if !user_ips.is_empty() {
            if let Some(pid) = pid {
                if let Err(e) = self.symbolize_ips(&user_ips, Some(pid)) {
                    warn!("Failed to symbolize user IPs for PID {}: {}", pid, e);
                }
            } else {
                self.resolve_user_ips_systemwide(&user_ips);
            }
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
                                file: sym
                                    .module
                                    .as_ref()
                                    .and_then(|m| m.to_str())
                                    .map(String::from),
                                line: None, // Line info not available in this API version
                                module: sym
                                    .module
                                    .as_ref()
                                    .and_then(|m| m.to_str())
                                    .map(String::from),
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

    /// Best-effort resolution of userspace IPs when no target PID is specified.
    ///
    /// Scans /proc for running process PIDs and tries each one until all IPs are
    /// resolved (or we run out of PIDs to try). Each process may only resolve the
    /// IPs that belong to its address space.
    fn resolve_user_ips_systemwide(&mut self, ips: &[u64]) {
        use std::fs;

        let unresolved: Vec<u64> = ips
            .iter()
            .filter(|ip| !self.cache.contains_key(ip))
            .copied()
            .collect();

        if unresolved.is_empty() {
            return;
        }

        // Collect PIDs from /proc (numeric directory entries)
        let mut pids: Vec<i32> = Vec::new();
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if let Ok(pid) = name.parse::<i32>() {
                        pids.push(pid);
                    }
                }
            }
        }

        if pids.is_empty() {
            warn!("No PIDs found in /proc — cannot resolve userspace symbols in system-wide mode");
            return;
        }

        debug!(
            "System-wide mode: trying {} PIDs to resolve {} user IPs",
            pids.len(),
            unresolved.len()
        );

        let mut total_resolved = 0usize;
        for pid in &pids {
            // Only try IPs that are still unresolved
            let still_unresolved: Vec<u64> = unresolved
                .iter()
                .filter(|ip| {
                    self.cache.get(ip).map_or(true, |f| {
                        f.function.as_ref().map_or(true, |n| n.starts_with("0x"))
                    })
                })
                .copied()
                .collect();

            if still_unresolved.is_empty() {
                break;
            }

            // Try resolving with this PID's maps
            if let Ok(()) = self.symbolize_ips(&still_unresolved, Some(*pid)) {
                let newly_resolved = still_unresolved
                    .iter()
                    .filter(|ip| {
                        self.cache
                            .get(ip)
                            .and_then(|f| f.function.as_ref())
                            .is_some_and(|n| !n.starts_with("0x"))
                    })
                    .count();
                total_resolved += newly_resolved;
            }
        }

        debug!(
            "System-wide resolution: {}/{} user IPs resolved across {} processes",
            total_resolved,
            unresolved.len(),
            pids.len()
        );
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

impl Default for SymbolCache {
    fn default() -> Self {
        Self::new()
    }
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
        let (mut user_resolved, mut kernel_resolved) = (0u32, 0u32);
        if !user_ips.is_empty() || !kernel_ips.is_empty() {
            let symbolizer = Symbolizer::new();
            if !user_ips.is_empty() {
                user_resolved = Self::resolve_ips(&symbolizer, &mut self.cache, &user_ips, pid);
            }
            if !kernel_ips.is_empty() {
                kernel_resolved =
                    Self::resolve_ips(&symbolizer, &mut self.cache, &kernel_ips, None);
            }
            // symbolizer is dropped here — Rc freed before any .await
        }
        if !user_ips.is_empty() || !kernel_ips.is_empty() {
            debug!(
                "Symbol resolution: {}/{} user IPs, {}/{} kernel IPs resolved (cache: {} entries)",
                user_resolved,
                user_ips.len(),
                kernel_resolved,
                kernel_ips.len(),
                self.cache.len(),
            );
        }

        // 3. Populate symbol fields on each event (encoding module info into the string)
        for event in events.iter_mut() {
            match event {
                ProfileEvent::CpuSample(s) => {
                    s.user_stack_symbols = s
                        .user_stack
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(Self::encode_symbol))
                        .collect();
                    s.kernel_stack_symbols = s
                        .kernel_stack
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(Self::encode_symbol))
                        .collect();
                }
                ProfileEvent::Lock(ev) => {
                    ev.stack_symbols = ev
                        .stack_trace
                        .iter()
                        .map(|ip| self.cache.get(ip).and_then(Self::encode_symbol))
                        .collect();
                }
                _ => {}
            }
        }
    }

    /// Encode a cached Frame into a symbol string for the wire protocol.
    /// Format: "function_name [module_basename]" when module is available.
    /// This allows the UI to parse out the module info.
    fn encode_symbol(frame: &Frame) -> Option<String> {
        match (&frame.function, &frame.module) {
            (Some(func), Some(module)) if !module.is_empty() => {
                let basename = module.rsplit('/').next().unwrap_or(module);
                Some(format!("{} [{}]", func, basename))
            }
            (Some(func), _) => Some(func.clone()),
            _ => None,
        }
    }

    /// Returns the number of successfully resolved IPs.
    fn resolve_ips(
        symbolizer: &Symbolizer,
        cache: &mut HashMap<u64, Frame>,
        ips: &[u64],
        pid: Option<i32>,
    ) -> u32 {
        let input = Input::AbsAddr(ips);
        let source_label = if pid.is_some() { "user" } else { "kernel" };
        let source = if let Some(pid) = pid {
            Source::Process(Process::new(Pid::from(pid as u32)))
        } else {
            Source::Kernel(Kernel::default())
        };

        match symbolizer.symbolize(&source, input) {
            Ok(results) => {
                let mut resolved = 0u32;
                for (i, result) in results.iter().enumerate() {
                    let ip = ips[i];
                    let frame = match result {
                        Symbolized::Sym(sym) => {
                            resolved += 1;
                            Frame {
                                ip,
                                function: Some(sym.name.to_string()),
                                file: sym
                                    .module
                                    .as_ref()
                                    .and_then(|m| m.to_str())
                                    .map(String::from),
                                line: None,
                                module: sym
                                    .module
                                    .as_ref()
                                    .and_then(|m| m.to_str())
                                    .map(String::from),
                            }
                        }
                        Symbolized::Unknown(reason) => {
                            debug!(
                                ip = format_args!("0x{:x}", ip),
                                source = source_label,
                                reason = ?reason,
                                "Unresolved IP"
                            );
                            Frame {
                                ip,
                                function: Some(format!("0x{:x}", ip)),
                                file: None,
                                line: None,
                                module: None,
                            }
                        }
                    };
                    cache.insert(ip, frame);
                }
                resolved
            }
            Err(e) => {
                warn!(
                    "Failed to symbolize {} batch ({} IPs): {}",
                    source_label,
                    ips.len(),
                    e
                );
                for &ip in ips {
                    cache.entry(ip).or_insert_with(|| Frame {
                        ip,
                        function: Some(format!("0x{:x}", ip)),
                        file: None,
                        line: None,
                        module: None,
                    });
                }
                0
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
