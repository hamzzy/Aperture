# eBPF Profiler Agent

**Platform: Linux only.** The agent loads kernel eBPF programs (PerfEventArray, etc.) and uses `perf_event_open`; these are not available on macOS or Windows. Run the agent on a Linux host, VM (e.g. OrbStack), or container. The aggregator and Web UI can run on any OS.

The profiling agent is responsible for:

1. Loading and managing eBPF programs
2. Collecting profiling samples from the kernel
3. Resolving symbols (instruction pointers → function names)
4. Generating output (flamegraphs, JSON)

## Architecture

```
┌─────────────────┐
│   main.rs       │  Entry point, CLI argument parsing
└────────┬────────┘
         │
    ┌────▼─────┐
    │  ebpf/   │  eBPF program loading and management
    └────┬─────┘
         │
    ┌────▼──────┐
    │ collector/│  Event collection and aggregation
    └────┬──────┘
         │
    ┌────▼──────┐
    │  output/  │  Flamegraph and JSON generation
    └───────────┘
```

## Implementation checklist

- [ ] eBPF program loading (`ebpf/loader.rs`)
- [ ] Perf event attachment
- [ ] Event collection from eBPF maps
- [ ] Symbol resolution using blazesym
- [ ] Flamegraph generation using inferno
- [ ] JSON output
- [ ] Integration tests

## Usage

```bash
# Profile a specific process for 30 seconds
sudo ./profiler-agent --pid 1234 --duration 30s

# Profile all processes for 1 minute at 99 Hz
sudo ./profiler-agent --duration 1m --sample-rate 99

# Output both flamegraph and JSON
sudo ./profiler-agent --pid 1234 --output profile.svg --json profile.json
```

## Development

See [docs/development.md](../docs/development.md) for development setup and guidelines.
