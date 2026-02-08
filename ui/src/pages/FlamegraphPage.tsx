import { useState } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { FlamegraphViewer } from "@/components/profiler/FlamegraphViewer";
import { CpuTimelineChart } from "@/components/profiler/CpuTimelineChart";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Search, Settings, Maximize2 } from "lucide-react";
import { usePhase8 } from "@/contexts/Phase8Context";
import { nsFromPreset } from "@/components/layout/TopBar";
import { useAggregateQuery } from "@/api/queries";

export default function FlamegraphPage() {
  const phase8 = usePhase8();
  const { start, end } = phase8 ? nsFromPreset(phase8.timePreset) : { start: 0, end: 0 };
  const aggregateQuery = useAggregateQuery({
    time_start_ns: start,
    time_end_ns: end,
    limit: 500,
    event_type: "cpu",
    enabled: !!phase8,
  });
  const cpu = aggregateQuery.data?.cpu;
  const stacks = cpu?.stacks ?? [];
  const totalSamples = cpu?.total_samples ?? 0;
  const [searchRegex, setSearchRegex] = useState("");
  const error = aggregateQuery.error?.message ?? null;

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-lg font-semibold text-foreground">CPU Profiles</h1>
            <div className="flex items-center gap-1 text-xs">
              <span className="text-muted-foreground">CPU % Mode:</span>
              <Button variant="default" size="sm" className="h-6 px-2 text-[10px]">
                Abs
              </Button>
              <Button variant="ghost" size="sm" className="h-6 px-2 text-[10px] text-muted-foreground">
                Rel
              </Button>
            </div>
            <Button variant="ghost" size="sm" className="h-7 gap-1.5 text-xs text-muted-foreground">
              <Settings className="h-3.5 w-3.5" />
              Flamegraph Settings
            </Button>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" className="h-7 w-7 text-muted-foreground">
              <Maximize2 className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

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
