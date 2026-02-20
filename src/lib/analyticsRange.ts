import type { AnalyticsRange } from "@/types";

const DAY_SECONDS = 86_400;

export type StatsPeriod = "day" | "week" | "month";

export function statsDaysForPeriod(period: StatsPeriod): number {
  switch (period) {
    case "day":
      return 1;
    case "week":
      return 7;
    case "month":
      return 30;
    default:
      return 7;
  }
}

export function buildAnalyticsRange(
  days: number,
  nowTs = Math.floor(Date.now() / 1000),
): AnalyticsRange {
  const normalizedDays = Math.max(1, Math.floor(days));

  return {
    from: nowTs - normalizedDays * DAY_SECONDS,
    to: nowTs,
  };
}
