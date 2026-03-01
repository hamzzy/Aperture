# Run examples

How to run the agent, aggregator, and Web UI in different setups.

---

## Agent (Linux only)

The agent needs Linux (eBPF). Run on a Linux host, in Docker, or in OrbStack Ubuntu VM.

### Agent modes and options

```bash
# CPU profiling (default), push to aggregator, 60 seconds
sudo ./target/release/aperture-agent --mode cpu --duration 60s --aggregator http://HOST:50051

# CPU, 24 hours, 99 Hz (default)
sudo ./target/release/aperture-agent --aggregator http://HOST:50051 --mode cpu --duration 24h

# CPU, profile a single process by PID
sudo ./target/release/aperture-agent --mode cpu --pid 12345 --duration 5m --aggregator http://HOST:50051

# Lock contention (futex) profiling
sudo ./target/release/aperture-agent --mode lock --duration 30s --aggregator http://HOST:50051

# Syscall tracing
sudo ./target/release/aperture-agent --mode syscall --duration 30s --aggregator http://HOST:50051

# Lower overhead: 49 Hz, 10s push interval (set env or use default with APERTURE_LOW_OVERHEAD=1)
APERTURE_LOW_OVERHEAD=1 sudo ./target/release/aperture-agent --aggregator http://HOST:50051 --mode cpu --duration 1h

# Local run, write flamegraph to file (no aggregator)
sudo ./target/release/aperture-agent --mode cpu --duration 30s --output flamegraph.svg
```

Replace `HOST` with your aggregator host (e.g. `127.0.0.1`, `host.orb.internal` from OrbStack VM, or `aggregator` in Docker).

---

## OrbStack Ubuntu VM (from Mac)

Repo is available in the VM via path translation. Build once, then run the agent.

```bash
# 1) Build eBPF (nightly) and agent (stable) in the VM
orb run -m ubuntu -w /Users/user/aperture bash -c '\
  cargo +nightly build -p aperture-ebpf --target bpfel-unknown-none -Z build-std=core --release && \
  cargo build -p aperture-agent --release'

# 2) Run agent (aggregator on Mac at host.orb.internal:50051)
orb run -m ubuntu -w /Users/user/aperture -u root bash -c '\
  sudo ./target/release/aperture-agent --aggregator http://host.orb.internal:50051 --mode cpu --duration 60s'

# 24h CPU profile
orb run -m ubuntu -w /Users/user/aperture -u root bash -c '\
  sudo ./target/release/aperture-agent --aggregator http://host.orb.internal:50051 --mode cpu --duration 24h'
```

---

## Run a Python program in OrbStack and capture data to Aperture

Yes, you can run a Python program in the OrbStack VM. Aperture does not ingest application logs (stdout/stderr); it **profiles** processes (CPU stacks, lock contention, syscalls) and sends that data to the aggregator. You can still keep your program’s logs separately.

### 1. Start the backend on your Mac

```bash
docker compose up -d clickhouse aggregator
cd ui && npm run dev
# Open http://localhost:8080
```

### 2. Run your Python program in the OrbStack VM

```bash
# Start Python in the VM (use your project path and script)
orb run -m ubuntu -w /path/to/your/app bash -c 'python3 -u your_script.py'
```

To **capture stdout/stderr to a file** (for your own logs), redirect in the VM:

```bash
orb run -m ubuntu -w /path/to/your/app bash -c 'python3 -u your_script.py 2>&1 | tee app.log'
# or
orb run -m ubuntu -w /path/to/your/app bash -c 'python3 -u your_script.py > app.log 2>&1'
```

### 3. Profile the Python process with Aperture (CPU / lock / syscall data)

Run the agent **in the same VM** as the Python process, and pass the Python process **PID** so only that process is profiled:

```bash
# In one terminal: start Python and note its PID (e.g. 12345)
orb run -m ubuntu -w /path/to/your/app bash -c 'python3 -u your_script.py & echo $!'

# In another: run the agent for 5 minutes, targeting that PID, pushing to aggregator on Mac
orb run -m ubuntu -w /Users/user/aperture -u root bash -c '\
  sudo ./target/release/aperture-agent --mode cpu --pid 12345 --duration 5m \
  --aggregator http://host.orb.internal:50051'
```

