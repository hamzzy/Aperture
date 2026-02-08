/**
 * Aperture Phase 8 – REST API types (aggregator /api/aggregate, /api/diff, /api/batches, /api/health)
 */

export interface Frame {
  ip: number;
  function?: string;
  file?: string;
  line?: number;
  module?: string;
}

export interface Stack {
  frames: Frame[];
}

export interface StackCount {
  stack: Stack;
  count: number;
}

export interface CpuProfileJson {
  start_time: number;
  end_time: number;
  total_samples: number;
  sample_period_ns: number;
  stacks: StackCount[];
}

export interface LockContentionJson {
  lock_addr: number;
  stack: Stack;
  count: number;
  total_wait_ns: number;
  max_wait_ns: number;
  min_wait_ns: number;
}

export interface LockProfileJson {
  start_time: number;
  end_time: number;
  total_events: number;
  contentions: LockContentionJson[];
}

export interface SyscallStats {
  syscall_id: number;
  name: string;
  count: number;
  total_duration_ns: number;
  max_duration_ns: number;
  min_duration_ns: number;
  error_count: number;
  latency_histogram: number[];
}

export interface SyscallProfileJson {
  start_time: number;
  end_time: number;
  syscalls: Record<string, SyscallStats>;
  total_events: number;
}

export interface AggregateResultJson {
  cpu?: CpuProfileJson;
  lock?: LockProfileJson;
  syscall?: SyscallProfileJson;
  total_events: number;
  /** Batches skipped due to invalid/corrupt payload (bincode decode errors). */
  skipped_batches?: number;
}

export interface StackDiff {
  stack: Stack;
  baseline_count: number;
  comparison_count: number;
  delta: number;
  delta_pct: number;
}

export interface CpuDiffJson {
  baseline_total: number;
  comparison_total: number;
  stacks: StackDiff[];
}

export interface BatchInfo {
  agent_id: string;
  sequence: number;
  event_count: number;
  received_at_ns: number;
}

export interface HealthInfo {
  status: "healthy" | "degraded";
  buffer_batches: number;
  buffer_utilization: number;
  storage_enabled: boolean;
  push_total_ok: number;
  push_total_error: number;
  push_events_total: number;
  clickhouse_flush_ok: number;
  clickhouse_flush_error: number;
  clickhouse_pending_rows: number;
}

// ── Alerts ──────────────────────────────────────────────────────────────

export type AlertMetric =
  | "buffer_utilization"
  | "push_error_rate"
  | "push_errors_total"
  | "clickhouse_flush_errors"
  | "clickhouse_pending_rows"
  | "event_throughput";

export type AlertOperator = "gt" | "gte" | "lt" | "lte" | "eq";

export type AlertSeverity = "info" | "warning" | "critical";

export interface AlertRule {
  id: string;
  name: string;
  metric: AlertMetric;
  operator: AlertOperator;
  threshold: number;
  severity: AlertSeverity;
  enabled: boolean;
  created_at: number;
}

export interface AlertEvent {
  rule_id: string;
  rule_name: string;
  severity: AlertSeverity;
  metric: AlertMetric;
  value: number;
  threshold: number;
  operator: AlertOperator;
  message: string;
  fired_at: number;
}

export interface MetricSnapshot {
  buffer_utilization: number;
  push_error_rate: number;
  push_errors_total: number;
  clickhouse_flush_errors: number;
  clickhouse_pending_rows: number;
  event_throughput: number;
}

export interface EvaluateResult {
  fired: AlertEvent[];
  snapshot: MetricSnapshot;
}
