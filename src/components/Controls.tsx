import { TimerState } from "../types";
import { Button } from "@/components/ui/button";
import { Play, Pause, SkipForward } from "lucide-react";

interface ControlsProps {
    timer: TimerState | null;
    onToggle: () => void;
    onSkip: () => void;
    busy: boolean;
}

export default function Controls({
    timer,
    onToggle,
    onSkip,
    busy,
}: ControlsProps) {
    const isRunning = timer?.isRunning ?? false;
    const hasStarted =
        timer?.startedAt !== null && timer?.startedAt !== undefined;

    return (
        <div className="flex flex-row justify-center gap-4 mt-2">
            <Button
                variant={isRunning ? "secondary" : "default"}
                disabled={!timer || busy}
                onClick={onToggle}
                size="lg"
                className="min-w-[140px] gap-2 h-12 text-lg"
            >
                {isRunning ? (
                    <>
                        <Pause className="h-5 w-5" /> Pause
                    </>
                ) : hasStarted ? (
                    <>
                        <Play className="h-5 w-5" /> Resume
                    </>
                ) : (
                    <>
                        <Play className="h-5 w-5" /> Start Focus
                    </>
                )}
            </Button>

            <Button
                variant="ghost"
                disabled={!timer || busy}
                onClick={onSkip}
                size="lg"
                className="h-12 w-12 p-0"
                title="Skip current phase"
            >
                <SkipForward className="h-6 w-6" />
            </Button>
        </div>
    );
}
