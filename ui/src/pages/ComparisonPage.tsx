import { useState, useCallback } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { ComparisonView } from "@/components/profiler/ComparisonView";
import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useDashboard } from "@/contexts/DashboardContext";
import { nsFromPreset } from "@/components/layout/TopBar";
import { useDiffMutation } from "@/api/queries";

export default function ComparisonPage() {
  const dashboard = useDashboard();
  const [baselineStart, setBaselineStart] = useState("");
  const [baselineEnd, setBaselineEnd] = useState("");
  const [comparisonStart, setComparisonStart] = useState("");
  const [comparisonEnd, setComparisonEnd] = useState("");
  const [search, setSearch] = useState("");
  const diffMutation = useDiffMutation();
  const diff = diffMutation.data ?? null;
  const loading = diffMutation.isPending;
  const error = diffMutation.error?.message ?? null;

  const runDiff = useCallback(() => {
    const { start: defStart, end: defEnd } = nsFromPreset(dashboard?.timePreset ?? "1h");
    diffMutation.mutate({
      baseline_start_ns: baselineStart ? Number(baselineStart) : defStart,
      baseline_end_ns: baselineEnd ? Number(baselineEnd) : defEnd,
      comparison_start_ns: comparisonStart ? Number(comparisonStart) : defStart,
      comparison_end_ns: comparisonEnd ? Number(comparisonEnd) : defEnd,
      event_type: "cpu",
      limit: 500,
    });
  }, [baselineStart, baselineEnd, comparisonStart, comparisonEnd, dashboard?.timePreset, diffMutation]);

  const swap = useCallback(() => {
    setBaselineStart((p) => comparisonStart);
    setBaselineEnd((p) => comparisonEnd);
    setComparisonStart((p) => baselineStart);
    setComparisonEnd((p) => baselineEnd);
  }, [baselineStart, baselineEnd, comparisonStart, comparisonEnd]);

  const baselineStartNs = baselineStart ? Number(baselineStart) : undefined;
  const baselineEndNs = baselineEnd ? Number(baselineEnd) : undefined;
  const comparisonStartNs = comparisonStart ? Number(comparisonStart) : undefined;
  const comparisonEndNs = comparisonEnd ? Number(comparisonEnd) : undefined;

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold text-foreground">Differential Functions</h1>
          <span className="text-xs text-muted-foreground">Differential Analysis</span>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <label className="text-xs font-medium text-foreground">Baseline (ns)</label>
            <div className="flex gap-2">
              <Input
                placeholder="Start ns"
                value={baselineStart}
                onChange={(e) => setBaselineStart(e.target.value)}
                className="h-8 text-xs font-mono"
              />
              <Input
                placeholder="End ns"
                value={baselineEnd}
                onChange={(e) => setBaselineEnd(e.target.value)}
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
          <div className="space-y-2">
            <label className="text-xs font-medium text-foreground">Comparison (ns)</label>
            <div className="flex gap-2">
              <Input
                placeholder="Start ns"
                value={comparisonStart}
                onChange={(e) => setComparisonStart(e.target.value)}
                className="h-8 text-xs font-mono"
              />
              <Input
                placeholder="End ns"
                value={comparisonEnd}
                onChange={(e) => setComparisonEnd(e.target.value)}
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
        </div>
        <Button onClick={runDiff} disabled={loading} size="sm" className="h-8">
          {loading ? "Running…" : "Run diff"}
        </Button>

        <div className="relative max-w-xs">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            placeholder="Search"
            className="h-8 pl-8 text-xs bg-background border-border"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        <ComparisonView
          diff={diff}
          baselineStartNs={baselineStartNs}
          baselineEndNs={baselineEndNs}
          comparisonStartNs={comparisonStartNs}
          comparisonEndNs={comparisonEndNs}
          onSwap={swap}
          loading={loading}
          error={error}
        />
      </div>
    </AppLayout>
  );
}
