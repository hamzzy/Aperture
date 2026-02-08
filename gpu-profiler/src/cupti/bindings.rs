//! CUPTI FFI bindings

// TODO : Generate FFI bindings for CUPTI
// Use bindgen or manual bindings for CUPTI headers
// Key functions:
// - cuptiSubscribe()
// - cuptiEnableDomain()
// - cuptiActivityEnable()
// - cuptiActivityGetNextRecord()

#![allow(dead_code)]

// Placeholder types
pub type CUptiResult = u32;
pub type CUpti_SubscriberHandle = *mut std::ffi::c_void;

pub const CUPTI_SUCCESS: CUptiResult = 0;
