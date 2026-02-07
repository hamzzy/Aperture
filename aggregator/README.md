# Aggregator Service (Phase 5)

The aggregator service is the central coordinator for distributed profiling deployments. It receives profile data from agents via gRPC and buffers it in memory (with optional persistent backends in Phase 6).

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

## Phase 6: ClickHouse storage (optional)

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

## Planned

- ScyllaDB backend, TLS, Docker / Kubernetes manifests
