# GPU Profiler (Phase 4+)

GPU profiling support for CUDA and ROCm workloads.

## Features (Planned)

- CUDA kernel profiling via CUPTI
- Memory transfer tracking
- GPU utilization metrics
- Correlation with CPU profiling data

## Dependencies

### CUDA Support
- NVIDIA CUDA Toolkit 11.0+
- CUPTI (included with CUDA Toolkit)

### ROCm Support (Future)
- AMD ROCm 5.0+
- rocProfiler

## Example Usage

```rust
use gpu_profiler::cupti::CuptiProfiler;
use gpu_profiler::GpuProfiler;

let mut profiler = CuptiProfiler::new()?;
profiler.start()?;

// Run GPU workload

profiler.stop()?;
let metrics = profiler.collect_metrics()?;
```

## Status

🚧 Not yet implemented - Phase 4+
