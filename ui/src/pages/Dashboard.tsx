import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { AppLayout } from "@/components/layout/AppLayout";
import { CpuTimelineChart } from "@/components/profiler/CpuTimelineChart";
import { Flame, BarChart3, GitCompare, Shield, Database, Activity } from "lucide-react";
import { usePhase8 } from "@/contexts/Phase8Context";
import { useHealthQuery, useAggregateQuery, useBatchesQuery } from "@/api/queries";
import { formatNs } from "@/lib/format";
import type { HealthInfo } from "@/api/types";

function StatCard({
  label,
  value,
  sub,
  positive,
}: {
  label: string;
  value: string;
  sub?: string;
  positive?: boolean;
}) {
  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="text-[11px] text-muted-foreground mb-1">{label}</div>
      <div className="text-xl font-semibold font-mono text-foreground">{value}</div>
      {sub && (
        <div className={`text-[11px] mt-1 ${positive === false ? "text-destructive" : "text-muted-foreground"}`}>
          {sub}
        </div>
      )}
    </div>
  );
}

function HealthBadge({ health }: { health: HealthInfo | null }) {
  if (!health) {
    return (
      <div className="flex items-center gap-2">
        <span className="h-2 w-2 rounded-full bg-muted-foreground" />
        <span className="text-xs text-muted-foreground">Connecting…</span>
      </div>
    );
  }
  const isHealthy = health.status === "healthy";
  return (
    <div className="flex items-center gap-2">
      <span className={`h-2 w-2 rounded-full ${isHealthy ? "bg-success animate-pulse" : "bg-warning animate-pulse"}`} />
      <span className="text-xs text-muted-foreground">
        {isHealthy ? "Healthy" : "Degraded"}
        {" · "}
        {health.push_events_total.toLocaleString()} events ingested
      </span>
    </div>
  );
}

