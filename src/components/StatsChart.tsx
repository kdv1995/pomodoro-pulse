import { Bar, BarChart, ResponsiveContainer, XAxis, Tooltip, Cell, YAxis } from "recharts";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useMemo } from "react";
import { SessionRecord, TimeseriesPoint } from "../types";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";

interface StatsChartProps {
    period: "day" | "week" | "month";
    onPeriodChange: (period: "day" | "week" | "month") => void;
    timeseriesData?: TimeseriesPoint[];
    sessionData?: SessionRecord[];
}

export default function StatsChart({ period, onPeriodChange, timeseriesData, sessionData }: StatsChartProps) {
    const chartData = useMemo(() => {
        if (period === "day") {
            // Aggregate sessions by hour
            const hours = new Array(24).fill(0).map((_, i) => ({
                label: i.toString().padStart(2, '0') + ":00",
                value: 0,
                sortIndex: i
            }));

            sessionData?.forEach(session => {
                const hour = new Date(session.startedAt * 1000).getHours();
                hours[hour].value += session.durationSec / 60; // Minutes
            });

            return hours;
        } else if (period === "week") {
            // Last 7 days
            return timeseriesData?.map(point => ({
                label: new Date(point.date).toLocaleDateString("en-US", { weekday: "short" }),
                value: Number((point.focusSeconds / 3600).toFixed(1)), // Hours
                fullDate: point.date
            })) ?? [];
        } else {
            // Last 30 days
            return timeseriesData?.map(point => ({
                label: new Date(point.date).getDate().toString(),
                value: Number((point.focusSeconds / 3600).toFixed(1)), // Hours
                fullDate: point.date
            })) ?? [];
        }
    }, [period, timeseriesData, sessionData]);

    const title = period === "day" ? "Focus Minutes (Today)" : "Focus Hours";

    return (
        <Card>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-base font-normal">Activity</CardTitle>
                <Tabs value={period} onValueChange={(v) => onPeriodChange(v as "day" | "week" | "month")}>
                    <TabsList>
                        <TabsTrigger value="day">Day</TabsTrigger>
                        <TabsTrigger value="week">Week</TabsTrigger>
                        <TabsTrigger value="month">Month</TabsTrigger>
                    </TabsList>
                </Tabs>
            </CardHeader>
            <CardContent>
                <div className="h-[250px] w-full mt-4">
                    <ResponsiveContainer width="100%" height="100%">
                        <BarChart data={chartData}>
                            <XAxis
                                dataKey="label"
                                axisLine={false}
                                tickLine={false}
                                tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 12 }}
                                dy={10}
                                minTickGap={20}
                            />
                            <YAxis
                                axisLine={false}
                                tickLine={false}
                                tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 12 }}
                            />
                            <Tooltip
                                cursor={{ fill: "hsl(var(--muted)/0.2)" }}
                                contentStyle={{
                                    borderRadius: "var(--radius)",
                                    border: "1px solid hsl(var(--border))",
                                    backgroundColor: "hsl(var(--popover))",
                                    color: "hsl(var(--popover-foreground))",
                                    boxShadow: "0 4px 12px rgba(0,0,0,0.1)"
                                }}
                                formatter={(value: number | undefined) => [
                                    `${value ?? 0} ${period === "day" ? "mins" : "hrs"}`,
                                    title
                                ]}
                            />
                            <Bar dataKey="value" radius={[4, 4, 0, 0]}>
                                {chartData.map((_, index) => (
                                    <Cell key={`cell-${index}`} fill="hsl(var(--primary))" />
                                ))}
                            </Bar>
                        </BarChart>
                    </ResponsiveContainer>
                </div>
            </CardContent>
        </Card>
    );
}
