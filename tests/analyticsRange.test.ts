import { buildAnalyticsRange, statsDaysForPeriod } from "@/lib/analyticsRange";

describe("analytics range helpers", () => {
  it("maps stats periods to expected day counts", () => {
    expect(statsDaysForPeriod("day")).toBe(1);
    expect(statsDaysForPeriod("week")).toBe(7);
    expect(statsDaysForPeriod("month")).toBe(30);
  });

  it("builds range for a given number of days", () => {
    const nowTs = 1_700_000_000;
    const range = buildAnalyticsRange(14, nowTs);

    expect(range.to).toBe(nowTs);
    expect(range.from).toBe(nowTs - 14 * 86_400);
  });

  it("normalizes invalid day input to one day", () => {
    const nowTs = 1_700_000_000;
    const range = buildAnalyticsRange(0, nowTs);

    expect(range.to).toBe(nowTs);
    expect(range.from).toBe(nowTs - 86_400);
  });
});
