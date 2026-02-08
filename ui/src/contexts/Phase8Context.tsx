import { createContext, useContext, useState, useCallback, useMemo, type ReactNode } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { TimePreset } from "@/components/layout/TopBar";
import { nsFromPreset } from "@/components/layout/TopBar";

type TimeRange = { start: number; end: number };

type Phase8ContextValue = {
  timePreset: TimePreset;
  setTimePreset: (p: TimePreset) => void;
  /** Stable time range (ms) — only changes on preset change or refresh. */
  timeRange: TimeRange;
  registerRefresh: (fn: () => void) => void;
  triggerRefresh: () => void;
  refreshing: boolean;
  setRefreshing: (v: boolean) => void;
};

const Phase8Context = createContext<Phase8ContextValue | null>(null);

export function Phase8Provider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [timePreset, setTimePresetRaw] = useState<TimePreset>("1h");
  const [onRefresh, setOnRefresh] = useState<(() => void) | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  // refreshCounter bumps when user refreshes, causing timeRange to recompute
  const [refreshCounter, setRefreshCounter] = useState(0);

  // Stable time range: only recomputes when preset or refreshCounter changes
  const timeRange = useMemo<TimeRange>(
    () => nsFromPreset(timePreset),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [timePreset, refreshCounter]
  );

  const setTimePreset = useCallback((p: TimePreset) => {
    setTimePresetRaw(p);
  }, []);

  const registerRefresh = useCallback((fn: () => void) => {
    setOnRefresh(() => fn);
  }, []);
  const triggerRefresh = useCallback(() => {
    setRefreshCounter((c) => c + 1);
    queryClient.invalidateQueries();
    onRefresh?.();
  }, [onRefresh, queryClient]);
  return (
    <Phase8Context.Provider
      value={{
        timePreset,
        setTimePreset,
        timeRange,
        registerRefresh,
        triggerRefresh,
        refreshing,
        setRefreshing,
      }}
    >
      {children}
    </Phase8Context.Provider>
  );
}

export function usePhase8() {
  const ctx = useContext(Phase8Context);
  return ctx;
}
