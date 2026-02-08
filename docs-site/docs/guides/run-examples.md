---
sidebar_position: 1
title: Run Examples
---

# Run Examples

Detailed examples for running the agent, aggregator, and web UI in different setups.

## Agent (Linux only)

The agent needs Linux (eBPF). Run on a Linux host, in Docker, or in an OrbStack Ubuntu VM.

### Agent Modes

```bash
# CPU profiling (default), push to aggregator, 60 seconds
sudo ./target/release/aperture-agent \
  --mode cpu --duration 60s --aggregator http://HOST:50051

# CPU, 24 hours, 99 Hz (default)
sudo ./target/release/aperture-agent \
  --aggregator http://HOST:50051 --mode cpu --duration 24h

# CPU, profile a single process by PID
sudo ./target/release/aperture-agent \
  --mode cpu --pid 12345 --duration 5m --aggregator http://HOST:50051

# Lock contention (futex) profiling
sudo ./target/release/aperture-agent \
  --mode lock --duration 30s --aggregator http://HOST:50051

# Syscall tracing
sudo ./target/release/aperture-agent \
  --mode syscall --duration 30s --aggregator http://HOST:50051

# All modes simultaneously
sudo ./target/release/aperture-agent \
  --mode all --duration 30s --aggregator http://HOST:50051

# Local run, write flamegraph to file (no aggregator)
sudo ./target/release/aperture-agent \
  --mode cpu --duration 30s --output flamegraph.svg
```

Replace `HOST` with your aggregator host (e.g. `127.0.0.1`, `host.orb.internal` from OrbStack VM, or `aggregator` in Docker).

## OrbStack Ubuntu VM (from Mac)

The repo is available in the VM via path translation. Build once, then run the agent.

```bash
# 1) Build eBPF (nightly) and agent (stable) in the VM
orb run -m ubuntu -w /Users/user/aperture bash -c '\
  cargo +nightly build -p aperture-ebpf \
    --target bpfel-unknown-none -Z build-std=core --release && \
  cargo build -p aperture-agent --release'

# 2) Run agent (aggregator on Mac at host.orb.internal:50051)
orb run -m ubuntu -w /Users/user/aperture -u root bash -c '\
  sudo ./target/release/aperture-agent \
    --aggregator http://host.orb.internal:50051 \
    --mode cpu --duration 60s'

# 24h CPU profile
orb run -m ubuntu -w /Users/user/aperture -u root bash -c '\
  sudo ./target/release/aperture-agent \
    --aggregator http://host.orb.internal:50051 \
    --mode cpu --duration 24h'
```

## Docker (Full Stack)

From the repository root:

```bash
# Start ClickHouse + aggregator + agent
docker compose up -d

# Only backend (no agent): ClickHouse + aggregator
docker compose up -d clickhouse aggregator

# Run the web UI after backend is up
cd ui && npm install && npm run dev
# Open http://localhost:5173
```

:::tip
If the agent fails with ELF/BPF errors in Docker, run the agent in the OrbStack VM instead and keep `docker compose up -d clickhouse aggregator` on the host.
:::

## Aggregator + ClickHouse

```bash
# ClickHouse in Docker, aggregator locally
docker compose up -d clickhouse

export APERTURE_CLICKHOUSE_ENDPOINT="http://127.0.0.1:8123"
export APERTURE_CLICKHOUSE_DATABASE="aperture"
export APERTURE_CLICKHOUSE_PASSWORD="e2etest"

cargo run -p aperture-aggregator --features clickhouse-storage
# Admin/API: http://127.0.0.1:9090
# gRPC: 127.0.0.1:50051
```

## CLI Commands

With the aggregator running:

```bash
# Query in-memory buffer
cargo run -p aperture-cli -- query \
  --endpoint http://127.0.0.1:50051 --limit 10

# Aggregate from storage (CPU events)
cargo run -p aperture-cli -- aggregate \
  --endpoint http://127.0.0.1:50051 --limit 100 --event_type cpu

# Diff two time windows (CPU)
cargo run -p aperture-cli -- diff \
  --endpoint http://127.0.0.1:50051 --event_type cpu --limit 100
```

## Service Endpoints

| Service | URL | Purpose |
|---------|-----|---------|
| Aggregator Admin/API | `http://127.0.0.1:9090` | Health, metrics, `/api/*` |
| Aggregator gRPC | `127.0.0.1:50051` | Agent push, CLI query |
| ClickHouse HTTP | `127.0.0.1:8123` | When using Docker ClickHouse |
| Web UI | `http://localhost:5173` | After `npm run dev` in `ui/` |
| Prometheus metrics | `http://127.0.0.1:9090/metrics` | Scrape endpoint |