export default function Dashboard() {
  const phase8 = usePhase8();
  const [eventType, setEventType] = useState<"cpu" | "lock" | "syscall" | "">("");
  const { start, end } = phase8?.timeRange ?? { start: 0, end: 0 };

  const healthQuery = useHealthQuery();
  const aggregateQuery = useAggregateQuery({
    time_start_ns: start,
    time_end_ns: end,
    limit: 20,
    event_type: eventType || undefined,
    enabled: !!phase8,
  });
  const batchesQuery = useBatchesQuery({ limit: 50 });

  const aggregate = aggregateQuery.data ?? null;
  const batches = batchesQuery.data?.batches ?? [];
  const health = healthQuery.data ?? null;
  const error =
    aggregateQuery.error?.message ??
    batchesQuery.error?.message ??
    (healthQuery.isError ? healthQuery.error?.message : null);

  useEffect(() => {
    if (phase8) {
      phase8.registerRefresh(() => {
        healthQuery.refetch();
        aggregateQuery.refetch();
        batchesQuery.refetch();
      });
    }
  }, [phase8?.registerRefresh]);

  useEffect(() => {
    phase8?.setRefreshing(
      aggregateQuery.isFetching || batchesQuery.isFetching || healthQuery.isFetching
    );
  }, [aggregateQuery.isFetching, batchesQuery.isFetching, healthQuery.isFetching, phase8?.setRefreshing]);

  const cpu = aggregate?.cpu;
  const lock = aggregate?.lock;
  const syscall = aggregate?.syscall;
  const totalSamples = cpu?.total_samples ?? 0;
  const stacksCount = cpu?.stacks?.length ?? 0;
  const durationNs = cpu ? cpu.end_time - cpu.start_time : 0;

  const lockEvents = lock?.total_events ?? 0;
  const lockContentions = lock?.contentions?.length ?? 0;
  const syscallEvents = syscall?.total_events ?? 0;
  const syscallCount = syscall ? Object.keys(syscall.syscalls).length : 0;

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold text-foreground">Dashboard</h1>
          <HealthBadge health={health} />
        </div>

        {error && (
          <div className="rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
            {error}
            {!error.includes("port") && !error.includes("storage") && (
              <> — Ensure the aggregator is running (e.g. port 9090) and storage is enabled.</>
            )}
          </div>
        )}
        {aggregate?.skipped_batches != null && aggregate.skipped_batches > 0 && (
          <div className="rounded-md border border-amber-500/50 bg-amber-500/10 px-4 py-2 text-sm text-amber-700 dark:text-amber-400">
            {aggregate.skipped_batches} batch(es) skipped (invalid or corrupt data). Results are partial.
          </div>
        )}

        {/* Event type selector */}
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">Profile type:</span>
          {(["cpu", "lock", "syscall", ""] as const).map((t) => (
            <button
              key={t || "all"}
              onClick={() => setEventType(t)}
              className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                eventType === t
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground hover:text-foreground"
              }`}
            >
              {t === "" ? "All" : t === "cpu" ? "CPU" : t === "lock" ? "Lock" : "Syscall"}
            </button>
          ))}
        </div>

        {/* Stats cards */}
        <div className="grid grid-cols-4 gap-3">
          {(eventType === "cpu" || eventType === "") && (
            <>
              <StatCard
                label="CPU samples"
                value={aggregate ? totalSamples.toLocaleString() : "—"}
                sub={`${stacksCount} unique stacks`}
              />
              <StatCard
                label="CPU duration"
                value={durationNs ? formatNs(durationNs) : "—"}
                sub="Profile window"
              />
            </>
          )}
          {(eventType === "lock" || eventType === "") && (
            <StatCard
              label="Lock events"
              value={aggregate ? lockEvents.toLocaleString() : "—"}
              sub={`${lockContentions} unique contentions`}
            />
          )}
          {(eventType === "syscall" || eventType === "") && (
            <StatCard
              label="Syscall events"
              value={aggregate ? syscallEvents.toLocaleString() : "—"}
              sub={`${syscallCount} unique syscalls`}
            />
          )}
          <StatCard
            label="Batches (buffer)"
            value={batches.length.toLocaleString()}
            sub={health ? `${(health.buffer_utilization * 100).toFixed(0)}% full` : "Recent ingest batches"}
          />
        </div>

        {/* Health panel (Phase 7 metrics) */}
        {health && (
          <div className="grid grid-cols-3 gap-3">
            <div className="rounded-md border border-border bg-card p-3 flex items-center gap-3">
              <Shield className="h-4 w-4 text-muted-foreground" />
              <div>
                <div className="text-[11px] text-muted-foreground">Push RPCs</div>
                <div className="text-sm font-mono text-foreground">
                  {health.push_total_ok.toLocaleString()} ok
                  {health.push_total_error > 0 && (
                    <span className="text-destructive ml-1">
                      / {health.push_total_error.toLocaleString()} err
                    </span>
                  )}
                </div>
              </div>
            </div>
            <div className="rounded-md border border-border bg-card p-3 flex items-center gap-3">
              <Database className="h-4 w-4 text-muted-foreground" />
              <div>
                <div className="text-[11px] text-muted-foreground">ClickHouse</div>
                <div className="text-sm font-mono text-foreground">
                  {health.storage_enabled ? (
                    <>
                      {health.clickhouse_flush_ok.toLocaleString()} flushes
                      {health.clickhouse_pending_rows > 0 && (
                        <span className="text-warning ml-1">({health.clickhouse_pending_rows} pending)</span>
                      )}
                    </>
                  ) : (
                    <span className="text-muted-foreground">disabled</span>
                  )}
                </div>
              </div>
            </div>
            <div className="rounded-md border border-border bg-card p-3 flex items-center gap-3">
              <Activity className="h-4 w-4 text-muted-foreground" />
              <div>
                <div className="text-[11px] text-muted-foreground">Buffer</div>
                <div className="text-sm font-mono text-foreground">
                  {health.buffer_batches} batches · {(health.buffer_utilization * 100).toFixed(1)}%
                </div>
              </div>
            </div>
          </div>
        )}

        <div className="grid grid-cols-2 gap-3">
          <CpuTimelineChart title="CPU Core Usage" height={140} />
          <CpuTimelineChart title="GPU Kernel Activity" height={140} />
        </div>

        {/* Syscall table when syscall mode is selected */}
        {syscall && Object.keys(syscall.syscalls).length > 0 && (eventType === "syscall" || eventType === "") && (
          <div className="rounded-md border border-border bg-card p-4">
            <h2 className="text-sm font-medium text-foreground mb-3">Syscall Profile</h2>
            <div className="rounded-md border border-border overflow-hidden">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border bg-muted/30">
                    <th className="text-left px-3 py-2 font-medium text-muted-foreground">Syscall</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Count</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Avg (us)</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Max (us)</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Errors</th>
                  </tr>
                </thead>
                <tbody>
                  {Object.values(syscall.syscalls)
                    .sort((a, b) => b.count - a.count)
                    .slice(0, 20)
                    .map((s) => {
                      const avgUs = s.count > 0 ? s.total_duration_ns / s.count / 1000 : 0;
                      const maxUs = s.max_duration_ns / 1000;
                      return (
                        <tr key={s.syscall_id} className="border-b border-border/50 hover:bg-muted/20">
                          <td className="px-3 py-2 font-mono text-foreground">{s.name}</td>
                          <td className="text-right px-3 py-2 font-mono">{s.count.toLocaleString()}</td>
                          <td className="text-right px-3 py-2 font-mono">{avgUs.toFixed(1)}</td>
                          <td className="text-right px-3 py-2 font-mono">{maxUs.toFixed(1)}</td>
                          <td className="text-right px-3 py-2 font-mono">
                            {s.error_count > 0 ? (
                              <span className="text-destructive">{s.error_count.toLocaleString()}</span>
                            ) : (
                              "0"
                            )}
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Lock contention table when lock mode is selected */}
        {lock && lock.contentions.length > 0 && (eventType === "lock" || eventType === "") && (
          <div className="rounded-md border border-border bg-card p-4">
            <h2 className="text-sm font-medium text-foreground mb-3">Lock Contention</h2>
            <div className="rounded-md border border-border overflow-hidden">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border bg-muted/30">
                    <th className="text-left px-3 py-2 font-medium text-muted-foreground">Lock</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Count</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Total wait</th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground">Max wait</th>
                  </tr>
                </thead>
                <tbody>
                  {lock.contentions.slice(0, 20).map((c, i) => {
                    const topFrame = c.stack.frames[0];
                    const label = topFrame?.function ?? `0x${c.lock_addr.toString(16)}`;
                    return (
                      <tr key={i} className="border-b border-border/50 hover:bg-muted/20">
                        <td className="px-3 py-2 font-mono text-foreground truncate max-w-xs" title={label}>{label}</td>
                        <td className="text-right px-3 py-2 font-mono">{c.count.toLocaleString()}</td>
                        <td className="text-right px-3 py-2 font-mono">{formatNs(c.total_wait_ns)}</td>
                        <td className="text-right px-3 py-2 font-mono">{formatNs(c.max_wait_ns)}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        )}

        <div className="flex flex-wrap gap-2">
          <Link to="/flamegraph">
            <span className="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-3 py-2 text-xs text-foreground hover:bg-muted/50">
              <Flame className="h-3.5 w-3.5" />
              Flamegraph
            </span>
          </Link>
          <Link to="/functions">
            <span className="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-3 py-2 text-xs text-foreground hover:bg-muted/50">
              <BarChart3 className="h-3.5 w-3.5" />
              Top Functions
            </span>
          </Link>
          <Link to="/comparison">
            <span className="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-3 py-2 text-xs text-foreground hover:bg-muted/50">
              <GitCompare className="h-3.5 w-3.5" />
              Compare
            </span>
          </Link>
        </div>

        <div className="rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-medium text-foreground mb-3">Recent batches</h2>
          <div className="space-y-2 max-h-48 overflow-auto">
            {batches.length === 0 ? (
              <p className="text-xs text-muted-foreground">No batches in buffer.</p>
            ) : (
              batches.slice(0, 20).map((b, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 text-[11px] py-1.5 border-b border-border/30 last:border-0"
                >
                  <span className="font-mono text-muted-foreground w-24 shrink-0">{b.agent_id}</span>
                  <span className="font-mono text-muted-foreground w-16">seq {b.sequence}</span>
                  <span className="font-mono text-foreground/80">{b.event_count} events</span>
                  <span className="font-mono text-muted-foreground text-[10px]">
                    {new Date(b.received_at_ns / 1e6).toLocaleTimeString()}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </AppLayout>
  );
}
