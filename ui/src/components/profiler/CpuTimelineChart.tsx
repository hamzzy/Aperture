import { Area, AreaChart, ResponsiveContainer, XAxis, YAxis, Tooltip } from "recharts";

const generateTimelineData = () => {
  const data = [];
  const now = Date.now();
  for (let i = 60; i >= 0; i--) {
    const time = new Date(now - i * 15000);
    data.push({
      time: time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      used: Math.random() * 40 + 20,
      idle: Math.random() * 20 + 5,
    });
  }
  return data;
};

const data = generateTimelineData();

interface CpuTimelineChartProps {
  height?: number;
  title?: string;
}

export function CpuTimelineChart({ height = 120, title = "CPU Core Usage" }: CpuTimelineChartProps) {
  return (
    <div className="rounded-md border border-border bg-card p-3">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-foreground">{title}</span>
        <span className="text-[10px] text-muted-foreground">Avg</span>
      </div>
      <ResponsiveContainer width="100%" height={height}>
        <AreaChart data={data} margin={{ top: 0, right: 0, left: -20, bottom: 0 }}>
          <defs>
            <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="hsl(170, 80%, 45%)" stopOpacity={0.3} />
              <stop offset="100%" stopColor="hsl(170, 80%, 45%)" stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <XAxis
            dataKey="time"
            tick={{ fontSize: 10, fill: 'hsl(215, 15%, 55%)' }}
            axisLine={{ stroke: 'hsl(220, 14%, 18%)' }}
            tickLine={false}
            interval="preserveStartEnd"
          />
          <YAxis
            tick={{ fontSize: 10, fill: 'hsl(215, 15%, 55%)' }}
            axisLine={false}
            tickLine={false}
            tickFormatter={(v) => `${v.toFixed(0)} mc`}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(220, 16%, 12%)',
              border: '1px solid hsl(220, 14%, 18%)',
              borderRadius: '6px',
              fontSize: '11px',
              color: 'hsl(210, 20%, 90%)',
            }}
          />
          <Area
            type="monotone"
            dataKey="used"
            stroke="hsl(170, 80%, 45%)"
            strokeWidth={1.5}
            fill="url(#cpuGrad)"
          />
          <Area
            type="monotone"
            dataKey="idle"
            stroke="hsl(220, 14%, 35%)"
            strokeWidth={1}
            fill="hsl(220, 14%, 12%)"
            fillOpacity={0.5}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
