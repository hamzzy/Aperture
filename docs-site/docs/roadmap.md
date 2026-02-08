---
sidebar_position: 8
title: Roadmap
---

# Roadmap

Development status and future plans for Aperture.

## Completed

### CPU Profiling
- [x] eBPF perf_event software CPU clock sampling
- [x] Configurable sampling frequency (default 99 Hz)
- [x] PID filtering via `perf_event_open` scope
- [x] Stack trace collection (user + kernel)
- [x] Symbol resolution with blazesym
- [x] Flamegraph SVG output
- [x] JSON output

### Lock & Syscall Profiling
- [x] Lock contention: futex tracing via `sys_enter_futex` / `sys_exit_futex`
- [x] Syscall tracing: all syscalls via `sys_enter` / `sys_exit` raw tracepoints
- [x] Namespace-aware PID filtering (`bpf_get_ns_current_pid_tgid`)
- [x] Lock flamegraph (weighted by wait_ns)
- [x] Syscall latency histograms
- [x] `--mode cpu|lock|syscall|all` with concurrent execution

### Distributed Aggregator
- [x] gRPC service (Push, Query, Aggregate, Diff RPCs)
- [x] In-memory ring buffer
- [x] ClickHouse persistent storage
- [x] HTTP REST API
- [x] CLI client (query, aggregate, diff commands)
- [x] Authentication (Bearer token)

### WASM Filters
- [x] wasmtime-based filter runtime
- [x] EventContext ABI (flat C struct)
- [x] Fuel-limited execution (~1M instructions per call)
- [x] Host function: `env.log`
- [x] `filter_event()` and `filter_batch()` APIs

### Web Dashboard
- [x] React + Vite + Tailwind + shadcn/ui
- [x] Interactive flamegraph viewer
- [x] Top functions table
- [x] Syscall analysis page
- [x] Differential profiling (comparison view)
- [x] Timeline view
- [x] Settings page

### Alerts & Monitoring
- [x] In-memory alert engine (rules, evaluation, history)
- [x] 6 alert metrics (buffer, errors, throughput)
- [x] REST API for rule CRUD + evaluation
- [x] Alert management UI page
- [x] Prometheus metrics endpoint

### Export & Deployment
- [x] JSON export endpoint
- [x] Collapsed-stack format export
- [x] Dockerfiles (agent + aggregator)
- [x] CI/CD workflows (GitHub Actions)
- [x] Install script (`curl | sh`)
- [x] Kubernetes manifests (DaemonSet + Deployment)

### UI Polish
- [x] Unresolved frame detection and grey styling
- [x] Enhanced tooltips (module, depth, percentages)
- [x] Symbol resolution diagnostics in agent
- [x] Module info encoding in symbol strings
- [x] Real batch data in timeline chart

## In Progress

### GPU Profiling
- [ ] CUPTI integration for CUDA kernel tracking
- [ ] Memory transfer profiling
- [ ] GPU utilization metrics
- [ ] GPU-CPU correlation timeline
- [ ] ROCm support

## Planned

### Continuous Profiling
- [ ] Always-on mode with adaptive sampling
- [ ] Storage retention policies
- [ ] Historical trend analysis

### Advanced Aggregation
- [ ] Multi-aggregator clustering
- [ ] Cross-agent correlation
- [ ] Automatic anomaly detection

### Ecosystem Integration
- [ ] OpenTelemetry trace correlation
- [ ] Grafana datasource plugin
- [ ] pprof format import/export
- [ ] Jaeger span linking
