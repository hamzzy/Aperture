import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { StackCount, Frame } from "@/api/types";

function frameLabel(f: Frame): string {
  return f.function ?? f.module ?? `0x${f.ip.toString(16)}`;
}

export interface FunctionRow {
  name: string;
  module: string;
  self: number;
  total: number;
  selfPct: number;
  totalPct: number;
  type: "native" | "python" | "kernel" | "cuda";
}

const typeColor: Record<string, string> = {
  native: "bg-orange-600",
  python: "bg-sky-600",
  kernel: "bg-emerald-600",
  cuda: "bg-violet-600",
};

function inferType(name: string, module: string): FunctionRow["type"] {
  if (module.includes("python") || module.endsWith(".py")) return "python";
  if (module.includes("cuda") || module.includes("libcuda")) return "cuda";
  if (
    name.startsWith("__") ||
    name.startsWith("do_") ||
    name.startsWith("sys_") ||
    name.startsWith("entry_") ||
    module.includes("vmlinux") ||
    module.includes("[kernel")
  ) {
    return "kernel";
  }
  return "native";
}

/**
 * Build function rows with proper self/total metrics.
 * - self: count of stacks where this function is the top (leaf) frame
 * - total: count of stacks where this function appears anywhere
 */
export function rowsFromStacks(
  stacks: StackCount[],
  totalSamples: number,
): FunctionRow[] {
  const selfMap = new Map<string, number>();
  const totalMap = new Map<string, number>();
  const moduleMap = new Map<string, string>();

  for (const { stack, count } of stacks) {
    const frames = stack.frames;
    if (frames.length === 0) continue;

    // Self: top frame only (index 0 = leaf)
    const topLabel = frameLabel(frames[0]);
    selfMap.set(topLabel, (selfMap.get(topLabel) ?? 0) + count);
    if (!moduleMap.has(topLabel)) {
      moduleMap.set(topLabel, frames[0].module ?? frames[0].file ?? "");
    }

    // Total: every unique function in the stack
    const seen = new Set<string>();
    for (const f of frames) {
      const label = frameLabel(f);
      if (seen.has(label)) continue;
      seen.add(label);
      totalMap.set(label, (totalMap.get(label) ?? 0) + count);
      if (!moduleMap.has(label)) {
        moduleMap.set(label, f.module ?? f.file ?? "");
      }
    }
  }

  const total = totalSamples || 1;
  const rows: FunctionRow[] = [];

  // Union of all function names from both maps
  const allNames = new Set([...selfMap.keys(), ...totalMap.keys()]);
  for (const name of allNames) {
    const selfCount = selfMap.get(name) ?? 0;
    const totalCount = totalMap.get(name) ?? 0;
    const mod = moduleMap.get(name) ?? "";
    rows.push({
      name,
      module: mod,
      self: selfCount,
      total: totalCount,
      selfPct: (selfCount / total) * 100,
      totalPct: (totalCount / total) * 100,
      type: inferType(name, mod),
    });
  }

  return rows.sort((a, b) => b.self - a.self);
}

type SortKey = "self" | "total" | "selfPct" | "totalPct";

interface TopFunctionsTableProps {
  stacks?: StackCount[];
  totalSamples?: number;
}