Or run both in one VM session: start the agent in the background (with a known PID or without `--pid` to profile everything), then start your Python app:

```bash
orb run -m ubuntu -w /Users/user/aperture bash -c '
  # Start agent in background (profiles all processes; omit --pid)
  sudo ./target/release/aperture-agent --aggregator http://host.orb.internal:50051 \
    --mode cpu --duration 10m &
  sleep 2
  # Run your Python program
  python3 -u /path/to/your/script.py 2>&1 | tee app.log
  wait'
```

### 4. View the “log” in Aperture

- **Profile data**: Open the Web UI (http://localhost:8080). Use the dashboard, flamegraph, top functions, and syscalls views for the time range when the Python process was running. That’s the “log” of CPU usage, lock contention, and syscalls that Aperture captures.
- **Application logs**: Your Python stdout/stderr are in the file you used (`app.log` or similar). Aperture does not ingest or store those; keep that file or pipe it to your own logging system.

### Summary

| What you want              | How |
|----------------------------|-----|
| Run Python in OrbStack     | `orb run -m ubuntu -w /path bash -c 'python3 your_script.py'` |
| Profile it with Aperture   | Run the agent in the same VM with `--pid <python_pid>` (or no `--pid` to profile all), aggregator at `http://host.orb.internal:50051`. |
| Capture Python stdout/stderr | Redirect in the VM: `... 2>&1 \| tee app.log` or `... > app.log 2>&1`. |

---

## Docker (full stack)

From repo root:

```bash
# Start ClickHouse + aggregator + agent (agent in Docker)
docker compose up -d

# Only backend (no agent): ClickHouse + aggregator
docker compose up -d clickhouse aggregator

# Run Web UI after backend is up
cd ui && npm install && npm run dev
# Open http://localhost:8080
```

Agent in Docker uses: `--aggregator http://aggregator:50051 --mode cpu --duration 24h`. If the agent fails with ELF/BPF errors, run the agent in the OrbStack VM instead (see above) and keep `docker compose up -d clickhouse aggregator` on the host.

---

## Aggregator + ClickHouse

```bash
# ClickHouse in Docker, aggregator locally (Mac/Linux)
docker compose up -d clickhouse
export APERTURE_CLICKHOUSE_ENDPOINT="http://127.0.0.1:8123"
export APERTURE_CLICKHOUSE_DATABASE="aperture"
export APERTURE_CLICKHOUSE_PASSWORD="e2etest"
cargo run -p aperture-aggregator --features clickhouse-storage
# Admin/API: http://127.0.0.1:9090, gRPC: 127.0.0.1:50051
```

---

## CLI (query / aggregate / diff)

With aggregator (and optionally ClickHouse) running:

```bash
# In-memory buffer
cargo run -p aperture-cli -- query --endpoint http://127.0.0.1:50051 --limit 10

# Aggregate from storage (CPU events)
cargo run -p aperture-cli -- aggregate --endpoint http://127.0.0.1:50051 --limit 100 --event_type cpu

# Diff two time windows (CPU)
cargo run -p aperture-cli -- diff --endpoint http://127.0.0.1:50051 --event_type cpu --limit 100
```

---

## One-shot demo scripts

- **`docker compose up -d`** – Start ClickHouse + aggregator + agent (Docker). Then run the Web UI from the `ui` directory.
- **`./scripts/demo-live-orb.sh`** – Full demo via OrbStack: ClickHouse on Mac, sync to VM, build, run aggregator + agent + CLI in VM (requires `ssh ubuntu@orb`).
- **`./scripts/demo-live.sh`** – Same idea on native Linux (Docker for ClickHouse, local aggregator + agent + CLI).

---

## Endpoints

| Service    | URL                    | Purpose                    |
|-----------|-------------------------|----------------------------|
| Aggregator admin/API | http://127.0.0.1:9090 | Health, metrics, `/api/*`  |
| Aggregator gRPC      | 127.0.0.1:50051      | Agent push, CLI query      |
| ClickHouse HTTP      | 127.0.0.1:8123       | When using Docker clickhouse |
| Web UI               | http://localhost:8080 | After `npm run dev` in `ui`            |
