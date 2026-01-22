# Aperture Roadmap

This document outlines the development phases and milestones for the Aperture project.

## Development Phases

### Phase 0: Project Setup ✅

- [x] Repository structure and Cargo workspace
- [x] Core crate scaffolding (agent, agent-ebpf, shared, cli)
- [x] Basic configuration files (Cargo.toml, rustfmt, clippy)
- [x] Documentation structure

### Phase 1: Basic CPU Profiling 🚧

**Goal**: Implement fundamental CPU profiling using eBPF stack sampling

**Milestones**:
- [ ] eBPF program for CPU sampling
  - [ ] Implement perf_event-based sampling
  - [ ] Stack trace collection (kernel + userspace)
  - [ ] BPF map for storing stack traces
- [ ] Userspace agent
  - [ ] eBPF program loader (using Aya)
  - [ ] Stack trace retrieval from BPF maps
  - [ ] Symbol resolution (using DWARF/symbols)
  - [ ] Process and thread context collection
- [ ] Output generation
  - [ ] Flamegraph generation (using inferno)
  - [ ] JSON output format
  - [ ] CLI for basic operations
- [ ] Testing & validation
  - [ ] Unit tests for components
  - [ ] Integration tests
  - [ ] Benchmarking overhead (<1% target)

**Deliverables**:
- Working CPU profiler that can sample any process
- Flamegraph visualization
- Basic CLI tool

### Phase 2: Lock Contention & Syscall Tracing

**Goal**: Extend profiling to locks and system calls

**Milestones**:
- [ ] Lock contention profiling
  - [ ] Futex tracing eBPF program
  - [ ] Mutex hold time tracking
  - [ ] Lock acquisition latency histograms
- [ ] Syscall tracing
  - [ ] Syscall entry/exit probes
  - [ ] Per-syscall latency tracking
  - [ ] Error rate monitoring
- [ ] Enhanced output
  - [ ] Lock contention flamegraphs
  - [ ] Syscall latency histograms
  - [ ] JSON format extensions

**Deliverables**:
- Lock contention analysis
- Syscall performance profiling

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

### Phase 5: Distributed Aggregator

**Goal**: Build central aggregation service for multi-agent deployments

**Milestones**:
- [ ] Agent-to-aggregator protocol
  - [ ] Cap'n Proto schema definition
  - [ ] gRPC service implementation
  - [ ] TLS support
- [ ] Aggregator service
  - [ ] Multi-agent data ingestion
  - [ ] In-memory buffering
  - [ ] Query API
- [ ] Deployment
  - [ ] Docker images
  - [ ] Kubernetes manifests
  - [ ] Configuration management

**Deliverables**:
- Scalable aggregator service
- Multi-agent coordination
- Container deployment support

### Phase 6: Storage Integration

**Goal**: Persistent storage for long-term profiling data

**Milestones**:
- [ ] ClickHouse backend
  - [ ] Schema design for profiling data
  - [ ] Efficient batch insertion
  - [ ] Query optimization
- [ ] ScyllaDB backend (optional)
  - [ ] Wide-column schema
  - [ ] Time-series optimization
- [ ] Query layer
  - [ ] Time-range queries
  - [ ] Aggregation functions
  - [ ] Differential profiling

**Deliverables**:
- Persistent storage backend
- Query API
- Historical analysis capabilities

### Phase 7: Production Hardening

**Goal**: Make the system production-ready

**Milestones**:
- [ ] Security hardening
  - [ ] Capability-based permissions
  - [ ] Secure agent-aggregator communication
  - [ ] Audit logging
- [ ] Reliability
  - [ ] Error handling and recovery
  - [ ] Resource limits and backpressure
  - [ ] Graceful degradation
- [ ] Observability
  - [ ] Prometheus metrics
  - [ ] Structured logging
  - [ ] Health checks
- [ ] Performance optimization
  - [ ] Memory efficiency
  - [ ] CPU overhead tuning
  - [ ] Network optimization

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
