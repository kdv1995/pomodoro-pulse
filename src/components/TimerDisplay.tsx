import { TimerPhase } from "../types";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface TimerDisplayProps {
  remainingSeconds: number;
  phase: TimerPhase;
  cycleIndex: number;
  interruptions: number;
}

function formatClock(totalSeconds: number) {
  const safe = Math.max(0, totalSeconds);
  const mm = Math.floor(safe / 60)
    .toString()
    .padStart(2, "0");
  const ss = Math.floor(safe % 60)
    .toString()
    .padStart(2, "0");
  return `${mm}:${ss}`;
}

function phaseLabel(phase: TimerPhase) {
  switch (phase) {
    case "focus":
      return "Focus Time";
    case "short_break":
      return "Short Break";
    case "long_break":
      return "Long Break";
    default:
      return phase;
  }
}

export default function TimerDisplay({
  remainingSeconds,
  phase,
  cycleIndex,
  interruptions,
}: TimerDisplayProps) {
  return (
    <Card className="flex flex-col items-center justify-center p-8 shadow-lg">
      <Badge variant={phase === "focus" ? "default" : "secondary"} className="mb-4 text-sm uppercase tracking-wider">
        {phaseLabel(phase)}
      </Badge>
      <div className="font-mono text-8xl font-bold tracking-tighter tabular-nums mb-4 text-primary">
        {formatClock(remainingSeconds)}
      </div>
      <p className="text-sm text-muted-foreground flex items-center gap-2">
        <span className="font-medium">Cycle #{cycleIndex + 1}</span>
        <span className="h-1 w-1 rounded-full bg-muted-foreground" />
        <span>Interruptions: {interruptions}</span>
      </p>
    </Card>
  );
}
