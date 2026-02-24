import StatsOverview from "@/components/StatsOverview";
import StatsChart from "@/components/StatsChart";
import HistoryList from "@/components/HistoryList";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type {
  AnalyticsSummary,
  SessionRecord,
  TimeseriesPoint,
} from "@/types";
import type { StatsPeriod } from "@/app/types";

interface StatsTabProps {
  summary: AnalyticsSummary | undefined;
  statsPeriod: StatsPeriod;
  onStatsPeriodChange: (period: StatsPeriod) => void;
  timeseriesData: TimeseriesPoint[];
  statsDayHistory: SessionRecord[];
  historyData: SessionRecord[];
  rangeDays: number;
  onRangeDaysChange: (days: number) => void;
  onExportCsv: () => void;
  onExportJson: () => void;
  getProjectName: (id: number | null) => string;
}

export function StatsTab({
  summary,
  statsPeriod,
  onStatsPeriodChange,
  timeseriesData,
  statsDayHistory,
  historyData,
  rangeDays,
  onRangeDaysChange,
  onExportCsv,
  onExportJson,
  getProjectName,
}: StatsTabProps) {
  return (
    <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-6">
      <h2 className="text-2xl font-bold tracking-tight">Statistics</h2>
      <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
        <div className="p-6">
          <StatsOverview summary={summary} />
        </div>
      </div>

      <StatsChart
        period={statsPeriod}
        onPeriodChange={onStatsPeriodChange}
        timeseriesData={timeseriesData}
        sessionData={statsPeriod === "day" ? statsDayHistory : historyData}
      />

      <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
        <div className="flex flex-row items-center justify-between p-6 pb-2">
          <h3 className="text-lg font-semibold leading-none tracking-tight">
            History
          </h3>
          <div className="flex items-center gap-2">
            <Select
              value={rangeDays.toString()}
              onValueChange={(value) => onRangeDaysChange(Number(value))}
            >
              <SelectTrigger className="w-[140px]">
                <SelectValue placeholder="Select range" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="7">Last 7 days</SelectItem>
                <SelectItem value="14">Last 14 days</SelectItem>
                <SelectItem value="30">Last 30 days</SelectItem>
              </SelectContent>
            </Select>
            <div className="flex gap-1">
              <button
                className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-9 px-3"
                onClick={onExportCsv}
                title="Export CSV"
              >
                CSV
              </button>
              <button
                className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-9 px-3"
                onClick={onExportJson}
                title="Export JSON"
              >
                {"{ } JSON"}
              </button>
            </div>
          </div>
        </div>
        <div className="p-6 pt-0">
          <HistoryList history={historyData} getProjectName={getProjectName} />
        </div>
      </div>
    </div>
  );
}
