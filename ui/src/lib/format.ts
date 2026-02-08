/**
 * Format helpers for Phase 8 UI (ns, samples, percentages)
 */

export function formatNs(ns: number): string {
  if (ns >= 1e9) return `${(ns / 1e9).toFixed(2)}s`;
  if (ns >= 1e6) return `${(ns / 1e6).toFixed(2)}ms`;
  if (ns >= 1e3) return `${(ns / 1e3).toFixed(2)}μs`;
  return `${ns}ns`;
}

/** Format time range (nanoseconds since epoch) for display */
export function formatTimeRange(startNs: number, endNs: number): string {
  const start = new Date(Number(startNs) / 1e6);
  const end = new Date(Number(endNs) / 1e6);
  return `${start.toLocaleString()} → ${end.toLocaleString()}`;
}
