import { useMemo, useState, useCallback, useRef, type MouseEvent } from "react";
import type { StackCount, Frame } from "@/api/types";

/* ── Symbol parsing ───────────────────────────────────────────── */

/** Detect hex-only addresses like "0xffff8b5b" */
function isHexAddress(name?: string): boolean {
  return !!name && /^0x[0-9a-f]+$/i.test(name);
}

/**
 * Parse "function_name [module_basename]" format produced by the agent.
 * Falls back to treating the whole string as the name.
 */
function parseSymbol(raw: string): { name: string; module?: string } {
  const m = raw.match(/^(.+?)\s+\[(.+)\]$/);
  if (m) return { name: m[1], module: m[2] };
  return { name: raw };
}

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
  /** Module basename (parsed from "func [module]" format) */
  module?: string;
  /** Raw IP address */
  ip?: number;
  /** True if the function name is just a hex address */
  isUnresolved: boolean;
}

function buildTree(stacks: StackCount[]): FNode {
  const root: FNode = {
    label: "root",
    total: 0,
    self: 0,
    children: new Map(),
    isUnresolved: false,
  };
  for (const { stack, count } of stacks) {
    const frames = stack.frames;
    if (frames.length === 0) continue;
    root.total += count;
    let current = root;
    // Walk bottom-up (callers first)
    for (let i = frames.length - 1; i >= 0; i--) {
      const f = frames[i];
      const raw = frameLabel(f);
      const key = `${f.ip}-${raw}`;
      let child = current.children.get(key);
      if (!child) {
        const parsed = parseSymbol(raw);
        child = {
          label: parsed.name,
          total: 0,
          self: 0,
          children: new Map(),
          module: parsed.module ?? f.module ?? undefined,
          ip: f.ip,
          isUnresolved: isHexAddress(parsed.name),
        };
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
  x: number;
  w: number;
  depth: number;
  node: FNode;
}

function layoutBlocks(root: FNode, total: number): Block[] {
  const out: Block[] = [];
  const MIN_W = 0.002;

  function walk(node: FNode, x: number, depth: number) {
    const w = total > 0 ? node.total / total : 0;
    if (w < MIN_W && depth > 0) return;
    out.push({ x, w, depth, node });
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
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) | 0;
  const hue = 10 + (Math.abs(h) % 40);
  const sat = 65 + (depth % 3) * 10;
  const lit = 42 + (Math.abs(h >> 8) % 15);
  return `hsl(${hue}, ${sat}%, ${lit}%)`;
}

function kernelColor(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) | 0;
  const hue = 180 + (Math.abs(h) % 60);
  return `hsl(${hue}, 50%, 40%)`;
}

function getBlockColor(node: FNode, depth: number): string {
  if (depth === 0) return "hsl(220, 15%, 30%)";
  if (node.isUnresolved) return "hsl(220, 8%, 28%)"; // grey for unresolved
  const label = node.label;
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
  module?: string;
  ip?: number;
  isUnresolved: boolean;
  total: number;
  self: number;
  totalPct: number;
  selfPct: number;
  depth: number;
  x: number;
  y: number;
}

function Tooltip({ info }: { info: TooltipInfo }) {
  return (
    <div
      className="fixed z-50 pointer-events-none px-3 py-2.5 rounded-md shadow-lg text-xs border border-border bg-popover text-popover-foreground"
      style={{
        left: info.x + 14,
        top: info.y - 12,
        maxWidth: 480,
      }}
    >
      {/* Function name */}
      <div className="flex items-center gap-2 mb-1">
        <span className="font-mono font-medium truncate">
          {info.label}
        </span>
        {info.isUnresolved && (
          <span className="shrink-0 text-[9px] px-1 py-0.5 rounded bg-muted text-muted-foreground">
            unresolved
          </span>
        )}
      </div>

      {/* Module */}
      {info.module && (
        <div className="text-[10px] text-muted-foreground mb-1.5 truncate font-mono">
          {info.module}
        </div>
      )}

      {/* Metrics */}
      <div className="space-y-1">
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground w-8">Total</span>
          <div className="flex-1 h-1.5 rounded-full bg-muted overflow-hidden">
            <div
              className="h-full rounded-full bg-primary"
              style={{ width: `${Math.min(100, info.totalPct)}%` }}
            />
          </div>
          <span className="font-mono text-foreground w-20 text-right">
            {info.total.toLocaleString()} ({info.totalPct.toFixed(1)}%)
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground w-8">Self</span>
          <div className="flex-1 h-1.5 rounded-full bg-muted overflow-hidden">
            <div
              className="h-full rounded-full bg-orange-500"
              style={{ width: `${Math.min(100, info.selfPct)}%` }}
            />
          </div>
          <span className="font-mono text-foreground w-20 text-right">
            {info.self.toLocaleString()} ({info.selfPct.toFixed(1)}%)
          </span>
        </div>
      </div>

      {/* Depth */}
      <div className="text-[10px] text-muted-foreground mt-1.5">
        Depth {info.depth}
        {info.ip != null && !info.isUnresolved && (
          <span className="ml-2">IP 0x{info.ip.toString(16)}</span>
        )}
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
        setZoomNode(null);
      } else if (node.children.size > 0) {
        setZoomNode(node);
      }
    },
    [zoomNode],
  );

  const handleMouseMove = useCallback(
    (e: MouseEvent, block: Block) => {
      const totalPct = rootTotal > 0 ? (block.node.total / rootTotal) * 100 : 0;
      const selfPct = rootTotal > 0 ? (block.node.self / rootTotal) * 100 : 0;
      setTooltip({
        label: block.node.label,
        module: block.node.module,
        ip: block.node.ip,
        isUnresolved: block.node.isUnresolved,
        total: block.node.total,
        self: block.node.self,
        totalPct,
        selfPct,
        depth: block.depth,
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
                    if (e.key === "Escape") setZoomNode(null);
                  }}
                  className="absolute box-border border-r border-b cursor-pointer transition-opacity"
                  style={{
                    left: `${leftPct}%`,
                    width: `${widthPct}%`,
                    top: b.depth * ROW_H,
                    height: ROW_H,
                    backgroundColor: getBlockColor(b.node, b.depth),
                    borderColor: "rgba(0,0,0,0.15)",
                    opacity: dimmed ? 0.3 : 1,
                  }}
                >
                  {widthPct > 2 && (
                    <span
                      className={`block truncate px-1 select-none leading-snug ${
                        b.node.isUnresolved
                          ? "text-white/50 italic"
                          : "text-white"
                      }`}
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

      {tooltip && <Tooltip info={tooltip} />}
    </div>
  );
}
