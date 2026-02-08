import { useMemo, useState } from "react";
import { Search, Columns, Maximize2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { StackCount, Frame } from "@/api/types";

function frameLabel(f: Frame): string {
  return f.function ?? f.module ?? `0x${f.ip.toString(16)}`;
}

export interface FunctionRow {
  name: string;
  module: string;
  coresSelf: number;
  coresTotal: number;
  selfCpu: string;
  totalCpu: string;
  type: "native" | "python" | "kernel" | "cuda";
}

const MOCK_FUNCTIONS: FunctionRow[] = [
  {
    name: "_PyEval_EvalFrameDefault",
    module: "libpython3.11",
    coresSelf: 0.33,
    coresTotal: 217.83,
    selfCpu: "0.15%",
    totalCpu: "99.96%",
    type: "python",
  },
  {
    name: "worker",
    module: "app/main.py",
    coresSelf: 2.17,
    coresTotal: 183.42,
    selfCpu: "0.99%",
    totalCpu: "84.17%",
    type: "python",
  },
  {
    name: "cuda_kernel_launch",
    module: "libcuda.so",
    coresSelf: 8.5,
    coresTotal: 174.92,
    selfCpu: "3.90%",
    totalCpu: "80.27%",
    type: "cuda",
  },
];

const typeColor: Record<string, string> = {
  native: "bg-orange-600",
  python: "bg-sky-600",
  kernel: "bg-emerald-600",
  cuda: "bg-violet-600",
};

function inferType(module: string): FunctionRow["type"] {
  if (module.includes("python") || module.endsWith(".py")) return "python";
  if (module.includes("cuda") || module.includes("libcuda")) return "cuda";
  if (module.includes("libc") || module.includes("sysdeps")) return "kernel";
  return "native";
}

/** Build function rows from Phase 8 API stacks (top frame = function, count = samples) */
export function rowsFromStacks(stacks: StackCount[], totalSamples: number): FunctionRow[] {
  const byLabel = new Map<string, { count: number; module: string }>();
  for (const { stack, count } of stacks) {
    const frames = stack.frames;
    if (frames.length === 0) continue;
    const top = frames[0];
    const name = frameLabel(top);
    const module = top.module ?? top.file ?? "";
    const prev = byLabel.get(name);
    if (prev) prev.count += count;
    else byLabel.set(name, { count, module });
  }
  const total = totalSamples || 1;
  return [...byLabel.entries()]
    .map(([name, { count, module }]) => ({
      name,
      module,
      coresSelf: count,
      coresTotal: count,
      selfCpu: `${((count / total) * 100).toFixed(2)}%`,
      totalCpu: `${((count / total) * 100).toFixed(2)}%`,
      type: inferType(module),
    }))
    .sort((a, b) => b.coresTotal - a.coresTotal);
}

interface TopFunctionsTableProps {
  /** Phase 8 API: when provided, use real data instead of mock */
  stacks?: StackCount[];
  totalSamples?: number;
}

export function TopFunctionsTable({ stacks, totalSamples = 0 }: TopFunctionsTableProps) {
  const [search, setSearch] = useState("");
  const [sortKey, setSortKey] = useState<"coresTotal" | "coresSelf">("coresTotal");

  const rows = useMemo(() => {
    if (stacks && stacks.length > 0) {
      return rowsFromStacks(stacks, totalSamples);
    }
    return MOCK_FUNCTIONS;
  }, [stacks, totalSamples]);

  const filtered = useMemo(
    () =>
      rows
        .filter((f) => f.name.toLowerCase().includes(search.toLowerCase()))
        .sort((a, b) => b[sortKey] - a[sortKey]),
    [rows, search, sortKey]
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <div className="relative flex-1 max-w-xs">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search functions..."
            className="h-8 pl-8 text-xs bg-background border-border"
          />
        </div>
        <Button variant="ghost" size="sm" className="h-8 gap-1.5 text-xs text-muted-foreground">
          <Columns className="h-3.5 w-3.5" />
          Columns
        </Button>
        <Button variant="ghost" size="icon" className="h-8 w-8 text-muted-foreground">
          <Maximize2 className="h-3.5 w-3.5" />
        </Button>
      </div>

      <div className="rounded-md border border-border overflow-hidden">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-border bg-muted/30">
              <th className="text-left px-3 py-2 font-medium text-muted-foreground">Function</th>
              <th
                className={cn(
                  "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground",
                  sortKey === "coresSelf" ? "text-foreground" : "text-muted-foreground"
                )}
                onClick={() => setSortKey("coresSelf")}
              >
                Samples (self) ↕
              </th>
              <th
                className={cn(
                  "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground",
                  sortKey === "coresTotal" ? "text-foreground" : "text-muted-foreground"
                )}
                onClick={() => setSortKey("coresTotal")}
              >
                Samples (total) ↕
              </th>
              <th className="text-right px-3 py-2 font-medium text-muted-foreground">Self CPU</th>
              <th className="text-right px-3 py-2 font-medium text-muted-foreground">Total CPU</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((fn, i) => (
              <tr
                key={i}
                className="border-b border-border/50 hover:bg-muted/20 cursor-pointer transition-colors"
              >
                <td className="px-3 py-2">
                  <div className="flex items-center gap-2">
                    <div className={cn("h-3 w-3 rounded-sm shrink-0", typeColor[fn.type])} />
                    <div>
                      <span className="font-mono text-foreground">{fn.name}</span>
                      <span className="block text-[10px] text-muted-foreground">{fn.module || "—"}</span>
                    </div>
                  </div>
                </td>
                <td className="text-right px-3 py-2 font-mono text-foreground">
                  {fn.coresSelf.toLocaleString()}
                </td>
                <td className="text-right px-3 py-2 font-mono text-foreground">
                  {fn.coresTotal.toLocaleString()}
                </td>
                <td className="text-right px-3 py-2 font-mono text-muted-foreground">{fn.selfCpu}</td>
                <td className="text-right px-3 py-2">
                  <div className="flex items-center justify-end gap-2">
                    <div className="w-16 h-1.5 rounded-full bg-muted overflow-hidden">
                      <div
                        className="h-full rounded-full bg-primary"
                        style={{
                          width: fn.totalCpu.replace("%", "").trim()
                            ? `${Math.min(100, parseFloat(fn.totalCpu))}%`
                            : "0%",
                        }}
                      />
                    </div>
                    <span className="font-mono text-foreground w-14 text-right">{fn.totalCpu}</span>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
