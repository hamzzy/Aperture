# Aggregator Service

The aggregator service is the central coordinator for distributed profiling deployments. It receives profile data from agents via gRPC and buffers it in memory (with optional persistent backends).

## Features (Planned)

Agents can push batches using the generated gRPC client (`PushRequest`: `agent_id`, `sequence`, `payload` with bincode `Message`). Agent-side push integration is a follow-up; the aggregator is ready to receive.

## Architecture

```
┌──────────┐       ┌──────────┐       ┌──────────┐
│  Agent   │──────▶│          │       │          │
│  Node 1  │       │          │       │ Storage  │
└──────────┘       │          │──────▶│ Backend  │
                   │Aggregator│       │(ClickHouse)
┌──────────┐       │          │       │          │
│  Agent   │──────▶│          │       └──────────┘
│  Node 2  │       │          │
└──────────┘       └──────────┘
```

## Live demo (OrbStack)

From macOS, run the full pipeline (agent → aggregator → ClickHouse) inside OrbStack:

```bash
./scripts/demo-live-orb.sh
```

Syncs to `ubuntu@orb`, builds eBPF + aggregator (with ClickHouse) + agent + CLI, then runs: ClickHouse → aggregator → agent (CPU 5s, `--aggregator`) → `aperture query`. Requires OrbStack and `ssh ubuntu@orb`; the agent step uses sudo (eBPF). On Linux natively, use `./scripts/demo-live.sh` instead.

## ClickHouse storage (optional)

Build with the feature and set env to persist batches:

```bash
cargo build -p aperture-aggregator --features clickhouse-storage

export APERTURE_CLICKHOUSE_ENDPOINT=http://localhost:8123
export APERTURE_CLICKHOUSE_DATABASE=aperture
./target/debug/aperture-aggregator
```

- **Push**: each batch is written to the in-memory buffer and to the `aperture_batches` table.
- **Query**: in-memory buffer (unchanged).
- **QueryStorage**: time-range query against ClickHouse (`time_start_ns`, `time_end_ns`, `agent_id`, `limit`). Use a gRPC client (e.g. grpcurl) or add a CLI command.

### Wiping ClickHouse for a fresh start

If you have old or incompatible data (e.g. after a schema/crate change), drop the table or database and let the aggregator recreate it on next push:

```bash
# With ClickHouse in Docker (container name aperture-clickhouse, database aperture)
docker exec -it aperture-clickhouse clickhouse-client --password e2etest --query "DROP TABLE IF EXISTS aperture.aperture_batches"

# Or drop the whole database
docker exec -it aperture-clickhouse clickhouse-client --password e2etest --query "DROP DATABASE IF EXISTS aperture"
```

Restart the aggregator (and optionally the agent) so new pushes use the current schema.

## Planned

- ScyllaDB backend, TLS, Docker / Kubernetes manifests
