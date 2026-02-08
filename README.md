# Aperture

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](#license)

> Production-grade distributed profiler for CPU & GPU workloads built with Rust and eBPF

## Overview

Aperture is a high-performance profiling system that uses eBPF for low-overhead performance monitoring. The project aims to provide CPU, GPU(WIP), lock contention, and syscall profiling in a distributed architecture.

<img src ="./intro.png"/>

## Features (Planned)

- **CPU Profiling**: Continuous stack trace sampling with <1% overhead using eBPF
- **Lock Contention**: Detect mutex/futex bottlenecks in production
- **Syscall Tracing**: Latency histograms for system calls
- **WASM Filters**: Programmable event filtering
- **GPU Profiling**: CUDA kernel tracing and memory analysis
- **Distributed Architecture**: Agent-aggregator model with scalable storage

## Repository Structure

```
aperture/
├── agent/           # Main profiling agent
├── agent-ebpf/      # eBPF programs (kernel space)
├── shared/          # Common types and utilities
├── cli/             # Command-line interface
├── aggregator/      # Central aggregation service (future)
├── wasm-runtime/    # WASM filter runtime (future)
└── gpu-profiler/    # GPU profiling support (future)
```

## Prerequisites

- Linux kernel 5.8+ with BPF support
- Rust 1.75+ (stable for userspace, nightly for eBPF programs)
- LLVM 14+ and Clang
- Linux headers for your kernel

## Getting Started

```bash
# Clone the repository
git clone https://github.com/yourusername/aperture.git
cd aperture

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Check code quality
cargo clippy --workspace --all-targets
cargo fmt --check
```

## Development

This project is in early development. See [docs/roadmap.md](docs/roadmap.md) for the development plan.

### Workspace Crates

- **agent**: Main profiling agent that loads eBPF programs and collects performance data
- **agent-ebpf**: eBPF programs compiled to BPF bytecode
- **shared**: Common types, protocols, and utilities shared across crates
- **cli**: Command-line interface for profiling operations
- **aggregator**: Distributed aggregation service (future phase)
- **wasm-runtime**: WASM-based event filtering (future phase)
- **gpu-profiler**: GPU profiling capabilities (future phase)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) once available.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Acknowledgments

Built with:

- [Aya](https://aya-rs.dev/) - Rust eBPF library
- [Tokio](https://tokio.rs/) - Async runtime
- [Inferno](https://github.com/jonhoo/inferno) - Flamegraph generation
