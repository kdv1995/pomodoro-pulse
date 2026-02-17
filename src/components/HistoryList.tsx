import { format } from "date-fns";
import { SessionRecord, TimerPhase } from "../types";
import { Badge } from "@/components/ui/badge";

interface HistoryListProps {
    history: SessionRecord[];
    getProjectName: (id: number | null) => string;
}

function formatDuration(seconds: number) {
    const mins = Math.round(seconds / 60);
    return `${mins}m`;
}

function phaseLabel(phase: TimerPhase) {
    switch (phase) {
        case "focus":
            return "Focus";
        case "short_break":
            return "Break"; // Shortened for badge
        case "long_break":
            return "Long Break";
        default:
            return phase;
    }
}

export default function HistoryList({ history, getProjectName }: HistoryListProps) {
    return (
        <div className="space-y-4">
            {/* Note: Title is handled by parent in App.tsx now? No, App.tsx has "History" header in the card around it? 
                 Let's check App.tsx again. App.tsx HAS a card around HistoryList.
                 So HistoryList should probably NOT wrap in another Card if it's already inside one.
                 
                 In App.tsx:
                 <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
                   ... title ...
                   <HistoryList ... />
                 </div>

                 So HistoryList should just return the table.
             */}
            <div className="relative w-full overflow-auto">
                <table className="w-full caption-bottom text-sm text-left">
                    <thead className="[&_tr]:border-b">
                        <tr className="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                            <th className="h-10 px-2 text-left align-middle font-medium text-muted-foreground w-[80px]">Time</th>
                            <th className="h-10 px-2 text-left align-middle font-medium text-muted-foreground w-[100px]">Phase</th>
                            <th className="h-10 px-2 text-left align-middle font-medium text-muted-foreground w-[80px]">Duration</th>
                            <th className="h-10 px-2 text-left align-middle font-medium text-muted-foreground">Project</th>
                        </tr>
                    </thead>
                    <tbody className="[&_tr:last-child]:border-0">
                        {history.slice(0, 50).map((session) => (
                            <tr key={session.id} className="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                                <td className="p-2 align-middle">{format(new Date(session.endedAt * 1000), "HH:mm")}</td>
                                <td className="p-2 align-middle">
                                    <Badge variant={session.phase === "focus" ? "default" : "secondary"} className="text-xs font-normal">
                                        {phaseLabel(session.phase)}
                                    </Badge>
                                </td>
                                <td className="p-2 align-middle">{formatDuration(session.durationSec)}</td>
                                <td className="p-2 align-middle text-muted-foreground">
                                    {getProjectName(session.projectId)}
                                </td>
                            </tr>
                        ))}
                        {history.length === 0 && (
                            <tr>
                                <td colSpan={4} className="p-4 text-center text-muted-foreground">
                                    No sessions recorded yet.
                                </td>
                            </tr>
                        )}
                    </tbody>
                </table>
            </div>
        </div>
    );
}
