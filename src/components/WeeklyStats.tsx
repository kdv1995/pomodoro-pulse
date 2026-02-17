import { BarChart, Bar, XAxis, Tooltip, ResponsiveContainer, Cell } from "recharts";
import { TimeseriesPoint } from "../types";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

interface WeeklyStatsProps {
    data: TimeseriesPoint[];
}

export default function WeeklyStats({ data }: WeeklyStatsProps) {
    const formattedData = data.map((point) => {
        const date = new Date(point.date);
        return {
            ...point,
            day: date.toLocaleDateString("en-US", { weekday: "short" }), // Mon, Tue...
            focusHours: Number((point.focusSeconds / 3600).toFixed(1)),
        };
    });

    return (
        <Card>
            <CardHeader>
                <CardTitle>Weekly Focus</CardTitle>
            </CardHeader>
            <CardContent>
                <div className="h-[200px] w-full">
                    <ResponsiveContainer width="100%" height="100%">
                        <BarChart data={formattedData}>
                            <XAxis
                                dataKey="day"
                                axisLine={false}
                                tickLine={false}
                                tick={{ fill: "hsl(var(--muted-foreground))", fontSize: 12 }}
                                dy={10}
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
                            />
                            <Bar dataKey="focusHours" radius={[4, 4, 0, 0]}>
                                {formattedData.map((_, index) => (
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
