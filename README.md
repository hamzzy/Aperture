# Aperture

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0%20OR%20MIT-blue.svg)](#license)

> Production-grade distributed profiler for CPU & GPU workloads built with Rust and eBPF

## Overview

Aperture is a high-performance profiling system that uses eBPF for low-overhead performance monitoring. It supports CPU sampling, lock contention tracing, and syscall analysis in a distributed agent-aggregator architecture.

<img src ="./intro.png"/>

## Features

- **CPU Profiling** — Continuous stack trace sampling with <1% overhead using eBPF perf events
- **Lock Contention** — Detect mutex/futex bottlenecks via tracepoint-based tracking
- **Syscall Tracing** — Latency histograms and error analysis for all system calls
- **Symbol Resolution** — Automatic kernel + userspace symbol resolution with blazesym
- **Distributed Architecture** — Agent-aggregator model with gRPC transport and ClickHouse storage
- **Web Dashboard** — Interactive flamegraphs, top functions, syscall analysis, differential profiling
- **Alert Engine** — Configurable threshold alerts on buffer, error, and throughput metrics
- **Data Export** — JSON and collapsed-stack format export for integration with external tools
- **Prometheus Metrics** — Built-in `/metrics` endpoint for aggregator observability
- **WASM Filters** — Programmable event filtering with WebAssembly (wasmtime)

## Quick Start

### Install from Source

```bash
git clone https://github.com/yourusername/aperture.git
cd aperture
cargo build --release --bin aperture-aggregator --bin aperture-cli
```

### Install from Release

```bash
curl -fsSL https://raw.githubusercontent.com/yourusername/aperture/main/scripts/install.sh | bash
```

### Docker

```bash
docker compose up -d    # Starts ClickHouse + Aggregator + Agent
```

### Run the Agent (Linux, requires root)

```bash
# CPU profiling (system-wide, 99 Hz, 30 seconds)
sudo ./target/release/aperture-agent --mode cpu --duration 30s --freq 99

# With aggregator push
sudo ./target/release/aperture-agent --mode all --aggregator http://localhost:50051 --duration 24h

# PID-filtered
sudo ./target/release/aperture-agent --mode cpu --pid 1234 --duration 10s
```

### Run the Web UI

```bash
cd ui && npm install && npm run dev
# Open http://localhost:5173 (proxies /api to aggregator at :9090)
```

## Repository Structure

```
aperture/
├── agent/              # Userspace profiling agent (loads eBPF, resolves symbols)
├── agent-ebpf/         # eBPF programs (cpu-profiler, lock-profiler, syscall-tracer)
├── shared/             # Shared types, wire protocol, utilities
├── aggregator/         # Central aggregation service (gRPC + HTTP REST API)
├── cli/                # CLI for querying aggregator (query, aggregate, diff)
├── wasm-runtime/       # WASM filter runtime (wasmtime-based)
├── gpu-profiler/       # GPU profiling support (CUDA/CUPTI, WIP)
├── ui/                 # React web dashboard (Vite + Tailwind + shadcn/ui)
├── deploy/k8s/         # Kubernetes manifests (DaemonSet + Deployment)
├── scripts/            # Demo and setup scripts
├── docs/               # Documentation
│   ├── API-REFERENCE.md
│   ├── ARCHITECTURE.md
│   ├── SYMBOL-RESOLUTION.md
│   ├── RUN-EXAMPLES.md
│   └── roadmap.md
├── docker-compose.yml  # Full-stack Docker setup
├── Dockerfile.agent
└── Dockerfile.aggregator
```

## Documentation

- [Architecture Overview](docs/ARCHITECTURE.md) — System design, data flow, component details
- [API Reference](docs/API-REFERENCE.md) — REST API endpoints, gRPC RPCs, Prometheus metrics
- [Run Examples](docs/RUN-EXAMPLES.md) — Agent modes, Docker setup, CLI commands
- [Symbol Resolution](docs/SYMBOL-RESOLUTION.md) — Debug symbol setup and troubleshooting
- [Development Roadmap](docs/roadmap.md) — Phase-by-phase development plan

## Development

```bash
# Build all workspace crates
cargo build --workspace

# Run tests
cargo test -p aperture-agent -p aperture-aggregator -p aperture-shared -p aperture-cli -p aperture-wasm

# Lint
cargo clippy --workspace --all-targets
cargo fmt --check

# Build UI
cd ui && npm run build
```

### Building eBPF Programs

eBPF programs require nightly Rust and a Linux target:

```bash
cargo +nightly build -Zbuild-std=core --target bpfel-unknown-none \
  --bin cpu-profiler --bin lock-profiler --bin syscall-tracer
```

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

- [Aya](https://aya-rs.dev/) — Rust eBPF library
- [Tokio](https://tokio.rs/) — Async runtime
- [wasmtime](https://wasmtime.dev/) — WebAssembly runtime
- [Tonic](https://github.com/hyperium/tonic) — gRPC framework
- [ClickHouse](https://clickhouse.com/) — Column-oriented storage
- [Inferno](https://github.com/jonhoo/inferno) — Flamegraph generation
