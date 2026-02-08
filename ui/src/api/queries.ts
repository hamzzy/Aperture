/**
 * React Query hooks for aggregator API.
 * Uses @tanstack/react-query for caching, refetch, and invalidation.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchAggregate,
  fetchBatches,
  fetchDiff,
  fetchHealth,
} from "./client";
import type { CpuDiffJson } from "./types";

export const queryKeys = {
  health: ["health"] as const,
  aggregate: (timeStartNs?: number, timeEndNs?: number, eventType?: string) =>
    ["aggregate", timeStartNs, timeEndNs, eventType] as const,
  batches: (agentId?: string) => ["batches", agentId] as const,
};

export function useHealthQuery() {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: fetchHealth,
    refetchInterval: 30_000,
    retry: 1,
  });
}

export function useAggregateQuery(params: {
  time_start_ns?: number;
  time_end_ns?: number;
  limit?: number;
  event_type?: string;
  enabled?: boolean;
}) {
  const { time_start_ns, time_end_ns, limit = 500, event_type, enabled = true } = params;
  return useQuery({
    queryKey: queryKeys.aggregate(time_start_ns, time_end_ns, event_type ?? ""),
    queryFn: () =>
      fetchAggregate({
        time_start_ns,
        time_end_ns,
        limit,
        event_type: event_type || undefined,
      }),
    enabled: enabled && (time_start_ns != null || time_end_ns != null),
  });
}

export function useBatchesQuery(params?: { agent_id?: string; limit?: number }) {
  return useQuery({
    queryKey: queryKeys.batches(params?.agent_id),
    queryFn: () => fetchBatches({ agent_id: params?.agent_id, limit: params?.limit ?? 50 }),
  });
}

export function useDiffMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: Parameters<typeof fetchDiff>[0]) => {
      const res = await fetchDiff(params);
      if (res.error) throw new Error(res.error);
      return JSON.parse(res.result_json) as CpuDiffJson;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["aggregate"] });
    },
  });
}
