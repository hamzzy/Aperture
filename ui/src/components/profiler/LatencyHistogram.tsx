import { useMemo } from "react";
import { cn } from "@/lib/utils";

/** Power-of-2 latency buckets: bucket i covers [2^i ns, 2^(i+1) ns) */
const BUCKET_COUNT = 30;

function bucketLabel(i: number): string {
  const ns = 2 ** i;
  if (ns >= 1e9) return `${(ns / 1e9).toFixed(0)}s`;
  if (ns >= 1e6) return `${(ns / 1e6).toFixed(0)}ms`;
  if (ns >= 1e3) return `${(ns / 1e3).toFixed(0)}us`;
  return `${ns}ns`;
}

interface LatencyHistogramProps {
  /** 30-element array of counts per power-of-2 bucket */
  histogram: number[];
  className?: string;
  height?: number;
}

export function LatencyHistogram({
  histogram,
  className,
  height = 120,
}: LatencyHistogramProps) {
  const { bars, maxCount, firstNonZero, lastNonZero } = useMemo(() => {
    const h = histogram.length >= BUCKET_COUNT
      ? histogram.slice(0, BUCKET_COUNT)
      : [...histogram, ...Array(BUCKET_COUNT - histogram.length).fill(0)];
    let max = 0;
    let first = -1;
    let last = -1;
    for (let i = 0; i < h.length; i++) {
      if (h[i] > 0) {
        max = Math.max(max, h[i]);
        if (first === -1) first = i;
        last = i;
      }
    }
    // Show at least 1 bucket on each side for context
    const lo = Math.max(0, first - 1);
    const hi = Math.min(BUCKET_COUNT - 1, last + 1);
    return {
      bars: h.slice(lo, hi + 1).map((count, idx) => ({
        bucket: lo + idx,
        count,
        label: bucketLabel(lo + idx),
      })),
      maxCount: max,
      firstNonZero: first,
      lastNonZero: last,
    };
  }, [histogram]);

  if (firstNonZero === -1) {
    return (
      <div className={cn("text-xs text-muted-foreground text-center py-4", className)}>
        No latency data
      </div>
    );
  }

  const barWidth = bars.length > 0 ? Math.max(8, Math.min(24, 300 / bars.length)) : 16;
  // Show a subset of labels to avoid overlap
  const labelStep = Math.max(1, Math.floor(bars.length / 8));

  return (
    <div className={cn("flex flex-col", className)}>
      <div className="flex items-end gap-px" style={{ height }}>
        {bars.map((bar, i) => {
          const pct = maxCount > 0 ? (bar.count / maxCount) * 100 : 0;
          return (
            <div
              key={bar.bucket}
              className="group relative flex flex-col items-center"
              style={{ width: barWidth }}
            >
              {/* Tooltip on hover */}
              <div className="absolute bottom-full mb-1 hidden group-hover:block z-20 whitespace-nowrap rounded bg-popover border border-border px-2 py-1 text-[10px] text-popover-foreground shadow-md">
                {bar.label}: {bar.count.toLocaleString()}
              </div>
              <div
                className="w-full rounded-t-sm bg-primary/70 hover:bg-primary transition-colors"
                style={{
                  height: `${Math.max(pct > 0 ? 2 : 0, pct)}%`,
                  minHeight: pct > 0 ? 2 : 0,
                }}
              />
            </div>
          );
        })}
      </div>
      {/* X-axis labels */}
      <div className="flex gap-px mt-1">
        {bars.map((bar, i) => (
          <div
            key={bar.bucket}
            className="text-center text-[9px] text-muted-foreground"
            style={{ width: barWidth }}
          >
            {i % labelStep === 0 ? bar.label : ""}
          </div>
        ))}
      </div>
    </div>
  );
}
