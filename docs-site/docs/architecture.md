---
sidebar_position: 3
title: Architecture
---

# Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Target Host(s)                        │
│                                                         │
│  ┌──────────────┐    ┌──────────────────────────────┐   │
│  │  eBPF Progs  │───▶│       aperture-agent         │   │
│  │  (kernel)    │    │                              │   │
│  │              │    │  CPU Profiler (perf_event)   │   │
│  │ cpu-profiler │    │  Lock Profiler (tracepoints) │   │
│  │ lock-profiler│    │  Syscall Tracer (raw tp)     │   │
│  │ syscall-tracer│   │  Symbol Resolver (blazesym)  │   │
│  └──────────────┘    │  WASM Filter (optional)      │   │
│                      └──────────────┬───────────────┘   │
└─────────────────────────────────────┼───────────────────┘
                                      │ gRPC Push
                                      ▼
                         ┌─────────────────────────┐
                         │  aperture-aggregator     │
                         │                         │
                         │  gRPC Server (:50051)   │
                         │  HTTP Admin  (:9090)    │
                         │  In-Memory Buffer       │
                         │  Alert Engine           │
                         │  Export Module           │
                         │                         │
                         │  ┌───────────────────┐  │
                         │  │   ClickHouse      │◀─┤── Optional persistence
                         │  │   (storage)       │  │
                         │  └───────────────────┘  │
                         └─────────────┬───────────┘
                                       │ REST API
                                       ▼
                         ┌─────────────────────────┐
                         │     Web UI (React)      │
                         │                         │
                         │  Dashboard              │
                         │  Flamegraph Viewer       │
                         │  Top Functions           │
                         │  Syscall Analysis        │
                         │  Differential Profiling  │
                         │  Alert Management        │
                         └─────────────────────────┘
```

## Workspace Crates

| Crate | Path | Target | Description |
|-------|------|--------|-------------|
| `aperture-agent` | `agent/` | Linux (x86_64/aarch64) | Userspace agent, loads eBPF, collects events, pushes to aggregator |
| `agent-ebpf` | `agent-ebpf/` | `bpfel-unknown-none` | eBPF programs (no_std, runs in kernel) |
| `aperture-shared` | `shared/` | any | Shared types (events, profiles, wire protocol) |
| `aperture-aggregator` | `aggregator/` | any | Central aggregation service (gRPC + HTTP) |
| `aperture-cli` | `cli/` | any | CLI for querying aggregator and profiling |
| `aperture-wasm` | `wasm-runtime/` | any | WASM filter runtime (wasmtime-based) |
| `gpu-profiler` | `gpu-profiler/` | Linux (CUDA) | GPU profiling (CUDA/CUPTI, WIP) |

## Data Flow

### 1. Event Collection (Agent)

```
Kernel eBPF Program
    │
    ▼ PerfEventArray
Per-CPU Reader Tasks (tokio)
    │
    ▼ process_event()
Collector (CpuCollector / LockCollector / SyscallCollector)
    │
    ▼ take_pending_events()
Symbol Resolver (blazesym) ── resolves IPs to function names
    │
    ▼ ProfileEvent (CpuSample | Lock | Syscall)
Wire Protocol (bincode + base64) ── serialize for transport
    │
    ▼ gRPC Push
Aggregator
```

### 2. Event Storage (Aggregator)

```
gRPC PushRequest
    │
    ▼ Authentication check (Bearer token)
InMemoryBuffer (ring buffer, configurable size)
    │
    ├──▶ ClickHouse (async flush, batched writes)
    │
    └──▶ REST API (query, aggregate, diff, export)
```

### 3. Aggregation Pipeline

```
/api/aggregate request
    │
    ▼ Fetch payloads (ClickHouse → fallback to buffer)
Deserialize batches (bincode)
    │
    ▼ aggregate_batches()
Merge CPU profiles (stack dedup + count sum)
Merge Lock profiles (contention sites by address)
Merge Syscall profiles (stats per syscall ID)
    │
    ▼ filter_by_type() (optional)
