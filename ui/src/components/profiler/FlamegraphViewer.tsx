import { useMemo, useState, useCallback, useRef, type MouseEvent } from "react";
import type { StackCount, Frame } from "@/api/types";

function frameLabel(f: Frame): string {
  return f.function ?? f.module ?? `0x${f.ip.toString(16)}`;
}

/* ── Tree types ───────────────────────────────────────────────── */

interface FNode {
  label: string;
  /** Total samples (includes children) */
  total: number;
  /** Self samples (leaf-only, not passed to children) */
  self: number;
  children: Map<string, FNode>;
}

function buildTree(stacks: StackCount[]): FNode {
  const root: FNode = { label: "root", total: 0, self: 0, children: new Map() };
  for (const { stack, count } of stacks) {
    const frames = stack.frames;
    if (frames.length === 0) continue;
    root.total += count;
    let current = root;
    // Walk bottom-up (callers first)
    for (let i = frames.length - 1; i >= 0; i--) {
      const f = frames[i];
      const key = `${f.ip}-${f.function ?? ""}`;
      let child = current.children.get(key);
      if (!child) {
        child = { label: frameLabel(f), total: 0, self: 0, children: new Map() };
        current.children.set(key, child);
      }
      child.total += count;
      if (i === 0) child.self += count; // leaf frame
      current = child;
    }
  }
  return root;
}

/* ── Layout ───────────────────────────────────────────────────── */

const ROW_H = 22;

interface Block {
  /** Left offset as fraction 0..1 relative to zoom root */
  x: number;
  /** Width as fraction 0..1 */
  w: number;
  depth: number;
  node: FNode;
}

function layoutBlocks(root: FNode, total: number): Block[] {
  const out: Block[] = [];
  const MIN_W = 0.002; // hide blocks < 0.2% width

  function walk(node: FNode, x: number, depth: number) {
    const w = total > 0 ? node.total / total : 0;
    if (w < MIN_W && depth > 0) return;
    out.push({ x, w, depth, node });
    // Sort children by total descending for stable layout
    const sorted = [...node.children.values()].sort((a, b) => b.total - a.total);
    let childX = x;
    for (const c of sorted) {
      walk(c, childX, depth + 1);
      childX += total > 0 ? c.total / total : 0;
    }
  }
  walk(root, 0, 0);
  return out;
}

/* ── Color palette ────────────────────────────────────────────── */

function flameColor(label: string, depth: number): string {
  // Simple hash for deterministic color per function
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) | 0;
  const hue = 10 + (Math.abs(h) % 40); // warm range 10-50
  const sat = 65 + (depth % 3) * 10;
  const lit = 42 + (Math.abs(h >> 8) % 15);
  return `hsl(${hue}, ${sat}%, ${lit}%)`;
}

function kernelColor(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) | 0;
  const hue = 180 + (Math.abs(h) % 60); // cool range
  return `hsl(${hue}, 50%, 40%)`;
}

function getBlockColor(label: string, depth: number): string {
  if (depth === 0) return "hsl(220, 15%, 30%)"; // root
  // Kernel frames often start with specific prefixes
  if (
    label.startsWith("__") ||
    label.startsWith("do_") ||
    label.startsWith("sys_") ||
    label.startsWith("entry_") ||
    label.startsWith("asm_") ||
    label.includes("[kernel]")
  ) {
    return kernelColor(label);
  }
  return flameColor(label, depth);
}

/* ── Tooltip ──────────────────────────────────────────────────── */

interface TooltipInfo {
  label: string;
  total: number;
  self: number;
  totalPct: string;
  selfPct: string;
  x: number;
  y: number;
}

function Tooltip({ info }: { info: TooltipInfo }) {
  return (
    <div
      className="fixed z-50 pointer-events-none px-3 py-2 rounded-md shadow-lg text-xs border border-border bg-popover text-popover-foreground"
      style={{
        left: info.x + 12,
        top: info.y - 10,
        maxWidth: 420,
      }}
    >
      <div className="font-mono font-medium truncate mb-1">{info.label}</div>
      <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-muted-foreground">
        <span>Total:</span>
        <span className="text-foreground font-mono">
          {info.total.toLocaleString()} ({info.totalPct})
        </span>
        <span>Self:</span>
        <span className="text-foreground font-mono">
          {info.self.toLocaleString()} ({info.selfPct})
        </span>
      </div>
    </div>
  );
}

/* ── Main component ───────────────────────────────────────────── */

