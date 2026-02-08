# Aperture Web UI

Web UI for Aperture profiling (flamegraph, top functions, comparison). Connects to the aggregator REST API; storage uses ClickHouse.

## Run with Docker (recommended)

From repo root:

```bash
docker compose up -d
```

This starts **ClickHouse**, **aggregator** (with storage), and **agent** (Linux eBPF in Docker, pushes to aggregator → ClickHouse). Then:

```bash
cd ui
npm install
npm run dev
```

Open **http://localhost:8080**. The agent feeds data so the flamegraph and dashboard show real profiles after a short delay.

## Run without agent

To run only ClickHouse + aggregator (no agent):

```bash
docker compose up -d clickhouse aggregator
cd ui && npm run dev
```

Then `/api/aggregate` and flamegraph will work only if data was previously pushed (e.g. from another run with the agent).

## Backend only (ClickHouse + aggregator)

```bash
docker compose up -d clickhouse aggregator
# API: http://127.0.0.1:9090
```

## Local aggregator + ClickHouse in Docker

```bash
docker compose up -d clickhouse
export APERTURE_CLICKHOUSE_ENDPOINT="http://127.0.0.1:8123"
export APERTURE_CLICKHOUSE_DATABASE="aperture"
export APERTURE_CLICKHOUSE_PASSWORD="e2etest"
cargo run -p aperture-aggregator --features clickhouse-storage
```

Then run the UI from the `ui` directory as above; agent must run on Linux (e.g. in Docker or a VM).
