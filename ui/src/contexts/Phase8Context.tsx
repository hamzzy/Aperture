import { createContext, useContext, useState, useCallback, type ReactNode } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { TimePreset } from "@/components/layout/TopBar";

type Phase8ContextValue = {
  timePreset: TimePreset;
  setTimePreset: (p: TimePreset) => void;
  registerRefresh: (fn: () => void) => void;
  triggerRefresh: () => void;
  refreshing: boolean;
  setRefreshing: (v: boolean) => void;
};

const Phase8Context = createContext<Phase8ContextValue | null>(null);

export function Phase8Provider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const [timePreset, setTimePreset] = useState<TimePreset>("1h");
  const [onRefresh, setOnRefresh] = useState<(() => void) | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const registerRefresh = useCallback((fn: () => void) => {
    setOnRefresh(() => fn);
  }, []);
  const triggerRefresh = useCallback(() => {
    queryClient.invalidateQueries();
    onRefresh?.();
  }, [onRefresh, queryClient]);
  return (
    <Phase8Context.Provider
      value={{
        timePreset,
        setTimePreset,
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
