//! Stack frame filter example
//!
//! This filter excludes events with specific stack frames.

use aperture_filter::*;

filter_fn!(stack_filter, |input: &FilterInput| {
    // Drop events with very short stack traces (likely noise)
    if input.stack_trace.len() < 3 {
        return FilterResult::Drop;
    }
    
    // Keep all other events
    FilterResult::Keep
});
