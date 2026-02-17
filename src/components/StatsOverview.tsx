import { AnalyticsSummary, TimerPhase } from "../types";

interface StatsOverviewProps {
    summary?: AnalyticsSummary;
    currentPhase?: TimerPhase;
}

function formatDuration(seconds: number) {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.round((seconds % 3600) / 60);
    if (hrs > 0) {
        return `${hrs}h ${mins}m`;
    }
    return `${mins}m`;
}

export default function StatsOverview({ summary }: StatsOverviewProps) {
    return (
        <div className="grid grid-cols-2 gap-4 md:grid-cols-5">
            <div className="flex flex-col items-center justify-center rounded-lg border bg-card p-3 text-card-foreground shadow-sm">
                <span className="text-xs font-medium text-muted-foreground">Total Focus</span>
                <span className="text-xl font-bold tracking-tight">{formatDuration(summary?.totalFocusSec ?? 0)}</span>
            </div>
            <div className="flex flex-col items-center justify-center rounded-lg border bg-card p-3 text-card-foreground shadow-sm">
                <span className="text-xs font-medium text-muted-foreground">Pomodoros</span>
                <span className="text-xl font-bold tracking-tight">{summary?.completedPomodoros ?? 0}</span>
            </div>
            <div className="flex flex-col items-center justify-center rounded-lg border bg-card p-3 text-card-foreground shadow-sm">
                <span className="text-xs font-medium text-muted-foreground">Streak</span>
                <span className="text-xl font-bold tracking-tight">{summary?.streakDays ?? 0} <span className="text-xs font-normal text-muted-foreground">days</span></span>
            </div>
            <div className="flex flex-col items-center justify-center rounded-lg border bg-card p-3 text-card-foreground shadow-sm">
                <span className="text-xs font-medium text-muted-foreground">Interruptions</span>
                <span className="text-xl font-bold tracking-tight">{summary?.interruptions ?? 0}</span>
            </div>
            <div className="flex flex-col items-center justify-center rounded-lg border bg-card p-3 text-card-foreground shadow-sm col-span-2 md:col-span-1">
                <span className="text-xs font-medium text-muted-foreground">Daily Avg</span>
                <span className="text-xl font-bold tracking-tight">{formatDuration(summary?.avgDailyFocusSec ?? 0)}</span>
            </div>
        </div>
    );
}
