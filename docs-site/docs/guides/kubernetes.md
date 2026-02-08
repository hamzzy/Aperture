---
sidebar_position: 4
title: Kubernetes Deployment
---

# Kubernetes Deployment

Aperture can be deployed on Kubernetes with the agent running as a DaemonSet (one per node) and the aggregator as a Deployment.

## Architecture

```
┌─────────────────────────────────────────────┐
│              Kubernetes Cluster              │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Node 1   │  │ Node 2   │  │ Node 3   │  │
│  │ Agent DS │  │ Agent DS │  │ Agent DS │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └──────────────┼──────────────┘        │
│                      ▼                       │
│            ┌──────────────────┐              │
│            │   Aggregator    │              │
│            │   (Deployment)  │              │
│            └────────┬────────┘              │
│                     ▼                        │
│            ┌─────────────────┐              │
│            │   ClickHouse    │              │
│            │  (StatefulSet)  │              │
│            └─────────────────┘              │
└─────────────────────────────────────────────┘
```

## Prerequisites

- Kubernetes 1.24+
- Nodes with Linux kernel 5.10+ (for eBPF)
- `kubectl` configured for your cluster

## Deploy

### 1. Create Namespace

```bash
kubectl apply -f deploy/k8s/namespace.yml
```

```yaml
# deploy/k8s/namespace.yml
apiVersion: v1
kind: Namespace
metadata:
  name: aperture
  labels:
    app.kubernetes.io/part-of: aperture
```

### 2. Deploy Aggregator

```bash
kubectl apply -f deploy/k8s/aggregator.yml
```

The aggregator runs as a single-replica Deployment with a ClusterIP Service:

- **gRPC** on port 50051 (agent data ingestion)
- **HTTP** on port 9090 (REST API, metrics, health checks)
- Liveness probe: `/healthz`
- Readiness probe: `/readyz`
- Prometheus annotations for auto-scraping

### 3. Deploy Agent DaemonSet

```bash
kubectl apply -f deploy/k8s/agent-daemonset.yml
```

The agent runs with:

- **Privileged mode** (required for eBPF)
- **Host PID namespace** (for process symbol resolution)
- Volume mounts: `/sys/kernel/debug` (debugfs), `/proc` (host proc)
- Automatic aggregator discovery via Kubernetes DNS: `aperture-aggregator.aperture.svc.cluster.local:50051`

## Configuration

### Aggregator Environment Variables

| Variable | Description |
|----------|-------------|
| `APERTURE_AUTH_TOKEN` | Bearer token for gRPC auth |
| `APERTURE_CLICKHOUSE_ENDPOINT` | ClickHouse HTTP URL |
| `APERTURE_CLICKHOUSE_DATABASE` | Database name (default: `aperture`) |
| `APERTURE_CLICKHOUSE_PASSWORD` | ClickHouse password |
| `APERTURE_BUFFER_CAPACITY` | In-memory buffer size |

### Agent Environment Variables

| Variable | Description |
|----------|-------------|
| `APERTURE_AGGREGATOR` | Aggregator gRPC URL |
| `APERTURE_AUTH_TOKEN` | Bearer token |
| `APERTURE_MODE` | `cpu`, `lock`, `syscall`, or `all` |
| `APERTURE_FREQ` | Sampling frequency in Hz |

## Monitoring

### Prometheus

The aggregator exposes a `/metrics` endpoint on port 9090. The DaemonSet and Deployment include Prometheus annotations:

```yaml
annotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "9090"
  prometheus.io/path: "/metrics"
```

### Health Checks

- **Liveness:** `GET /healthz` returns `ok`
- **Readiness:** `GET /readyz` checks buffer state

## Scaling

- **Agent:** Automatically scales with nodes (DaemonSet). One agent per node.
- **Aggregator:** Single replica is sufficient for most workloads. For high-throughput clusters, increase replicas and add a load balancer for the gRPC service.
- **ClickHouse:** Use a StatefulSet with replication for production persistence.

## Troubleshooting

### Agent pods fail to start

Check that nodes have eBPF support:

```bash
kubectl logs -n aperture -l app=aperture-agent
```

Common issues:
- Kernel too old (need 5.10+)
- Missing debugfs mount
- Insufficient privileges (needs `privileged: true`)

### No data in aggregator

Verify agent connectivity:

```bash
kubectl exec -n aperture -it deploy/aperture-aggregator -- \
  curl http://localhost:9090/api/health
```

Check that `push_total_ok > 0` in the health response.
