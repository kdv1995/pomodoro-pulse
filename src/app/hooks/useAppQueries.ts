import { useCallback, useMemo } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import {
  analyticsGetSummary,
  analyticsGetTimeseries,
  projectsList,
  sessionHistory,
  settingsGet,
  tagsList,
} from "@/api";
import { buildAnalyticsRange, statsDaysForPeriod } from "@/lib/analyticsRange";
import type { StatsPeriod } from "@/app/types";

export function useAppQueries(statsPeriod: StatsPeriod, rangeDays: number) {
  const queryClient = useQueryClient();

  const statsRange = useMemo(
    () => buildAnalyticsRange(statsDaysForPeriod(statsPeriod)),
    [statsPeriod],
  );
  const historyRange = useMemo(
    () => buildAnalyticsRange(rangeDays),
    [rangeDays],
  );

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: settingsGet,
  });

  const projectsQuery = useQuery({
    queryKey: ["projects"],
    queryFn: projectsList,
  });

  const tagsQuery = useQuery({
    queryKey: ["tags"],
    queryFn: tagsList,
  });

  const summaryQuery = useQuery({
    queryKey: ["summary", statsRange],
    queryFn: () => analyticsGetSummary(statsRange),
  });

  const seriesQuery = useQuery({
    queryKey: ["series", statsRange],
    queryFn: () => analyticsGetTimeseries(statsRange),
  });

  const historyQuery = useQuery({
    queryKey: ["history", historyRange],
    queryFn: () => sessionHistory(historyRange),
  });

  const statsHistoryQuery = useQuery({
    queryKey: ["history-stats", statsRange],
    queryFn: () => sessionHistory(statsRange),
  });

  const refreshAll = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["summary"] }),
      queryClient.invalidateQueries({ queryKey: ["series"] }),
      queryClient.invalidateQueries({ queryKey: ["history"] }),
      queryClient.invalidateQueries({ queryKey: ["history-stats"] }),
      queryClient.invalidateQueries({ queryKey: ["projects"] }),
      queryClient.invalidateQueries({ queryKey: ["tags"] }),
      queryClient.invalidateQueries({ queryKey: ["settings"] }),
    ]);
  }, [queryClient]);

  return {
    queryClient,
    statsRange,
    historyRange,
    settingsQuery,
    projectsQuery,
    tagsQuery,
    summaryQuery,
    seriesQuery,
    historyQuery,
    statsHistoryQuery,
    refreshAll,
  };
}
