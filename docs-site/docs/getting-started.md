---
sidebar_position: 2
title: Getting Started
---

# Getting Started

This guide walks through installing Aperture and running your first profiling session.

## Prerequisites

- **Linux host** with kernel 5.10+ (for eBPF support)
- **Root access** (eBPF programs require `CAP_BPF` or root)
- **Rust toolchain** (stable + nightly for eBPF programs)

## Install from Source

```bash
git clone https://github.com/yourusername/aperture.git
cd aperture

# Build aggregator and CLI (runs on any platform)
cargo build --release --bin aperture-aggregator --bin aperture-cli

# Build eBPF programs (requires nightly Rust, Linux target)
cargo +nightly build -Zbuild-std=core --target bpfel-unknown-none \
  --bin cpu-profiler --bin lock-profiler --bin syscall-tracer --release

# Build agent (Linux only)
cargo build --release --bin aperture-agent
```

## Install from Release

```bash
curl -fsSL https://raw.githubusercontent.com/yourusername/aperture/main/scripts/install.sh | bash
```

This downloads the latest release binaries for your platform and installs them to `/usr/local/bin`.

## Docker

The fastest way to get the full stack running:

```bash
docker compose up -d    # Starts ClickHouse + Aggregator + Agent
```

## Quick Profile (Local)

Run a 30-second CPU profile that generates a flamegraph SVG locally (no aggregator needed):

```bash
sudo ./target/release/aperture-agent \
  --mode cpu \
  --duration 30s \
  --output flamegraph.svg
```

Open `flamegraph.svg` in your browser.

## Full Stack Setup

### 1. Start ClickHouse + Aggregator

```bash
# Start ClickHouse in Docker
docker compose up -d clickhouse

# Run aggregator (connects to ClickHouse)
export APERTURE_CLICKHOUSE_ENDPOINT="http://127.0.0.1:8123"
export APERTURE_CLICKHOUSE_DATABASE="aperture"
cargo run --release -p aperture-aggregator --features clickhouse-storage
```

The aggregator exposes:
- **gRPC** on `:50051` (agent data ingestion)
- **HTTP** on `:9090` (REST API, metrics, health)

### 2. Run the Agent

```bash
# CPU profiling, push to aggregator, 24 hours
sudo ./target/release/aperture-agent \
  --mode cpu \
  --aggregator http://localhost:50051 \
  --duration 24h

# Profile a specific process
sudo ./target/release/aperture-agent \
  --mode cpu \
  --pid 1234 \
  --aggregator http://localhost:50051 \
  --duration 10m

# All modes simultaneously
sudo ./target/release/aperture-agent \
  --mode all \
  --aggregator http://localhost:50051 \
  --duration 1h
```

### 3. Open the Dashboard

```bash
cd ui && npm install && npm run dev
```

Open [http://localhost:5173](http://localhost:5173) in your browser. The UI proxies API requests to the aggregator at `:9090`.

### 4. Query with CLI

```bash
# Query in-memory buffer
cargo run -p aperture-cli -- query \
  --endpoint http://127.0.0.1:50051 --limit 10

# Aggregate CPU events from storage
cargo run -p aperture-cli -- aggregate \
  --endpoint http://127.0.0.1:50051 --event_type cpu --limit 100
```

## Agent Modes

| Mode | Flag | Description |
|------|------|-------------|
| CPU | `--mode cpu` | Stack trace sampling via perf events (default 99 Hz) |
| Lock | `--mode lock` | Futex contention tracing via tracepoints |
| Syscall | `--mode syscall` | All syscall latency/error tracking |
| All | `--mode all` | Run all three modes concurrently |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `APERTURE_AUTH_TOKEN` | (none) | Bearer token for aggregator auth |
| `APERTURE_CLICKHOUSE_ENDPOINT` | (none) | ClickHouse HTTP URL |
| `APERTURE_CLICKHOUSE_DATABASE` | `aperture` | ClickHouse database name |
| `APERTURE_CLICKHOUSE_PASSWORD` | (none) | ClickHouse password |

## Next Steps

- [Run Examples](./guides/run-examples) — More detailed usage scenarios
- [Symbol Resolution](./guides/symbol-resolution) — Fix unresolved hex addresses
- [Kubernetes Deployment](./guides/kubernetes) — Deploy on a cluster
