---
sidebar_position: 7
title: API Reference
---

# API Reference

The aggregator exposes two servers:

- **gRPC** (default `:50051`) — agent data ingestion and programmatic queries
- **HTTP** (default `:9090`) — health checks, metrics, REST API for the web UI

## HTTP Endpoints

### Health & Operations

| Method | Path | Description |
|--------|------|-------------|
| GET | `/healthz` | Liveness probe (returns `ok`) |
| GET | `/readyz` | Readiness probe (checks buffer) |
| GET | `/metrics` | Prometheus metrics (OpenMetrics text) |

### Profiling Data

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/aggregate` | Aggregate profiling events |
| POST | `/api/diff` | Differential profiling (requires storage) |
| GET | `/api/batches` | List recent ingested batches |
| GET | `/api/health` | UI health info (buffer, push stats, ClickHouse) |

### Alerts

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/alerts` | List all alert rules |
| POST | `/api/alerts` | Create a new alert rule |
| DELETE | `/api/alerts/:id` | Delete an alert rule |
| POST | `/api/alerts/:id/toggle` | Enable/disable an alert rule |
| GET | `/api/alerts/history` | List fired alert events |
| POST | `/api/alerts/evaluate` | Evaluate rules against current metrics |

### Export

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/export/json` | Download aggregated profile as JSON |
| GET | `/api/export/collapsed` | Download CPU stacks in collapsed format |

---

## Endpoint Details

### POST /api/aggregate

Aggregate profiling events from the buffer or ClickHouse storage.

**Request body (JSON):**

```json
{
  "agent_id": "optional-agent-filter",
  "time_start_ns": 1700000000000000000,
  "time_end_ns": 1700000060000000000,
  "limit": 100,
  "event_type": "cpu"
}
```

- `event_type`: `"cpu"`, `"lock"`, `"syscall"`, or omit for all
- `limit`: max batches to aggregate (capped at 100)
- All fields are optional

**Response:**

```json
{
  "cpu": {
    "start_time": 1700000000000000000,
    "end_time": 1700000060000000000,
    "total_samples": 5000,
    "sample_period_ns": 10000000,
    "stacks": [
      {
        "stack": {
          "frames": [
            { "ip": 4194304, "function": "main", "module": "myapp" }
          ]
        },
        "count": 150
      }
    ]
  },
  "lock": { "..." : "..." },
  "syscall": { "..." : "..." },
  "total_events": 12000,
  "skipped_batches": 0
}
```

### POST /api/diff

Compare two time windows or agent profiles. Requires ClickHouse storage.

**Request body:**

```json
{
  "baseline_agent_id": "agent-1",
  "baseline_start_ns": 1700000000000000000,
  "baseline_end_ns": 1700000030000000000,
  "comparison_agent_id": "agent-1",
  "comparison_start_ns": 1700000030000000000,
  "comparison_end_ns": 1700000060000000000,
  "event_type": "cpu",
  "limit": 100
}
```

**Response:**

```json
{
  "result_json": "{\"baseline_total\":2500,\"comparison_total\":3000,\"stacks\":[...]}",
  "error": ""
}
```

### GET /api/batches

**Query parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `agent_id` | string | (all) | Filter by agent |
| `limit` | number | 100 | Max results |

**Response:**

```json
{
  "batches": [
    {
      "agent_id": "agent-abc123",
      "sequence": 42,
      "event_count": 500,
      "received_at_ns": 1700000000000000000
    }
  ],
  "error": ""
}
```

### GET /api/health

**Response:**

```json
{
  "status": "healthy",
  "buffer_batches": 150,
  "buffer_utilization": 0.015,
  "storage_enabled": true,
  "push_total_ok": 1500,
  "push_total_error": 0,
  "push_events_total": 75000,
  "clickhouse_flush_ok": 30,
  "clickhouse_flush_error": 0,
  "clickhouse_pending_rows": 0
}
```

### POST /api/alerts

Create a new alert rule.

**Request body:**

```json
{
  "name": "High buffer usage",
  "metric": "buffer_utilization",
  "operator": "gt",
  "threshold": 0.9,
  "severity": "warning"
}
```

**Available metrics:** `buffer_utilization`, `push_error_rate`, `push_errors_total`, `clickhouse_flush_errors`, `clickhouse_pending_rows`, `event_throughput`

**Operators:** `gt`, `gte`, `lt`, `lte`, `eq`

**Severities:** `info`, `warning`, `critical`

**Response:**

```json
{ "id": "alert-1" }
```

### POST /api/alerts/evaluate

Evaluates all enabled rules against current aggregator metrics.

**Response:**

```json
{
  "fired": [
    {
      "rule_id": "alert-1",
      "rule_name": "High buffer usage",
      "severity": "warning",
      "metric": "buffer_utilization",
      "value": 0.95,
      "threshold": 0.9,
      "operator": "gt",
      "message": "High buffer usage: Buffer Utilization > 0.9 (current: 0.95)",
      "fired_at": 1700000000
    }
  ],
  "snapshot": {
    "buffer_utilization": 0.95,
    "push_error_rate": 0.0,
    "push_errors_total": 0.0,
    "clickhouse_flush_errors": 0.0,
    "clickhouse_pending_rows": 0.0,
    "event_throughput": 75000.0
  }
}
```

### GET /api/export/json

Download the aggregated profile as a JSON file.

**Query parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `event_type` | string | (all) | `cpu`, `lock`, or `syscall` |
| `limit` | number | 100 | Max batches |

Returns `Content-Disposition: attachment` for browser download.

### GET /api/export/collapsed

Download CPU stacks in Brendan Gregg's collapsed format.

**Query parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | number | 100 | Max batches |

**Output format** (one line per unique stack):

```
main;handle_request;process_data;compute 150
main;handle_request;db_query 95
```

Compatible with: `flamegraph.pl`, speedscope, Grafana Pyroscope, pprof tools.

---

## gRPC Service

Proto file: `aggregator/proto/aperture.proto`

### RPCs

| RPC | Request | Response | Description |
|-----|---------|----------|-------------|
| Push | PushRequest | PushResponse | Ingest agent data |
| Query | QueryRequest | QueryResponse | Query in-memory buffer |
| QueryStorage | QueryStorageRequest | QueryResponse | Query persistent storage |
| Aggregate | AggregateRequest | AggregateResponse | Server-side aggregation |
| Diff | DiffRequest | DiffResponse | Differential profiling |

### Authentication

Set `APERTURE_AUTH_TOKEN` on the aggregator. Agents send it as a `Bearer` token in the `authorization` gRPC metadata.

---

## Prometheus Metrics

Scrape at `http://<aggregator>:9090/metrics`.

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `aperture_push_total` | counter | status=ok\|error | Push RPCs received |
| `aperture_push_events_total` | counter | — | Total events ingested |
| `aperture_push_duration_seconds` | histogram | — | Push RPC latency |
| `aperture_buffer_batches` | gauge | — | Batches in buffer |
| `aperture_buffer_drops_total` | counter | — | Batches dropped (capacity) |
| `aperture_clickhouse_flush_total` | counter | status=ok\|error | ClickHouse flush attempts |
| `aperture_clickhouse_flush_rows_total` | counter | — | Rows flushed |
| `aperture_clickhouse_flush_duration_seconds` | histogram | — | Flush latency |
| `aperture_clickhouse_pending_rows` | gauge | — | Pending rows |
