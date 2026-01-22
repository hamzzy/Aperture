//! Generated kernel type definitions
//!
//! This file should be auto-generated from /sys/kernel/btf/vmlinux using aya-gen
//! or bpftool. For now, we include minimal manually-defined types.
//!
//! TODO Phase 1: Implement automatic BTF type generation in build.rs

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Placeholder kernel types
// In production, these should be generated from BTF

pub type __u32 = u32;
pub type __u64 = u64;
pub type __s32 = i32;
pub type __s64 = i64;

// TODO: Add kernel structure definitions as needed
// Example:
// #[repr(C)]
// pub struct task_struct {
//     pub pid: i32,
//     pub tgid: i32,
//     pub comm: [u8; 16],
// }
