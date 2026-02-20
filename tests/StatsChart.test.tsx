import React from "react";
import { render } from "@testing-library/react";
import StatsChart from "@/components/StatsChart";
import type { SessionRecord } from "@/types";

let capturedChartData: Array<{ label: string; value: number }> = [];

vi.mock("recharts", async () => {
  const ReactModule = await import("react");

  return {
    ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
      <div>{children}</div>
    ),
    BarChart: ({
      data,
      children,
    }: {
      data: Array<{ label: string; value: number }>;
      children: React.ReactNode;
    }) => {
      capturedChartData = data;
      return <div>{children}</div>;
    },
    XAxis: () => null,
    YAxis: () => null,
    Tooltip: () => null,
    Bar: ({ children }: { children: React.ReactNode }) => (
      <div>{children}</div>
    ),
    Cell: () => null,
    __esModule: true,
    default: ReactModule,
  };
});

function tsAtHour(hour: number): number {
  return Math.floor(new Date(2026, 1, 19, hour, 0, 0).getTime() / 1000);
}

describe("StatsChart", () => {
  it("counts only focus sessions in day mode", () => {
    capturedChartData = [];
    const sessions: SessionRecord[] = [
      {
        id: 1,
        startedAt: tsAtHour(9),
        endedAt: tsAtHour(9),
        phase: "focus",
        durationSec: 1800,
        completed: true,
        interruptions: 0,
        projectId: null,
        tagIds: [],
      },
      {
        id: 2,
        startedAt: tsAtHour(9),
        endedAt: tsAtHour(9),
        phase: "short_break",
        durationSec: 300,
        completed: true,
        interruptions: 0,
        projectId: null,
        tagIds: [],
      },
      {
        id: 3,
        startedAt: tsAtHour(10),
        endedAt: tsAtHour(10),
        phase: "focus",
        durationSec: 900,
        completed: true,
        interruptions: 0,
        projectId: null,
        tagIds: [],
      },
    ];

    render(
      <StatsChart
        period="day"
        onPeriodChange={() => {}}
        timeseriesData={[]}
        sessionData={sessions}
      />,
    );

    expect(capturedChartData).toHaveLength(24);
    expect(capturedChartData.find((point) => point.label === "09:00")?.value).toBe(
      30,
    );
    expect(capturedChartData.find((point) => point.label === "10:00")?.value).toBe(
      15,
    );
  });
});
