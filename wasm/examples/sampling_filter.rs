//! Sampling filter example
//!
//! This filter implements time-based sampling (keep only 10% of events).

use aperture_filter::*;

filter_fn!(sampling_filter, |input: &FilterInput| {
    // Keep only events where timestamp mod 10 == 0 (10% sampling)
    if input.timestamp % 10 == 0 {
        FilterResult::Keep
    } else {
        FilterResult::Drop
    }
});
