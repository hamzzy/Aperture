# Aperture Roadmap

This document outlines the milestones and future plans for the Aperture project.

## Milestones


  - [ ] Benchmarking overhead (<1% target)

**Deliverables**:
- Lock contention analysis with flamegraph visualization
- Syscall performance profiling with latency histograms
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


**Goal**: Add GPU profiling for CUDA workloads

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

- [ ] Deployment
  - [ ] Docker images
  - [ ] Kubernetes manifests

