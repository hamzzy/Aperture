import { useMemo } from "react";
import { Link } from "react-router-dom";
import { formatNs } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { AggregateResultJson } from "@/api/types";

interface Insight {
  severity: "info" | "warn" | "critical";
  category: "cpu" | "lock" | "syscall" | "general";
  title: string;
  detail: string;
  link?: string;
}

function computeInsights(data: AggregateResultJson): Insight[] {
  const insights: Insight[] = [];
  const { cpu, lock, syscall } = data;

  // --- CPU insights ---
  if (cpu && cpu.stacks && cpu.stacks.length > 0) {
    const totalSamples = cpu.total_samples || 1;

    // Top CPU consumer
    const top = cpu.stacks[0]; // sorted by count desc from aggregator
    if (top) {
      const topFrame = top.stack.frames[0];
      const topName = topFrame?.function ?? `0x${topFrame?.ip.toString(16)}`;
      const pct = ((top.count / totalSamples) * 100).toFixed(1);
      insights.push({
        severity: Number(pct) > 30 ? "critical" : Number(pct) > 15 ? "warn" : "info",
        category: "cpu",
        title: `Top CPU hotspot: ${pct}%`,
        detail: `${topName} accounts for ${top.count.toLocaleString()} of ${totalSamples.toLocaleString()} samples`,
        link: "/flamegraph",
      });
    }

    // Stack diversity (many unique stacks = spread workload, few = concentrated)
    const uniqueRatio = cpu.stacks.length / Math.max(totalSamples, 1);
    if (uniqueRatio < 0.05 && totalSamples > 50) {
      insights.push({
        severity: "warn",
        category: "cpu",
        title: "Concentrated CPU usage",
        detail: `Only ${cpu.stacks.length} unique stacks across ${totalSamples.toLocaleString()} samples — workload is concentrated in a few code paths`,
        link: "/functions",
      });
    }

    // Unresolved frames
    let totalFrames = 0;
    let unresolvedFrames = 0;
    for (const { stack } of cpu.stacks) {
      for (const f of stack.frames) {
        totalFrames++;
        const label = f.function ?? `0x${f.ip.toString(16)}`;
        if (/^0x[0-9a-f]+$/i.test(label)) unresolvedFrames++;
      }
    }
    if (totalFrames > 0) {
      const unresolvedPct = (unresolvedFrames / totalFrames) * 100;
      if (unresolvedPct > 30) {
        insights.push({
          severity: "warn",
          category: "general",
          title: `${unresolvedPct.toFixed(0)}% unresolved symbols`,
          detail: `${unresolvedFrames}/${totalFrames} frames are raw hex addresses. Install debug symbols and ensure kptr_restrict=0`,
        });
      }
    }
  }

  // --- Lock insights ---
  if (lock && lock.contentions && lock.contentions.length > 0) {
    // Worst contention by total wait time
    const sorted = [...lock.contentions].sort(
      (a, b) => b.total_wait_ns - a.total_wait_ns,
    );
    const worst = sorted[0];
    if (worst) {
      const topFrame = worst.stack.frames[0];
      const lockName = topFrame?.function ?? `0x${worst.lock_addr.toString(16)}`;
      insights.push({
        severity: worst.total_wait_ns > 1e9 ? "critical" : worst.total_wait_ns > 1e6 ? "warn" : "info",
        category: "lock",
        title: `Worst lock contention: ${formatNs(worst.total_wait_ns)} total wait`,
        detail: `${lockName} — ${worst.count.toLocaleString()} events, max single wait ${formatNs(worst.max_wait_ns)}`,
        link: "/flamegraph",
      });
    }

    // Total lock wait time
    const totalLockWait = lock.contentions.reduce((s, c) => s + c.total_wait_ns, 0);
    if (totalLockWait > 1e9) {
      insights.push({
        severity: "critical",
        category: "lock",
        title: `${formatNs(totalLockWait)} total lock wait time`,
        detail: `Across ${lock.contentions.length} contention sites and ${lock.total_events.toLocaleString()} events`,
      });
    }
  }

  // --- Syscall insights ---
  if (syscall && syscall.syscalls) {
    const syscalls = Object.values(syscall.syscalls);
    if (syscalls.length > 0) {
      // Slowest avg syscall
      const byAvg = [...syscalls]
        .filter((s) => s.count > 0)
        .sort((a, b) => b.total_duration_ns / b.count - a.total_duration_ns / a.count);
      if (byAvg.length > 0) {
        const slowest = byAvg[0];
        const avgNs = slowest.total_duration_ns / slowest.count;
        insights.push({
          severity: avgNs > 1e6 ? "critical" : avgNs > 1e4 ? "warn" : "info",
          category: "syscall",
          title: `Slowest syscall: ${slowest.name} avg ${formatNs(avgNs)}`,
          detail: `${slowest.count.toLocaleString()} calls, max ${formatNs(slowest.max_duration_ns)}`,
          link: "/syscalls",
        });
      }

      // Most frequent syscall
      const byCount = [...syscalls].sort((a, b) => b.count - a.count);
      if (byCount.length > 0) {
        const most = byCount[0];
        const pct = ((most.count / Math.max(syscall.total_events, 1)) * 100).toFixed(1);
        insights.push({
          severity: "info",
          category: "syscall",
          title: `Most frequent: ${most.name} (${pct}%)`,
          detail: `${most.count.toLocaleString()} of ${syscall.total_events.toLocaleString()} total syscalls`,
          link: "/syscalls",
        });
      }

      // Syscall errors
      const totalErrors = syscalls.reduce((s, sc) => s + sc.error_count, 0);
      if (totalErrors > 0) {
        const errorSyscalls = syscalls.filter((s) => s.error_count > 0).sort((a, b) => b.error_count - a.error_count);
        const worst = errorSyscalls[0];
        insights.push({
          severity: totalErrors > 100 ? "critical" : "warn",
          category: "syscall",
          title: `${totalErrors.toLocaleString()} syscall errors`,
          detail: `Worst: ${worst.name} with ${worst.error_count.toLocaleString()} errors out of ${worst.count.toLocaleString()} calls`,
          link: "/syscalls",
        });
      }
    }
  }

  // Sort: critical first, then warn, then info
  const order = { critical: 0, warn: 1, info: 2 };
  insights.sort((a, b) => order[a.severity] - order[b.severity]);

  return insights;
}

