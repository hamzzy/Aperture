//! Aperture WASM Filter SDK
//!
//! This SDK provides the tools to write custom filters for Aperture profiling events.

pub use aperture_shared::wasm::{FilterInput, FilterResult};

use std::alloc::{alloc, dealloc, Layout};
use std::slice;

/// Allocate memory for WASM host
#[no_mangle]
pub extern "C" fn alloc(size: u32) -> *mut u8 {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { alloc(layout) }
}

/// Deallocate memory for WASM host
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: u32) {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { dealloc(ptr, layout) }
}

/// Helper macro to define a filter function
///
/// # Example
/// ```
/// use aperture_filter::*;
///
/// filter_fn!(my_filter, |input: &FilterInput| {
///     if input.comm.contains("python") {
///         FilterResult::Keep
///     } else {
///         FilterResult::Drop
///     }
/// });
/// ```
#[macro_export]
macro_rules! filter_fn {
    ($name:ident, $body:expr) => {
        #[no_mangle]
        pub extern "C" fn filter(input_ptr: u32, input_len: u32) -> u32 {
            use $crate::{FilterInput, FilterResult};
            
            // Read input
            let input_bytes = unsafe {
                std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize)
            };
            
            let input: FilterInput = bincode::deserialize(input_bytes).unwrap();
            
            // Execute filter
            let result: FilterResult = $body(&input);
            
            // Serialize result
            let output_bytes = bincode::serialize(&result).unwrap();
            let output_len = output_bytes.len() as u32;
            
            // Allocate output buffer (4 bytes for length + data)
            let output_ptr = $crate::alloc(output_len + 4);
            
            // Write length
            unsafe {
                *(output_ptr as *mut u32) = output_len;
                std::ptr::copy_nonoverlapping(
                    output_bytes.as_ptr(),
                    output_ptr.offset(4),
                    output_len as usize,
                );
            }
            
            output_ptr as u32
        }
    };
}

/// Log a message from the filter (for debugging)
#[allow(dead_code)]
pub fn log(msg: &str) {
    extern "C" {
        fn log(ptr: u32, len: u32);
    }
    
    unsafe {
        log(msg.as_ptr() as u32, msg.len() as u32);
    }
}

/// Get current timestamp from host
#[allow(dead_code)]
pub fn get_timestamp() -> u64 {
    extern "C" {
        fn get_timestamp() -> u64;
    }
    
    unsafe { get_timestamp() }
}
