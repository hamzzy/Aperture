import { useState, useMemo } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { LatencyHistogram } from "@/components/profiler/LatencyHistogram";
import { useDashboard } from "@/contexts/DashboardContext";
import { useAggregateQuery } from "@/api/queries";
import { formatNs } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { SyscallStats } from "@/api/types";

type SortKey = "count" | "avg" | "max" | "min" | "errors" | "total";

interface SyscallRow extends SyscallStats {
  avg_duration_ns: number;
  pct: number;
}

export default function SyscallsPage() {
  const dashboard = useDashboard();
  const { start, end } = dashboard?.timeRange ?? { start: 0, end: 0 };
  const aggregateQuery = useAggregateQuery({
    time_start_ns: start,
    time_end_ns: end,
    limit: 500,
    event_type: "syscall",
    enabled: !!dashboard,
  });
  const data = aggregateQuery.data;
  const syscall = data?.syscall;

  const [sortKey, setSortKey] = useState<SortKey>("count");
  const [sortAsc, setSortAsc] = useState(false);
  const [selectedSyscall, setSelectedSyscall] = useState<string | null>(null);

  const rows = useMemo((): SyscallRow[] => {
    if (!syscall) return [];
    const totalEvents = syscall.total_events || 1;
    return Object.values(syscall.syscalls).map((s) => ({
      ...s,
      avg_duration_ns: s.count > 0 ? s.total_duration_ns / s.count : 0,
      pct: (s.count / totalEvents) * 100,
    }));
  }, [syscall]);

  const sorted = useMemo(() => {
    return [...rows].sort((a, b) => {
      let diff: number;
      switch (sortKey) {
        case "count":
          diff = a.count - b.count;
          break;
        case "avg":
          diff = a.avg_duration_ns - b.avg_duration_ns;
          break;
        case "max":
          diff = a.max_duration_ns - b.max_duration_ns;
          break;
        case "min":
          diff = a.min_duration_ns - b.min_duration_ns;
          break;
        case "errors":
          diff = a.error_count - b.error_count;
          break;
        case "total":
          diff = a.total_duration_ns - b.total_duration_ns;
          break;
        default:
          diff = 0;
      }
      return sortAsc ? diff : -diff;
    });
  }, [rows, sortKey, sortAsc]);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(false);
    }
  };

  const sortIndicator = (key: SortKey) => {
    if (sortKey !== key) return " \u2195";
    return sortAsc ? " \u2191" : " \u2193";
  };

  const selectedRow = selectedSyscall
    ? rows.find((r) => r.name === selectedSyscall)
    : null;

  const totalEvents = syscall?.total_events ?? 0;
  const uniqueSyscalls = rows.length;
  const totalDuration = rows.reduce((s, r) => s + r.total_duration_ns, 0);
  const totalErrors = rows.reduce((s, r) => s + r.error_count, 0);

  return (
    <AppLayout>
      <div className="space-y-4">
        <h1 className="text-lg font-semibold text-foreground">Syscalls</h1>

        {aggregateQuery.isLoading && (
          <p className="text-xs text-muted-foreground">Loading syscall data…</p>
        )}
        {aggregateQuery.error && (
          <p className="text-xs text-destructive">{aggregateQuery.error.message}</p>
        )}

        {/* Summary stats */}
        <div className="grid grid-cols-4 gap-3">
          <div className="rounded-md border border-border bg-card p-3">
            <div className="text-[11px] text-muted-foreground">Total events</div>
            <div className="text-lg font-semibold font-mono text-foreground">
              {totalEvents.toLocaleString()}
            </div>
          </div>
          <div className="rounded-md border border-border bg-card p-3">
            <div className="text-[11px] text-muted-foreground">Unique syscalls</div>
            <div className="text-lg font-semibold font-mono text-foreground">
              {uniqueSyscalls}
            </div>
          </div>
          <div className="rounded-md border border-border bg-card p-3">
            <div className="text-[11px] text-muted-foreground">Total duration</div>
            <div className="text-lg font-semibold font-mono text-foreground">
              {totalDuration > 0 ? formatNs(totalDuration) : "—"}
            </div>
          </div>
          <div className="rounded-md border border-border bg-card p-3">
            <div className="text-[11px] text-muted-foreground">Total errors</div>
            <div className={cn(
              "text-lg font-semibold font-mono",
              totalErrors > 0 ? "text-destructive" : "text-foreground",
            )}>
              {totalErrors.toLocaleString()}
            </div>
          </div>
        </div>

        {/* Selected syscall histogram */}
        {selectedRow && (
          <div className="rounded-md border border-border bg-card p-4">
            <div className="flex items-center justify-between mb-2">
              <h2 className="text-sm font-medium text-foreground">
                Latency distribution:{" "}
                <span className="font-mono text-primary">{selectedRow.name}</span>
              </h2>
              <button
                onClick={() => setSelectedSyscall(null)}
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Close
              </button>
            </div>
            <div className="flex items-center gap-4 text-[11px] text-muted-foreground mb-3">
              <span>Count: <span className="font-mono text-foreground">{selectedRow.count.toLocaleString()}</span></span>
              <span>Avg: <span className="font-mono text-foreground">{formatNs(selectedRow.avg_duration_ns)}</span></span>
              <span>Min: <span className="font-mono text-foreground">{formatNs(selectedRow.min_duration_ns)}</span></span>
              <span>Max: <span className="font-mono text-foreground">{formatNs(selectedRow.max_duration_ns)}</span></span>
            </div>
            <LatencyHistogram histogram={selectedRow.latency_histogram} height={140} />
          </div>
        )}

        {/* Syscall table */}
        {rows.length === 0 && !aggregateQuery.isLoading ? (
          <div className="rounded-md border border-border bg-card p-8 text-center text-sm text-muted-foreground">
            No syscall data available.
          </div>
        ) : rows.length > 0 && (
          <div className="rounded-md border border-border overflow-hidden">
            <div className="overflow-auto" style={{ maxHeight: 600 }}>
              <table className="w-full text-xs">
                <thead className="sticky top-0 z-10">
                  <tr className="border-b border-border bg-muted/50">
                    <th className="text-left px-3 py-2 font-medium text-muted-foreground w-[25%]">
                      Syscall
                    </th>
                    <th
                      className={cn(
                        "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                        sortKey === "count" ? "text-foreground" : "text-muted-foreground",
                      )}
                      onClick={() => handleSort("count")}
                    >
                      Count{sortIndicator("count")}
                    </th>
                    <th className="text-right px-3 py-2 font-medium text-muted-foreground whitespace-nowrap">
                      % Total
                    </th>
                    <th
                      className={cn(
                        "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                        sortKey === "avg" ? "text-foreground" : "text-muted-foreground",
                      )}
                      onClick={() => handleSort("avg")}
                    >
                      Avg latency{sortIndicator("avg")}
                    </th>
                    <th
                      className={cn(
                        "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                        sortKey === "min" ? "text-foreground" : "text-muted-foreground",
                      )}
                      onClick={() => handleSort("min")}
                    >
                      Min{sortIndicator("min")}
                    </th>
                    <th
                      className={cn(
                        "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                        sortKey === "max" ? "text-foreground" : "text-muted-foreground",
                      )}
                      onClick={() => handleSort("max")}
                    >
                      Max{sortIndicator("max")}
                    </th>
                    <th
                      className={cn(
                        "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                        sortKey === "total" ? "text-foreground" : "text-muted-foreground",
                      )}
                      onClick={() => handleSort("total")}
                    >
                      Total time{sortIndicator("total")}
                    </th>
                    <th
                      className={cn(
                        "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                        sortKey === "errors" ? "text-foreground" : "text-muted-foreground",
                      )}
                      onClick={() => handleSort("errors")}
                    >
                      Errors{sortIndicator("errors")}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {sorted.map((row) => (
                    <tr
                      key={row.syscall_id}
                      className={cn(
                        "border-b border-border/50 hover:bg-muted/20 transition-colors cursor-pointer",
                        selectedSyscall === row.name && "bg-primary/5",
                      )}
                      onClick={() =>
                        setSelectedSyscall(
                          selectedSyscall === row.name ? null : row.name,
                        )
                      }
                    >
                      <td className="px-3 py-1.5">
                        <span className="font-mono text-foreground">{row.name}</span>
                        <span className="ml-1.5 text-[10px] text-muted-foreground">
                          #{row.syscall_id}
                        </span>
                      </td>
                      <td className="text-right px-3 py-1.5 font-mono text-foreground">
                        {row.count.toLocaleString()}
                      </td>
                      <td className="text-right px-3 py-1.5">
                        <div className="flex items-center justify-end gap-2">
                          <div className="w-10 h-1.5 rounded-full bg-muted overflow-hidden">
                            <div
                              className="h-full rounded-full bg-primary/60"
                              style={{ width: `${Math.min(100, row.pct)}%` }}
                            />
                          </div>
                          <span className="font-mono text-muted-foreground w-12 text-right">
                            {row.pct.toFixed(1)}%
                          </span>
                        </div>
                      </td>
                      <td className="text-right px-3 py-1.5 font-mono text-foreground">
                        {formatNs(row.avg_duration_ns)}
                      </td>
                      <td className="text-right px-3 py-1.5 font-mono text-muted-foreground">
                        {formatNs(row.min_duration_ns)}
                      </td>
                      <td className="text-right px-3 py-1.5 font-mono text-foreground">
                        {formatNs(row.max_duration_ns)}
                      </td>
                      <td className="text-right px-3 py-1.5 font-mono text-muted-foreground">
                        {formatNs(row.total_duration_ns)}
                      </td>
                      <td className="text-right px-3 py-1.5 font-mono">
                        {row.error_count > 0 ? (
                          <span className="text-destructive">
                            {row.error_count.toLocaleString()}
                          </span>
                        ) : (
                          <span className="text-muted-foreground">0</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        <p className="text-[11px] text-muted-foreground">
          Click a row to view its latency histogram. Syscall events are captured via raw tracepoints and do not include stack traces.
        </p>
      </div>
    </AppLayout>
  );
}
