import { useMemo, useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import type { StackCount, Frame } from "@/api/types";

function frameLabel(f: Frame): string {
  return f.function ?? f.module ?? `0x${f.ip.toString(16)}`;
}

interface Node {
  label: string;
  count: number;
  children: Map<string, Node>;
  stack: Frame[];
}

function buildTree(stacks: StackCount[]): Node {
  const root: Node = { label: "(root)", count: 0, children: new Map(), stack: [] };
  for (const { stack, count } of stacks) {
    const frames = stack.frames;
    if (frames.length === 0) continue;
    let current = root;
    current.count += count;
    for (let i = frames.length - 1; i >= 0; i--) {
      const f = frames[i];
      const key = `${f.ip}-${f.function ?? ""}`;
      let child = current.children.get(key);
      if (!child) {
        child = { label: frameLabel(f), count: 0, children: new Map(), stack: frames.slice(i) };
        current.children.set(key, child);
      }
      child.count += count;
      current = child;
    }
  }
  return root;
}

interface Block {
  x: number;
  width: number;
  depth: number;
  label: string;
  count: number;
  pct: number;
  node: Node;
}

function layout(node: Node, total: number, x: number, depth: number, out: Block[]): void {
  if (node.count === 0) return;
  const width = (node.count / total) * 100;
  out.push({
    x,
    width,
    depth,
    label: node.label,
    count: node.count,
    pct: (node.count / total) * 100,
    node,
  });
  let offset = x;
  const sorted = [...node.children.values()].sort((a, b) => b.count - a.count);
  for (const c of sorted) {
    layout(c, total, offset, depth + 1, out);
    offset += (c.count / total) * 100;
  }
}

const ROW_HEIGHT = 20;
const typeColors = [
  "bg-orange-700/80 hover:bg-orange-600/80",
  "bg-sky-700/80 hover:bg-sky-600/80",
  "bg-emerald-700/70 hover:bg-emerald-600/70",
  "bg-violet-700/80 hover:bg-violet-600/80",
  "bg-amber-700/60 hover:bg-amber-600/60",
];

interface FlamegraphViewerProps {
  /** Phase 8 API: stacks from /api/aggregate */
  stacks?: StackCount[];
  totalSamples?: number;
  /** Regex filter (e.g. from search input) */
  searchRegex?: string;
  height?: number;
}

export function FlamegraphViewer({
  stacks = [],
  totalSamples = 0,
  searchRegex,
  height = 400,
}: FlamegraphViewerProps) {
  const [zoomNode, setZoomNode] = useState<Node | null>(null);

  const filteredStacks = useMemo(() => {
    if (!searchRegex?.trim()) return stacks;
    try {
      const re = new RegExp(searchRegex, "i");
      return stacks.filter(({ stack }) =>
        stack.frames.some((f) => re.test(frameLabel(f)))
      );
    } catch {
      return stacks;
    }
  }, [stacks, searchRegex]);

  const tree = useMemo(() => buildTree(filteredStacks), [filteredStacks]);
  const root = zoomNode ?? tree;
  const total = zoomNode ? zoomNode.count : (totalSamples || tree.count);
  const blocks = useMemo(() => {
    const out: Block[] = [];
    layout(root, total || 1, 0, 0, out);
    return out;
  }, [root, total]);

  const handleBlockClick = useCallback((b: Block) => {
    if (b.node.children.size > 0) setZoomNode(b.node);
    else setZoomNode(null);
  }, []);

  const maxDepth = blocks.length ? Math.max(...blocks.map((b) => b.depth)) : 0;
  const svgHeight = (maxDepth + 1) * ROW_HEIGHT;

  const getColor = (depth: number) => typeColors[depth % typeColors.length];

  if (stacks.length === 0) {
    return (
      <div className="rounded-md border border-border bg-card p-8 text-center text-sm text-muted-foreground">
        No profile data. Run a query from Dashboard or ensure the aggregator has data for the selected time range.
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="h-8 rounded border border-border bg-muted/30 relative overflow-hidden">
        <div
          className="absolute inset-0 opacity-30"
          style={{
            background:
              "linear-gradient(90deg, hsl(25,95%,55%) 0%, hsl(270,55%,40%) 40%, hsl(145,50%,30%) 70%, hsl(200,55%,35%) 100%)",
          }}
        />
      </div>

      <div className="rounded-md border border-border bg-card overflow-hidden">
        <div className="flex items-center justify-between px-3 py-2 border-b border-border text-xs text-muted-foreground">
          <span>
            {filteredStacks.length} stacks · {(total || tree.count).toLocaleString()} samples
            {zoomNode && " (zoomed)"}
          </span>
          {zoomNode && (
            <button
              type="button"
              onClick={() => setZoomNode(null)}
              className="text-primary hover:underline"
            >
              Reset zoom
            </button>
          )}
        </div>
        <div className="overflow-auto p-2" style={{ maxHeight: height }}>
          <svg
            width="100%"
            height={svgHeight}
            viewBox={`0 0 100 ${svgHeight}`}
            preserveAspectRatio="none"
            className="min-h-[200px]"
          >
            {blocks.map((b, i) => {
              if (b.width < 0.1) return null;
              return (
                <g key={i}>
                  <rect
                    x={b.x}
                    y={b.depth * ROW_HEIGHT}
                    width={b.width}
                    height={ROW_HEIGHT - 1}
                    className={cn("cursor-pointer transition-opacity hover:opacity-90", getColor(b.depth))}
                    onClick={() => handleBlockClick(b)}
                  />
                  {b.width > 8 && (
                    <text
                      x={b.x + b.width / 2}
                      y={b.depth * ROW_HEIGHT + ROW_HEIGHT / 2 + 4}
                      textAnchor="middle"
                      fill="white"
                      fontSize="10"
                      className="pointer-events-none"
                    >
                      {b.label.length > 20 ? b.label.slice(0, 18) + "…" : b.label}
                    </text>
                  )}
                  <title>
                    {b.label} — {b.count.toLocaleString()} ({b.pct.toFixed(1)}%) · Click to zoom
                  </title>
                </g>
              );
            })}
          </svg>
        </div>
      </div>

      <div className="flex items-center gap-4 text-[11px]">
        {["Native", "Python", "Kernel", "CUDA", "Other"].map((label, i) => (
          <div key={label} className="flex items-center gap-1.5">
            <div className={cn("h-3 w-3 rounded-sm", typeColors[i])} />
            <span className="text-muted-foreground">{label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
