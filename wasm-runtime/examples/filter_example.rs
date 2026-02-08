//! Example WASM filter
//!
//! This example shows how to write a filter in Rust that compiles to WASM

// TODO: Create example filter
// Example: Filter events from specific PIDs or with specific stack patterns

fn main() {
    println!("TODO: Implement example WASM filter");
    println!("Compile with: cargo build --target wasm32-wasi");
}

// Example filter function
#[no_mangle]
pub extern "C" fn filter_event() -> bool {
    // TODO: Implement filter logic
    // Return true to keep event, false to discard
    true
}
