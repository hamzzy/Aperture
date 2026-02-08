/**
 * Aperture REST API client (aggregator admin port, e.g. 9090)
 */

import type {
  AggregateResultJson,
  AlertEvent,
  AlertMetric,
  AlertOperator,
  AlertRule,
  AlertSeverity,
  BatchInfo,
  EvaluateResult,
  HealthInfo,
} from "./types";

const API = "/api";

/** Optional bearer token for authenticated aggregators */
let authToken: string | null = null;

export function setAuthToken(token: string | null) {
  authToken = token;
}

function headers(): HeadersInit {
  const h: Record<string, string> = { "Content-Type": "application/json" };
  if (authToken) h["Authorization"] = `Bearer ${authToken}`;
  return h;
}

export async function fetchAggregate(params: {
  agent_id?: string;
  time_start_ns?: number;
  time_end_ns?: number;
  limit?: number;
  event_type?: string;
}): Promise<AggregateResultJson> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);
  try {
    const res = await fetch(`${API}/aggregate`, {
      method: "POST",
      headers: headers(),
      body: JSON.stringify(params),
      signal: controller.signal,
    });
    if (!res.ok) throw new Error(await res.text());
    return res.json();
  } finally {
    clearTimeout(timeout);
  }
}

export async function fetchDiff(params: {
  baseline_agent_id?: string;
  baseline_start_ns?: number;
  baseline_end_ns?: number;
  comparison_agent_id?: string;
  comparison_start_ns?: number;
  comparison_end_ns?: number;
  event_type?: string;
  limit?: number;
}): Promise<{ result_json: string; error: string }> {
  const res = await fetch(`${API}/diff`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify(params),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function fetchBatches(params?: {
  agent_id?: string;
  limit?: number;
}): Promise<{ batches: BatchInfo[]; error: string }> {
  const q = new URLSearchParams();
  if (params?.agent_id) q.set("agent_id", params.agent_id);
  if (params?.limit) q.set("limit", String(params.limit ?? 50));
  const qs = q.toString();
  const res = await fetch(`${API}/batches${qs ? `?${qs}` : ""}`, {
    headers: headers(),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function fetchHealth(): Promise<HealthInfo> {
  const res = await fetch(`${API}/health`, { headers: headers() });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

// ── Alerts ──────────────────────────────────────────────────────────────

export async function fetchAlertRules(): Promise<AlertRule[]> {
  const res = await fetch(`${API}/alerts`, { headers: headers() });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function createAlertRule(params: {
  name: string;
  metric: AlertMetric;
  operator: AlertOperator;
  threshold: number;
  severity: AlertSeverity;
}): Promise<{ id: string }> {
  const res = await fetch(`${API}/alerts`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify(params),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function deleteAlertRule(id: string): Promise<{ deleted: boolean }> {
  const res = await fetch(`${API}/alerts/${id}`, {
    method: "DELETE",
    headers: headers(),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function toggleAlertRule(id: string): Promise<{ enabled: boolean }> {
  const res = await fetch(`${API}/alerts/${id}/toggle`, {
    method: "POST",
    headers: headers(),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function fetchAlertHistory(limit?: number): Promise<AlertEvent[]> {
  const q = limit ? `?limit=${limit}` : "";
  const res = await fetch(`${API}/alerts/history${q}`, { headers: headers() });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function evaluateAlerts(): Promise<EvaluateResult> {
  const res = await fetch(`${API}/alerts/evaluate`, {
    method: "POST",
    headers: headers(),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
