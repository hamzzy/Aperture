# Aperture Roadmap

This document outlines the development phases and milestones for the Aperture project.

## Development Phases

### Phase 0: Project Setup ✅

- [x] Repository structure and Cargo workspace
- [x] Core crate scaffolding (agent, agent-ebpf, shared, cli)
- [x] Basic configuration files (Cargo.toml, rustfmt, clippy)
- [x] Documentation structure

### Phase 1: Basic CPU Profiling ✅

**Goal**: Implement fundamental CPU profiling using eBPF stack sampling

**Milestones**:
- [x] eBPF program for CPU sampling
  - [x] Implement perf_event-based sampling (software CPU clock, configurable Hz)
  - [x] Stack trace collection (kernel + userspace)
  - [x] BPF map for storing stack traces (EVENTS PerfEventArray + STACKS StackTraceMap)
- [x] Userspace agent
  - [x] eBPF program loader (using Aya)
  - [x] Stack trace retrieval from BPF maps (per-CPU async readers)
  - [x] Symbol resolution (using blazesym)
  - [x] Process and thread context collection
  - [x] PID filtering via perf_event_open scope (handles PID namespaces)
  - [x] Timestamp conversion (monotonic → wall clock)
- [x] Output generation
  - [x] Flamegraph generation (using inferno)
  - [x] JSON output format
  - [x] CLI for basic operations (wired to agent library)
- [x] Testing & validation
  - [x] Unit tests for components (30 tests across agent + shared)
  - [x] Integration test (full pipeline: collect → build → flamegraph + JSON)
  - [ ] Benchmarking overhead (<1% target)

**Deliverables**:
- Working CPU profiler that can sample any process
- Flamegraph visualization
- Basic CLI tool

### Phase 2: Lock Contention & Syscall Tracing ✅

**Goal**: Extend profiling to locks and system calls

**Milestones**:
- [x] Lock contention profiling
  - [x] Futex tracing eBPF program (sys_enter_futex / sys_exit_futex tracepoints)
  - [x] Lock wait time tracking (entry/exit correlation via BPF HashMap)
  - [x] Lock contention aggregation by (lock_addr, stack) with min/max/total stats
- [x] Syscall tracing
  - [x] Syscall entry/exit raw tracepoints (sys_enter / sys_exit)
  - [x] Per-syscall latency tracking with power-of-2 histogram (30 buckets)
  - [x] Error rate monitoring (negative return values)
  - [x] x86_64 syscall name resolution
- [x] Enhanced output
  - [x] Lock contention flamegraphs (stacks weighted by wait time in ns)
  - [x] Syscall latency histograms (text table with count, avg, p50, p99, max, errors)
  - [x] JSON format extensions (lock profile + syscall profile)
- [x] Multi-mode support
  - [x] `--mode cpu|lock|syscall|all` CLI flag
  - [x] Concurrent profiling via tokio::join! for `--mode all`
- [x] Testing & validation
  - [x] Unit tests for lock and syscall collectors
  - [x] Integration tests (lock and syscall pipeline)
  - [x] E2E verification in VM (all modes)

**Deliverables**:
- Lock contention analysis with flamegraph visualization
- Syscall performance profiling with latency histograms

### Phase 3: WASM Filter Engine

**Goal**: Enable programmable filtering of profiling events

**Milestones**:
- [ ] WASM runtime integration (Wasmtime)
- [ ] Filter API design
  - [ ] Event input format
  - [ ] Filter result format
  - [ ] Performance constraints
- [ ] SDK for writing filters
  - [ ] Rust SDK with examples
  - [ ] API documentation
- [ ] Filter examples
  - [ ] Process name filtering
  - [ ] Stack frame filtering
  - [ ] Time-based sampling

**Deliverables**:
- WASM filter runtime
- SDK and examples
- Documentation for filter development

### Phase 4: GPU Profiling

**Goal**: Add GPU profiling for CUDA workloads

**Milestones**:
- [ ] CUPTI integration
  - [ ] CUDA kernel launch tracking
  - [ ] Memory transfer profiling
  - [ ] GPU utilization metrics
- [ ] ROCm support (optional)
- [ ] GPU-CPU correlation
  - [ ] Timeline visualization
  - [ ] Unified flamegraphs
- [ ] GPU-specific metrics
  - [ ] Kernel execution time
  - [ ] Memory bandwidth
  - [ ] Occupancy

**Deliverables**:
- CUDA profiling support
- GPU-aware flamegraphs
- Combined CPU+GPU analysis

### Phase 5: Distributed Aggregator ✅ (core done)

**Goal**: Build central aggregation service for multi-agent deployments

