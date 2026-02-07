//! Process name filter example
//!
//! This filter only keeps events from processes matching a specific name.

use aperture_filter::*;

filter_fn!(process_filter, |input: &FilterInput| {
    // Only keep events from processes containing "python"
    if input.comm.contains("python") {
        FilterResult::Keep
    } else {
        FilterResult::Drop
    }
});