const severityStyles: Record<string, { border: string; bg: string; dot: string }> = {
  critical: {
    border: "border-destructive/40",
    bg: "bg-destructive/5",
    dot: "bg-destructive",
  },
  warn: {
    border: "border-amber-500/40",
    bg: "bg-amber-500/5",
    dot: "bg-amber-500",
  },
  info: {
    border: "border-primary/30",
    bg: "bg-primary/5",
    dot: "bg-primary/60",
  },
};

const categoryLabels: Record<string, string> = {
  cpu: "CPU",
  lock: "Lock",
  syscall: "Syscall",
  general: "General",
};

interface ProfileInsightsProps {
  data: AggregateResultJson | null;
}

export function ProfileInsights({ data }: ProfileInsightsProps) {
  const insights = useMemo(() => {
    if (!data) return [];
    return computeInsights(data);
  }, [data]);

  if (insights.length === 0) return null;

  return (
    <div className="rounded-md border border-border bg-card p-4">
      <h2 className="text-sm font-medium text-foreground mb-3">
        Insights
        <span className="ml-2 text-[10px] text-muted-foreground font-normal">
          {insights.length} findings
        </span>
      </h2>
      <div className="space-y-2">
        {insights.map((insight, i) => {
          const style = severityStyles[insight.severity];
          const content = (
            <div
              key={i}
              className={cn(
                "rounded-md border px-3 py-2 text-xs transition-colors",
                style.border,
                style.bg,
                insight.link && "hover:brightness-95 cursor-pointer",
              )}
            >
              <div className="flex items-start gap-2">
                <span className={cn("h-2 w-2 rounded-full mt-1 shrink-0", style.dot)} />
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-foreground">{insight.title}</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                      {categoryLabels[insight.category]}
                    </span>
                  </div>
                  <div className="text-muted-foreground mt-0.5">{insight.detail}</div>
                </div>
              </div>
            </div>
          );
          if (insight.link) {
            return (
              <Link key={i} to={insight.link} className="block">
                {content}
              </Link>
            );
          }
          return content;
        })}
      </div>
    </div>
  );
}