**Milestones**:
- [x] Agent-to-aggregator protocol
  - [x] Wire format (bincode `Message`: version, sequence, events)
  - [x] gRPC service implementation (Push + Query)
  - [ ] TLS support (future)
- [x] Aggregator service
  - [x] Multi-agent data ingestion (Push RPC)
  - [x] In-memory buffering (ring buffer, 10k batches)
  - [x] Query API (Query RPC + `aperture query` CLI)
- [ ] Deployment
  - [ ] Docker images
  - [ ] Kubernetes manifests
  - [x] Configuration (env `APERTURE_AGGREGATOR_LISTEN`, default 0.0.0.0:50051)

**Deliverables**:
- Scalable aggregator service (gRPC server, in-memory buffer)
- CLI query command
- Agent push client integration (follow-up)
- Container deployment support (future)

### Phase 6: Storage Integration ✅

**Goal**: Persistent storage for long-term profiling data

**Milestones**:
- [x] ClickHouse backend
  - [x] Schema: `aperture_batches` (agent_id, sequence, received_at_ms, event_count, payload base64)
  - [x] Batch insertion on Push (optional, behind `clickhouse-storage` feature)
  - [x] Query optimization (indexes, partitioning, TTL)
- [ ] ScyllaDB backend (optional)
  - [ ] Wide-column schema
  - [ ] Time-series optimization
- [x] Query layer
  - [x] Time-range queries (QueryStorage RPC: time_start_ns, time_end_ns, agent_id, limit)
  - [x] Aggregation functions (Aggregate RPC: server-side merge of events into profiles)
  - [x] Differential profiling (Diff RPC: compare two time windows or agents)

**Deliverables**:
- ClickHouse persistent backend (feature-gated)
- QueryStorage gRPC + env config (`APERTURE_CLICKHOUSE_ENDPOINT`, `APERTURE_CLICKHOUSE_DATABASE`)
- Aggregate + Diff RPCs with CLI (`aperture aggregate`, `aperture diff`)
- ScyllaDB and aggregation (future)

### Phase 7: Production Hardening

**Goal**: Make the system production-ready

**Milestones**:
- [ ] Security hardening
  - [ ] Capability-based permissions
  - [x] Secure agent-aggregator communication (Bearer token auth, gRPC interceptor)
  - [x] Audit logging (structured events for auth and admin HTTP; target aperture::audit)
- [ ] Reliability
  - [x] Error handling and recovery (agent push retry + connection reuse)
  - [x] Resource limits and backpressure (configurable buffer, message size, backpressure signal)
  - [x] Graceful degradation (graceful ClickHouse shutdown flush)
- [ ] Observability
  - [x] Prometheus metrics (aggregator metrics module + instrumentation)
  - [x] Structured logging (APERTURE_LOG_FORMAT=json)
  - [x] Health checks (admin HTTP: /healthz, /readyz, /metrics; tonic-health gRPC)
- [ ] Performance optimization
  - [x] Memory efficiency (buffer pre-allocation VecDeque::with_capacity; single payload clone in push path)
  - [x] CPU overhead tuning (APERTURE_LOW_OVERHEAD=1: 49 Hz, 10s push interval; configurable push_interval)
  - [x] Network optimization (gzip compression for gRPC when auth disabled; agent send/accept gzip)

**Deliverables**:
- Production-grade system
- Comprehensive monitoring
- Security audit results

### Phase 8: Web UI

**Goal**: Build web-based UI for visualization and analysis

**Milestones**:
- [ ] Frontend development
  - [ ] Flamegraph viewer
  - [ ] Timeline visualization
  - [ ] Comparison view
- [ ] Backend API
  - [ ] REST API for queries
  - [ ] WebSocket for live updates
- [ ] Features
  - [ ] Saved queries
  - [ ] Dashboards
  - [ ] Alerting (basic)

**Deliverables**:
- Web-based UI
- Interactive visualizations
- Dashboard capabilities

## Success Metrics

- **Phase 1**: <1% CPU overhead, accurate flamegraphs
- **Phase 2**: Lock contention detection, syscall latency tracking
- **Phase 3**: Filter execution <100μs per event
- **Phase 4**: GPU kernel profiling with <2% overhead
- **Phase 5**: Support 1000+ agents per aggregator
- **Phase 6**: Store 1TB+ profiling data with <100ms query latency
- **Phase 7**: Pass security audit, 99.9% uptime
- **Phase 8**: Interactive UI with <1s response time

## Contributing

Want to contribute? Check which phase aligns with your interests:
- **eBPF/Kernel**: Phase 1, 2
- **WebAssembly**: Phase 3
- **GPU/CUDA**: Phase 4
- **Distributed Systems**: Phase 5, 6
- **Security/Reliability**: Phase 7
- **Frontend/UI**: Phase 8

See the main README for contribution guidelines.
