import { useState, useMemo } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { FlamegraphViewer } from "@/components/profiler/FlamegraphViewer";
import { CpuTimelineChart } from "@/components/profiler/CpuTimelineChart";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Search, Maximize2 } from "lucide-react";
import { usePhase8 } from "@/contexts/Phase8Context";
import { useAggregateQuery } from "@/api/queries";
import type { StackCount } from "@/api/types";

export default function FlamegraphPage() {
  const phase8 = usePhase8();
  const { start, end } = phase8?.timeRange ?? { start: 0, end: 0 };
  const [eventType, setEventType] = useState<"cpu" | "lock" | "">("");
  const aggregateQuery = useAggregateQuery({
    time_start_ns: start,
    time_end_ns: end,
    limit: 500,
    event_type: eventType || undefined,
    enabled: !!phase8,
  });
  const data = aggregateQuery.data;

  // Build stacks from whichever profile type is available
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
      // Convert lock contentions to StackCount[] for the flamegraph
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
    // Fallback: show CPU stacks even if empty
    return {
      stacks: cpu?.stacks ?? [],
      totalSamples: cpu?.total_samples ?? 0,
      activeType: "cpu" as const,
    };
  }, [data, eventType]);

  const [searchRegex, setSearchRegex] = useState("");
  const error = aggregateQuery.error?.message ?? null;

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-lg font-semibold text-foreground">
              {activeType === "lock" ? "Lock Contention" : "CPU"} Flamegraph
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
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground">
              <Maximize2 className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {aggregateQuery.isLoading && (
          <p className="text-xs text-muted-foreground">Loading profile data…</p>
        )}
        {error && (
          <p className="text-xs text-destructive">
            {error}
            {!error.includes("port") && !error.includes("storage") && (
              <> — Ensure aggregator is running with storage (e.g. port 9090).</>
            )}
          </p>
        )}

        <CpuTimelineChart height={100} />

        <div className="relative max-w-md">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            placeholder="Search (regex) - e.g. con.*call"
            className="h-8 pl-8 text-xs bg-background border-border"
            value={searchRegex}
            onChange={(e) => setSearchRegex(e.target.value)}
          />
        </div>

        <FlamegraphViewer
          stacks={stacks}
          totalSamples={totalSamples}
          searchRegex={searchRegex || undefined}
          height={500}
        />
      </div>
    </AppLayout>
  );
}
