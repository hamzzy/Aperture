import { useState, useMemo } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { TopFunctionsTable } from "@/components/profiler/TopFunctionsTable";
import { CpuTimelineChart } from "@/components/profiler/CpuTimelineChart";
import { useDashboard } from "@/contexts/DashboardContext";
import { useAggregateQuery, useBatchesQuery } from "@/api/queries";
import type { StackCount } from "@/api/types";

export default function TopFunctionsPage() {
  const dashboard = useDashboard();
  const { start, end } = dashboard?.timeRange ?? { start: 0, end: 0 };
  const [eventType, setEventType] = useState<"cpu" | "lock" | "">("");
  const batchesQuery = useBatchesQuery({ limit: 50 });
  const aggregateQuery = useAggregateQuery({
    time_start_ns: start,
    time_end_ns: end,
    limit: 500,
    event_type: eventType || undefined,
    enabled: !!dashboard,
  });
  const data = aggregateQuery.data;

  const { stacks, totalSamples, activeType } = useMemo(() => {
    const cpu = data?.cpu;
    const lock = data?.lock;
    if (eventType === "cpu" || (!eventType && cpu && (cpu.stacks?.length ?? 0) > 0)) {
      return {
        stacks: cpu?.stacks ?? [],
        totalSamples: cpu?.total_samples ?? 0,
        activeType: "cpu" as const,
      };
    }
    if (eventType === "lock" || (!eventType && lock && (lock.contentions?.length ?? 0) > 0)) {
      const lockStacks: StackCount[] = (lock?.contentions ?? []).map((c) => ({
        stack: c.stack,
        count: c.count,
      }));
      return {
        stacks: lockStacks,
        totalSamples: lock?.total_events ?? 0,
        activeType: "lock" as const,
      };
    }
    return {
      stacks: cpu?.stacks ?? [],
      totalSamples: cpu?.total_samples ?? 0,
      activeType: "cpu" as const,
    };
  }, [data, eventType]);

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-lg font-semibold text-foreground">
              Top {activeType === "lock" ? "Lock Contentions" : "Functions"}
            </h1>
            <div className="flex items-center gap-1 text-xs">
              <span className="text-muted-foreground">Type:</span>
              {(["", "cpu", "lock"] as const).map((t) => (
                <button
                  key={t || "all"}
                  onClick={() => setEventType(t)}
                  className={`px-2.5 py-1 rounded text-xs font-medium transition-colors ${
                    eventType === t
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {t === "" ? "All" : t === "cpu" ? "CPU" : "Lock"}
                </button>
              ))}
            </div>
          </div>
        </div>

        {aggregateQuery.isLoading && (
          <p className="text-xs text-muted-foreground">Loading profile data…</p>
        )}
        {aggregateQuery.error && (
          <p className="text-xs text-destructive">{aggregateQuery.error.message}</p>
        )}

        <CpuTimelineChart height={100} batches={batchesQuery.data?.batches} />
        <TopFunctionsTable stacks={stacks} totalSamples={totalSamples} />
      </div>
    </AppLayout>
  );
}