AggregateResult → JSON response
```

## eBPF Programs

### CPU Profiler

**Source:** `agent-ebpf/src/cpu_profiler.rs`

- **Type:** `perf_event` (software CPU clock)
- **Sampling rate:** configurable (default 99 Hz)
- **PID filtering:** kernel-level via `perf_event_open` scope
- **Output:** `SampleEvent` — timestamp, pid, tid, cpu, user/kernel stack IDs

### Lock Profiler

**Source:** `agent-ebpf/src/lock_profiler.rs`

- **Type:** tracepoints (`sys_enter_futex` / `sys_exit_futex`)
- **Tracks:** futex WAIT operations (wait_time = exit_ts - enter_ts)
- **PID filtering:** `bpf_get_ns_current_pid_tgid()` + PID_FILTER map
- **Output:** `LockEventRaw` — timestamp, pid, tid, lock_addr, wait_ns, stack_id

### Syscall Tracer

**Source:** `agent-ebpf/src/syscall_tracer.rs`

- **Type:** raw tracepoints (`sys_enter` / `sys_exit`)
- **Tracks:** all syscalls (duration = exit_ts - enter_ts)
- **PID filtering:** `bpf_get_ns_current_pid_tgid()` + PID_FILTER map
- **Output:** `SyscallEventRaw` — timestamp, pid, tid, syscall_id, duration_ns, return_value

### BPF Maps

| Map | Type | Key | Value | Used By |
|-----|------|-----|-------|---------|
| EVENTS | PerfEventArray | — | SampleEvent | CPU |
| LOCK_EVENTS | PerfEventArray | — | LockEventRaw | Lock |
| SYSCALL_EVENTS | PerfEventArray | — | SyscallEventRaw | Syscall |
| STACKS | StackTrace | stack_id | frame IPs | CPU |
| LOCK_STACKS | StackTrace | stack_id | frame IPs | Lock |
| PID_FILTER | Array&lt;u64&gt; | 0 | target PID | Lock, Syscall |

## Symbol Resolution

Two code paths resolve instruction pointer (IP) addresses to function names:

### Local Path (`SymbolResolver`)

Used when generating local output (flamegraph SVG, JSON). Splits IPs by address range:
- `>= 0xffff_0000_0000_0000` — kernel (resolves via `/proc/kallsyms`)
- Others — userspace (resolves via `/proc/PID/maps` + debug symbols)

### Aggregator Push Path (`SymbolCache`)

Pre-resolves symbols before sending to aggregator. Same kernel/user split. Encodes module info as `"function_name [module_basename]"`.

### Prerequisites

- `kernel.kptr_restrict=0` for kernel symbol resolution
- `debug = true` in Cargo release profile for Rust binaries
- `libc6-dbg` for glibc debug symbols

See the [Symbol Resolution guide](./guides/symbol-resolution) for troubleshooting.

## WASM Filters

Optional event filtering using WebAssembly modules:

```
ProfileEvent → EventContext (flat C struct) → WASM linear memory
                                                    │
                                              filter(ptr, len) → i32
                                                    │
                                              1 = keep, 0 = discard
```

- **Engine:** wasmtime 16
- **Security:** fuel-limited execution (~1M instructions per call), no threads, bounded memory (1MB)
- **Host functions:** `env.log(ptr, len)` for debug logging

See the [WASM Filters guide](./guides/wasm-filters) for writing custom filters.

## Alert System

The aggregator runs an in-memory alert engine:

- **Rules:** metric + operator + threshold → severity
- **Metrics:** buffer utilization, push error rate/total, ClickHouse flush errors, pending rows, event throughput
- **Evaluation:** manual via `/api/alerts/evaluate` (reads Prometheus counters + buffer state)
- **History:** bounded ring buffer (500 events)
- **Persistence:** in-memory only (rules lost on restart)

See the [Alerting guide](./guides/alerting) for configuration.

## Kubernetes Deployment

```
┌─────────────────────────────────────────────┐
│              Kubernetes Cluster              │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Node 1   │  │ Node 2   │  │ Node 3   │  │
│  │ Agent DS │  │ Agent DS │  │ Agent DS │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └──────────────┼──────────────┘        │
│                      ▼                       │
│            ┌──────────────────┐              │
│            │   Aggregator    │              │
│            │   (Deployment)  │              │
│            └────────┬────────┘              │
│                     ▼                        │
│            ┌─────────────────┐              │
│            │   ClickHouse    │              │
│            │  (StatefulSet)  │              │
│            └─────────────────┘              │
└─────────────────────────────────────────────┘
```

- **Agent** runs as a DaemonSet (one per node, privileged, hostPID)
- **Aggregator** runs as a Deployment (1 replica, ClusterIP service)
- **Prometheus** scrapes aggregator at `:9090/metrics`

See the [Kubernetes guide](./guides/kubernetes) for manifests and setup.
