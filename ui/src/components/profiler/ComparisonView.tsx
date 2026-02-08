import { cn } from "@/lib/utils";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import type { CpuDiffJson, StackDiff, Frame } from "@/api/types";
import { formatTimeRange } from "@/lib/format";

function frameLabel(f: Frame): string {
  return f.function ?? f.module ?? `0x${f.ip.toString(16)}`;
}

function stackLabel(s: StackDiff): string {
  const frames = s.stack.frames;
  if (frames.length === 0) return "—";
  return frames.map((f) => frameLabel(f)).join(" → ");
}

function DiffValue({ delta, deltaPct }: { delta: number; deltaPct: number }) {
  const isNeg = delta < 0;
  return (
    <div className="text-right">
      <div
        className={cn(
          "font-mono flex items-center justify-end gap-1",
          isNeg ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"
        )}
      >
        {isNeg ? <ArrowDownRight className="h-3 w-3" /> : <ArrowUpRight className="h-3 w-3" />}
        {deltaPct > 0 ? "+" : ""}
        {deltaPct.toFixed(1)}%
        <span className="text-muted-foreground text-[10px]">
          ({delta >= 0 ? "+" : ""}
          {delta})
        </span>
      </div>
    </div>
  );
}

interface ComparisonViewProps {
  /** Phase 8 API: diff result from /api/diff */
  diff: CpuDiffJson | null;
  /** Time ranges for display */
  baselineStartNs?: number;
  baselineEndNs?: number;
  comparisonStartNs?: number;
  comparisonEndNs?: number;
  /** Swap baseline ↔ comparison */
  onSwap?: () => void;
  loading?: boolean;
  error?: string | null;
}

export function ComparisonView({
  diff,
  baselineStartNs,
  baselineEndNs,
  comparisonStartNs,
  comparisonEndNs,
  onSwap,
  loading = false,
  error = null,
}: ComparisonViewProps) {
  if (error) {
    return (
      <div className="rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
        {error}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-md border border-border bg-card p-3">
          <div className="text-xs font-medium text-foreground mb-2">Baseline</div>
          <div className="flex flex-wrap gap-2">
            <span className="rounded bg-muted px-2 py-0.5 text-[10px] font-mono text-muted-foreground">
              CPU
            </span>
          </div>
          <div className="text-[10px] text-muted-foreground mt-2">
            {baselineStartNs != null && baselineEndNs != null
              ? formatTimeRange(baselineStartNs, baselineEndNs)
              : "Set time range (ns)"}
          </div>
        </div>
        <div className="rounded-md border border-border bg-card p-3 flex items-start justify-between">
          <div>
            <div className="text-xs font-medium text-foreground mb-2">Comparison</div>
            <div className="flex flex-wrap gap-2">
              <span className="rounded bg-muted px-2 py-0.5 text-[10px] font-mono text-muted-foreground">
                CPU
              </span>
            </div>
            <div className="text-[10px] text-muted-foreground mt-2">
              {comparisonStartNs != null && comparisonEndNs != null
                ? formatTimeRange(comparisonStartNs, comparisonEndNs)
                : "Set time range (ns)"}
            </div>
          </div>
          {onSwap && (
            <button
              type="button"
              onClick={onSwap}
              className="rounded p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
              title="Swap baseline and comparison"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M7 16V4M7 4L3 8M7 4L11 8" />
                <path d="M17 8v12M17 20l4-4M17 20l-4-4" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {loading && (
        <p className="text-xs text-muted-foreground">Running diff…</p>
      )}

      {diff && !loading && (
        <>
          <p className="text-xs text-muted-foreground">
            Baseline total: {diff.baseline_total.toLocaleString()} · Comparison total:{" "}
            {diff.comparison_total.toLocaleString()}
          </p>
          <div className="rounded-md border border-border overflow-hidden">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-border bg-muted/30">
                  <th className="text-left px-3 py-2 font-medium text-muted-foreground">Stack (top frame)</th>
                  <th className="text-right px-3 py-2 font-medium text-muted-foreground">Baseline</th>
                  <th className="text-right px-3 py-2 font-medium text-muted-foreground">Comparison</th>
                  <th className="text-right px-3 py-2 font-medium text-muted-foreground">Δ</th>
                  <th className="text-right px-3 py-2 font-medium text-muted-foreground">Δ %</th>
                </tr>
              </thead>
              <tbody>
                {diff.stacks.slice(0, 100).map((s, i) => (
                  <tr
                    key={i}
                    className={cn(
                      "border-b border-border/50 hover:bg-muted/20",
                      s.delta > 0 && "bg-red-500/5",
                      s.delta < 0 && "bg-green-500/5"
                    )}
                  >
                    <td
                      className="px-3 py-2.5 font-mono text-foreground max-w-md truncate"
                      title={stackLabel(s)}
                    >
                      {stackLabel(s)}
                    </td>
                    <td className="text-right px-3 py-2.5 font-mono">{s.baseline_count.toLocaleString()}</td>
                    <td className="text-right px-3 py-2.5 font-mono">{s.comparison_count.toLocaleString()}</td>
                    <td className="px-3 py-2.5">
                      <DiffValue delta={s.delta} deltaPct={s.delta_pct} />
                    </td>
                    <td className="text-right px-3 py-2.5 font-mono">
                      {s.delta_pct >= 0 ? "+" : ""}
                      {s.delta_pct.toFixed(1)}%
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {!diff && !loading && !error && (
        <p className="text-xs text-muted-foreground">Set baseline and comparison time ranges (ns) and run diff.</p>
      )}
    </div>
  );
}