export function TopFunctionsTable({
  stacks,
  totalSamples = 0,
}: TopFunctionsTableProps) {
  const [search, setSearch] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("self");
  const [sortAsc, setSortAsc] = useState(false);

  const rows = useMemo(() => {
    if (stacks && stacks.length > 0) {
      return rowsFromStacks(stacks, totalSamples);
    }
    return [];
  }, [stacks, totalSamples]);

  const filtered = useMemo(() => {
    let result = rows;
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(
        (f) =>
          f.name.toLowerCase().includes(q) ||
          f.module.toLowerCase().includes(q),
      );
    }
    return result.sort((a, b) => {
      const diff = a[sortKey] - b[sortKey];
      return sortAsc ? diff : -diff;
    });
  }, [rows, search, sortKey, sortAsc]);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(false);
    }
  };

  const sortIndicator = (key: SortKey) => {
    if (sortKey !== key) return " ↕";
    return sortAsc ? " ↑" : " ↓";
  };

  if (!stacks || stacks.length === 0) {
    return (
      <div className="rounded-md border border-border bg-card p-8 text-center text-sm text-muted-foreground">
        No function data available.
      </div>
    );
  }

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
        <span className="text-xs text-muted-foreground">
          {filtered.length} functions
        </span>
      </div>

      <div className="rounded-md border border-border overflow-hidden">
        <div className="overflow-auto" style={{ maxHeight: 500 }}>
          <table className="w-full text-xs">
            <thead className="sticky top-0 z-10">
              <tr className="border-b border-border bg-muted/50">
                <th className="text-left px-3 py-2 font-medium text-muted-foreground w-[45%]">
                  Function
                </th>
                <th
                  className={cn(
                    "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                    sortKey === "self"
                      ? "text-foreground"
                      : "text-muted-foreground",
                  )}
                  onClick={() => handleSort("self")}
                >
                  Self{sortIndicator("self")}
                </th>
                <th
                  className={cn(
                    "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                    sortKey === "selfPct"
                      ? "text-foreground"
                      : "text-muted-foreground",
                  )}
                  onClick={() => handleSort("selfPct")}
                >
                  Self %{sortIndicator("selfPct")}
                </th>
                <th
                  className={cn(
                    "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                    sortKey === "total"
                      ? "text-foreground"
                      : "text-muted-foreground",
                  )}
                  onClick={() => handleSort("total")}
                >
                  Total{sortIndicator("total")}
                </th>
                <th
                  className={cn(
                    "text-right px-3 py-2 font-medium cursor-pointer hover:text-foreground whitespace-nowrap",
                    sortKey === "totalPct"
                      ? "text-foreground"
                      : "text-muted-foreground",
                  )}
                  onClick={() => handleSort("totalPct")}
                >
                  Total %{sortIndicator("totalPct")}
                </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((fn, i) => (
                <tr
                  key={`${fn.name}-${i}`}
                  className="border-b border-border/50 hover:bg-muted/20 transition-colors"
                >
                  <td className="px-3 py-1.5">
                    <div className="flex items-center gap-2 min-w-0">
                      <div
                        className={cn(
                          "h-2.5 w-2.5 rounded-sm shrink-0",
                          typeColor[fn.type],
                        )}
                      />
                      <div className="min-w-0">
                        <span className="font-mono text-foreground block truncate">
                          {fn.name}
                        </span>
                        {fn.module && (
                          <span className="block text-[10px] text-muted-foreground truncate">
                            {fn.module}
                          </span>
                        )}
                      </div>
                    </div>
                  </td>
                  <td className="text-right px-3 py-1.5 font-mono text-foreground">
                    {fn.self.toLocaleString()}
                  </td>
                  <td className="text-right px-3 py-1.5">
                    <div className="flex items-center justify-end gap-2">
                      <div className="w-12 h-1.5 rounded-full bg-muted overflow-hidden">
                        <div
                          className="h-full rounded-full bg-orange-500"
                          style={{
                            width: `${Math.min(100, fn.selfPct)}%`,
                          }}
                        />
                      </div>
                      <span className="font-mono text-muted-foreground w-14 text-right">
                        {fn.selfPct.toFixed(1)}%
                      </span>
                    </div>
                  </td>
                  <td className="text-right px-3 py-1.5 font-mono text-foreground">
                    {fn.total.toLocaleString()}
                  </td>
                  <td className="text-right px-3 py-1.5">
                    <div className="flex items-center justify-end gap-2">
                      <div className="w-12 h-1.5 rounded-full bg-muted overflow-hidden">
                        <div
                          className="h-full rounded-full bg-primary"
                          style={{
                            width: `${Math.min(100, fn.totalPct)}%`,
                          }}
                        />
                      </div>
                      <span className="font-mono text-muted-foreground w-14 text-right">
                        {fn.totalPct.toFixed(1)}%
                      </span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
