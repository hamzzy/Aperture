# Aggregator Service (Phase 5+)

The aggregator service is the central coordinator for distributed profiling deployments.

## Features (Planned)

- gRPC server for receiving profiling data from agents
- Storage backends: ClickHouse, ScyllaDB
- Query API for retrieving aggregated profiles
- Profile merging and aggregation
- Retention policies

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

## Status

🚧 Not yet implemented - Phase 5+
