import { AppLayout } from "@/components/layout/AppLayout";
import { CpuTimelineChart } from "@/components/profiler/CpuTimelineChart";

export default function TimelinePage() {
  return (
    <AppLayout>
      <div className="space-y-4">
        <h1 className="text-lg font-semibold text-foreground">Timeline</h1>

        <CpuTimelineChart title="CPU Core Usage" height={180} />
        <CpuTimelineChart title="GPU Kernel Execution" height={180} />
        <CpuTimelineChart title="Memory Transfers (DtoH / HtoD)" height={140} />

        <div className="rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-medium text-foreground mb-3">Synchronization Points</h2>
          <div className="h-24 flex items-center justify-center">
            <div className="relative w-full h-8 bg-muted/30 rounded overflow-hidden">
              {[15, 28, 35, 52, 61, 73, 88].map((pos, i) => (
                <div
                  key={i}
                  className="absolute top-0 h-full w-px bg-warning/60"
                  style={{ left: `${pos}%` }}
                >
                  <div className="absolute -top-1 left-1/2 -translate-x-1/2 h-2 w-2 rounded-full bg-warning" />
                </div>
              ))}
              <div className="absolute inset-0 bg-gradient-to-r from-primary/20 via-purple-500/20 to-primary/10" />
            </div>
          </div>
          <div className="flex justify-between text-[10px] text-muted-foreground mt-1">
            <span>00:00:00</span>
            <span>00:05:00</span>
            <span>00:10:00</span>
          </div>
        </div>
      </div>
    </AppLayout>
  );
}
