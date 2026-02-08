---
sidebar_position: 5
title: Alerting
---

# Alerting

The aggregator includes an in-memory alert engine that monitors system metrics and fires alerts when thresholds are exceeded.

## Concepts

- **Rules** define what to monitor: a metric, comparison operator, threshold, and severity level
- **Evaluation** checks all enabled rules against current metric values
- **History** stores recently fired alerts (ring buffer, 500 events max)

:::note
Rules and history are stored in memory only. They are lost when the aggregator restarts.
:::

## Available Metrics

| Metric | Description | Range |
|--------|-------------|-------|
| `buffer_utilization` | Fraction of in-memory buffer used | 0.0 - 1.0 |
| `push_error_rate` | Ratio of failed pushes to total pushes | 0.0 - 1.0 |
| `push_errors_total` | Absolute count of failed push RPCs | 0+ |
| `clickhouse_flush_errors` | Count of ClickHouse flush failures | 0+ |
| `clickhouse_pending_rows` | Rows waiting to be flushed | 0+ |
| `event_throughput` | Total events ingested | 0+ |

## Creating Rules

### Via REST API

```bash
curl -X POST http://localhost:9090/api/alerts \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "High buffer usage",
    "metric": "buffer_utilization",
    "operator": "gt",
    "threshold": 0.9,
    "severity": "warning"
  }'
```

### Via Web UI

Navigate to the **Alerts** page in the dashboard. Use the form at the top to create rules with:
- **Name** — descriptive label
- **Metric** — select from dropdown
- **Operator** — `>`, `>=`, `<`, `<=`, `=`
- **Threshold** — numeric value
- **Severity** — Info, Warning, or Critical

## Operators

| API Value | Symbol | Description |
|-----------|--------|-------------|
| `gt` | `>` | Greater than |
| `gte` | `>=` | Greater than or equal |
| `lt` | `<` | Less than |
| `lte` | `<=` | Less than or equal |
| `eq` | `=` | Equal to |

## Evaluating Rules

Alerts are evaluated on demand (not continuously). Trigger evaluation via:

### REST API

```bash
curl -X POST http://localhost:9090/api/alerts/evaluate
```

### Web UI

Click the **Evaluate Now** button on the Alerts page.

### Response

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

## Managing Rules

### List all rules

```bash
curl http://localhost:9090/api/alerts
```

### Toggle enable/disable

```bash
curl -X POST http://localhost:9090/api/alerts/alert-1/toggle
```

### Delete a rule

```bash
curl -X DELETE http://localhost:9090/api/alerts/alert-1
```

### View history

```bash
curl http://localhost:9090/api/alerts/history?limit=50
```

## Example Alert Rules

### Production monitoring

```bash
# Alert when buffer is almost full
curl -X POST http://localhost:9090/api/alerts \
  -H 'Content-Type: application/json' \
  -d '{"name":"Buffer near capacity","metric":"buffer_utilization","operator":"gt","threshold":0.8,"severity":"warning"}'

# Alert on push errors
curl -X POST http://localhost:9090/api/alerts \
  -H 'Content-Type: application/json' \
  -d '{"name":"Push errors detected","metric":"push_errors_total","operator":"gt","threshold":0,"severity":"critical"}'

# Alert on ClickHouse flush failures
curl -X POST http://localhost:9090/api/alerts \
  -H 'Content-Type: application/json' \
  -d '{"name":"Storage flush failing","metric":"clickhouse_flush_errors","operator":"gt","threshold":0,"severity":"critical"}'
```
