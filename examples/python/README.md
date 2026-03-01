# Python workload + Aperture in OrbStack

Run a Python process in the OrbStack VM and profile it with the Aperture agent.

**Important:** The agent and the Python process must run in the **same** OrbStack VM session. Each `orb run` is a separate environment with its own PID namespace, so a PID from one terminal is not visible in another. Use the single-session flow below.

## Prerequisites

- OrbStack with an Ubuntu VM
- On your Mac: Aperture repo, aggregator + ClickHouse + UI running
- In the VM: aperture-agent built (eBPF + agent)

## 1. On your Mac: start backend and UI

```bash
cd /Users/user/aperture
docker compose up -d clickhouse aggregator
cd ui && npm run dev
# Open http://localhost:8080
```

## 2. Build agent in the VM (once)

```bash
orb run -m ubuntu -w /Users/user/aperture bash -c '\
  cargo +nightly build -p aperture-ebpf --target bpfel-unknown-none -Z build-std=core --release && \
  cargo build -p aperture-agent --release'
```

## 3. Run agent + Python in one VM session (recommended)

One `orb run` so both share the same PID namespace. Agent in background, then Python in foreground:

```bash
orb run -m ubuntu -w /Users/user/aperture bash -c '\
  sudo ./target/release/aperture-agent --aggregator http://host.orb.internal:50051 \
    --mode cpu --duration 6m &
  sleep 3
  python3 examples/python/workload.py 2>&1
  wait'
```

The agent profiles all processes (no `--pid`); the Python workload will show up. In the Web UI, use the last ~5 minutes; filter or look for the python3 process.

## 4. View the profile

In the Web UI (http://localhost:8080), open the dashboard, flamegraph, or top functions for the time range when the workload ran. You should see Python and the `busy` / `main` stack.

---

## Why “two terminals” doesn’t work

If you run Python in one `orb run` and the agent in another, they are in **different** environments (different PID namespaces). The PID you see in the Python terminal (e.g. 12345) does not exist in the agent’s environment, so you get:

```text
Cannot read /proc/12345/maps — userspace symbols for PID 12345 unavailable: No such file or directory
```

Always run both the workload and the agent in the **same** `orb run` invocation so they share one PID namespace.
