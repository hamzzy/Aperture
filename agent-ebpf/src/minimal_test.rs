#![no_std]
#![no_main]

use aya_ebpf::{macros::perf_event, programs::PerfEventContext};

#[no_mangle]
#[link_section = "license"]
pub static LICENSE: [u8; 4] = *b"GPL\0";

#[perf_event]
pub fn minimal_test(_ctx: PerfEventContext) -> i64 {
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
