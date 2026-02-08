import { AppLayout } from "@/components/layout/AppLayout";
import { TopFunctionsTable } from "@/components/profiler/TopFunctionsTable";
import { CpuTimelineChart } from "@/components/profiler/CpuTimelineChart";
import { Button } from "@/components/ui/button";
import { usePhase8 } from "@/contexts/Phase8Context";
import { nsFromPreset } from "@/components/layout/TopBar";
import { useAggregateQuery } from "@/api/queries";

export default function TopFunctionsPage() {
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

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-lg font-semibold text-foreground">Top Functions</h1>
            <div className="flex items-center gap-1 text-xs">
              <span className="text-muted-foreground">CPU % Mode:</span>
              <span className="text-muted-foreground">Abs</span>
              <Button variant="default" size="sm" className="h-6 px-2 text-[10px]">
                Rel
              </Button>
            </div>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" className="h-7 text-xs border-border">
              CPU
            </Button>
            <Button variant="ghost" size="sm" className="h-7 text-xs text-muted-foreground">
              GPU
            </Button>
          </div>
        </div>

        <CpuTimelineChart height={100} />
        <TopFunctionsTable stacks={stacks} totalSamples={totalSamples} />
      </div>
    </AppLayout>
  );
}
