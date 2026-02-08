---
sidebar_position: 1
slug: /
title: Introduction
---

# Aperture

**Production-grade distributed profiler for CPU & GPU workloads built with Rust and eBPF.**

Aperture is a high-performance profiling system that uses eBPF for low-overhead performance monitoring. It supports CPU sampling, lock contention tracing, and syscall analysis in a distributed agent-aggregator architecture.

## Features

- **CPU Profiling** — Continuous stack trace sampling with less than 1% overhead using eBPF perf events
- **Lock Contention** — Detect mutex/futex bottlenecks via tracepoint-based tracking
- **Syscall Tracing** — Latency histograms and error analysis for all system calls
- **Symbol Resolution** — Automatic kernel + userspace symbol resolution with blazesym
- **Distributed Architecture** — Agent-aggregator model with gRPC transport and ClickHouse storage
- **Web Dashboard** — Interactive flamegraphs, top functions, syscall analysis, differential profiling
- **Alert Engine** — Configurable threshold alerts on buffer, error, and throughput metrics
- **Data Export** — JSON and collapsed-stack format export for integration with external tools
- **Prometheus Metrics** — Built-in `/metrics` endpoint for aggregator observability
- **WASM Filters** — Programmable event filtering with WebAssembly (wasmtime)

## How It Works

```
Target Host                              Aggregator
┌──────────────────────┐      gRPC      ┌───────────────────┐
│  eBPF Programs       │───────────────▶│  gRPC Server      │
│  (kernel space)      │                │  HTTP Admin API   │
│                      │                │  In-Memory Buffer │
│  aperture-agent      │                │  Alert Engine     │
│  (userspace loader)  │                │  ClickHouse       │
└──────────────────────┘                └─────────┬─────────┘
                                                  │ REST API
                                                  ▼
                                        ┌───────────────────┐
                                        │   Web Dashboard   │
                                        │   (React + Vite)  │
                                        └───────────────────┘
```

1. **eBPF programs** run in the kernel, collecting stack traces with minimal overhead
2. **The agent** reads events from perf buffers, resolves symbols, and pushes data to the aggregator
3. **The aggregator** stores, aggregates, and serves data via REST and gRPC APIs
4. **The web dashboard** displays interactive flamegraphs, top functions, and syscall analysis

## Repository Structure

```
aperture/
├── agent/              # Userspace profiling agent
├── agent-ebpf/         # eBPF programs (no_std, bpfel-unknown-none)
├── shared/             # Shared types and wire protocol
├── aggregator/         # Aggregation service (gRPC + HTTP)
├── cli/                # CLI for querying aggregator
├── wasm-runtime/       # WASM filter runtime (wasmtime)
├── gpu-profiler/       # GPU profiling (CUDA/CUPTI, WIP)
├── ui/                 # React web dashboard
├── deploy/k8s/         # Kubernetes manifests
├── scripts/            # Setup and demo scripts
└── docs-site/          # This documentation (Docusaurus)
```

## Next Steps

- [Getting Started](./getting-started) — Install and run your first profile
- [Architecture](./architecture) — Deep dive into system design
- [API Reference](./api-reference) — REST, gRPC, and Prometheus endpoints
