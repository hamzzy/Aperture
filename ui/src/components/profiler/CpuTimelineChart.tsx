import { useMemo } from "react";
import { Area, AreaChart, ResponsiveContainer, XAxis, YAxis, Tooltip } from "recharts";
import type { BatchInfo } from "@/api/types";

interface CpuTimelineChartProps {
  height?: number;
  title?: string;
  /** Real batch data from /api/batches. When provided, replaces placeholder. */
  batches?: BatchInfo[];
}

/** Bucket batches into time intervals and sum event counts. */
function bucketBatches(batches: BatchInfo[], bucketCount: number) {
  if (batches.length === 0) return [];

  const times = batches.map((b) => b.received_at_ns / 1e6); // ns -> ms
  const minTime = Math.min(...times);
  const maxTime = Math.max(...times);
  const range = maxTime - minTime;
  if (range <= 0) {
    const total = batches.reduce((s, b) => s + b.event_count, 0);
    return [
      {
        time: new Date(minTime).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        events: total,
        batches: batches.length,
      },
    ];
  }

  const bucketSize = range / bucketCount;
  const result: { time: string; events: number; batches: number }[] = [];

  for (let i = 0; i < bucketCount; i++) {
    const bucketStart = minTime + i * bucketSize;
    const bucketEnd = bucketStart + bucketSize;
    let events = 0;
    let count = 0;
    for (let j = 0; j < batches.length; j++) {
      const t = times[j];
      if (t >= bucketStart && (i === bucketCount - 1 ? t <= bucketEnd : t < bucketEnd)) {
        events += batches[j].event_count;
        count++;
      }
    }
    result.push({
      time: new Date(bucketStart).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      }),
      events,
      batches: count,
    });
  }

  return result;
}

export function CpuTimelineChart({
  height = 120,
  title = "Event Throughput",
  batches,
}: CpuTimelineChartProps) {
  const data = useMemo(() => {
    if (!batches || batches.length === 0) return [];
    return bucketBatches(batches, 30);
  }, [batches]);

  if (data.length === 0) {
    return (
      <div className="rounded-md border border-border bg-card p-3">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-foreground">{title}</span>
        </div>
        <div
          className="flex items-center justify-center text-xs text-muted-foreground"
          style={{ height }}
        >
          No batch data available
        </div>
      </div>
    );
  }

  const totalEvents = data.reduce((s, d) => s + d.events, 0);

  return (
    <div className="rounded-md border border-border bg-card p-3">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-foreground">{title}</span>
        <span className="text-[10px] text-muted-foreground">
          {totalEvents.toLocaleString()} events
        </span>
      </div>
      <ResponsiveContainer width="100%" height={height}>
        <AreaChart data={data} margin={{ top: 0, right: 0, left: -20, bottom: 0 }}>
          <defs>
            <linearGradient id="eventsGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="hsl(170, 80%, 45%)" stopOpacity={0.3} />
              <stop offset="100%" stopColor="hsl(170, 80%, 45%)" stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="time"
            tick={{ fontSize: 10, fill: "hsl(215, 15%, 55%)" }}
            axisLine={{ stroke: "hsl(220, 14%, 18%)" }}
            tickLine={false}
            interval="preserveStartEnd"
          />
          <YAxis
            tick={{ fontSize: 10, fill: "hsl(215, 15%, 55%)" }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v: number) => v.toLocaleString()}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: "hsl(220, 16%, 12%)",
              border: "1px solid hsl(220, 14%, 18%)",
              borderRadius: "6px",
              fontSize: "11px",
              color: "hsl(210, 20%, 90%)",
            }}
            formatter={(value: number, name: string) => [
              value.toLocaleString(),
              name === "events" ? "Events" : "Batches",
            ]}
          />
          <Area
            type="monotone"
            dataKey="events"
            stroke="hsl(170, 80%, 45%)"
            strokeWidth={1.5}
            fill="url(#eventsGrad)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