interface FlamegraphViewerProps {
  stacks?: StackCount[];
  totalSamples?: number;
  searchRegex?: string;
  height?: number;
}

export function FlamegraphViewer({
  stacks = [],
  totalSamples = 0,
  searchRegex,
  height = 500,
}: FlamegraphViewerProps) {
  const [zoomNode, setZoomNode] = useState<FNode | null>(null);
  const [tooltip, setTooltip] = useState<TooltipInfo | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const tree = useMemo(() => buildTree(stacks), [stacks]);

  const root = zoomNode ?? tree;
  const rootTotal = zoomNode ? zoomNode.total : totalSamples || tree.total;

  const blocks = useMemo(() => layoutBlocks(root, rootTotal), [root, rootTotal]);

  const maxDepth = blocks.length ? Math.max(...blocks.map((b) => b.depth)) : 0;
  const graphHeight = (maxDepth + 1) * ROW_H;

  // Search regex compilation
  const searchRe = useMemo(() => {
    if (!searchRegex?.trim()) return null;
    try {
      return new RegExp(searchRegex, "i");
    } catch {
      return null;
    }
  }, [searchRegex]);

  const handleClick = useCallback(
    (node: FNode) => {
      if (zoomNode === node) {
        setZoomNode(null); // click zoomed root to unzoom
      } else if (node.children.size > 0) {
        setZoomNode(node);
      }
    },
    [zoomNode],
  );

  const handleMouseMove = useCallback(
    (e: MouseEvent, block: Block) => {
      const pct = rootTotal > 0 ? (block.node.total / rootTotal) * 100 : 0;
      const selfPct = rootTotal > 0 ? (block.node.self / rootTotal) * 100 : 0;
      setTooltip({
        label: block.node.label,
        total: block.node.total,
        self: block.node.self,
        totalPct: `${pct.toFixed(2)}%`,
        selfPct: `${selfPct.toFixed(2)}%`,
        x: e.clientX,
        y: e.clientY,
      });
    },
    [rootTotal],
  );

  const handleMouseLeave = useCallback(() => setTooltip(null), []);

  if (stacks.length === 0) {
    return (
      <div className="rounded-md border border-border bg-card p-8 text-center text-sm text-muted-foreground">
        No profile data. Run a query from Dashboard or ensure the aggregator has
        data for the selected time range.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {/* Header bar */}
      <div className="rounded-md border border-border bg-card overflow-hidden">
        <div className="flex items-center justify-between px-3 py-2 border-b border-border text-xs text-muted-foreground">
          <span>
            {stacks.length} stacks · {rootTotal.toLocaleString()} samples
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

        {/* Flame graph area */}
        <div
          ref={containerRef}
          className="overflow-auto relative"
          style={{ maxHeight: height }}
          onMouseLeave={handleMouseLeave}
        >
          <div style={{ height: graphHeight, position: "relative" }}>
            {blocks.map((b, i) => {
              const leftPct = b.x * 100;
              const widthPct = b.w * 100;
              if (widthPct < 0.08) return null;

              const isSearchMatch = searchRe ? searchRe.test(b.node.label) : false;
              const dimmed = searchRe && !isSearchMatch;

              return (
                <div
                  key={i}
                  role="button"
                  tabIndex={0}
                  onClick={() => handleClick(b.node)}
                  onMouseMove={(e) => handleMouseMove(e, b)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleClick(b.node);
                  }}
                  className="absolute box-border border-r border-b cursor-pointer transition-opacity"
                  style={{
                    left: `${leftPct}%`,
                    width: `${widthPct}%`,
                    top: b.depth * ROW_H,
                    height: ROW_H,
                    backgroundColor: getBlockColor(b.node.label, b.depth),
                    borderColor: "rgba(0,0,0,0.15)",
                    opacity: dimmed ? 0.3 : 1,
                  }}
                >
                  {widthPct > 2 && (
                    <span
                      className="block truncate px-1 text-white select-none leading-snug"
                      style={{
                        fontSize: 11,
                        lineHeight: `${ROW_H}px`,
                      }}
                    >
                      {b.node.label}
                    </span>
                  )}
                  {searchRe && isSearchMatch && (
                    <div
                      className="absolute inset-0 pointer-events-none"
                      style={{
                        boxShadow: "inset 0 0 0 1.5px hsl(50, 100%, 60%)",
                      }}
                    />
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Tooltip (portal-free, positioned fixed) */}
      {tooltip && <Tooltip info={tooltip} />}
    </div>
  );
}
