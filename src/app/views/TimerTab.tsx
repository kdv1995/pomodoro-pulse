import TimerDisplay from "@/components/TimerDisplay";
import Controls from "@/components/Controls";
import StatsOverview from "@/components/StatsOverview";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AnalyticsSummary, Project, Tag, TimerState } from "@/types";

interface TimerTabProps {
  timer: TimerState | null;
  summary: AnalyticsSummary | undefined;
  projects: Project[];
  tags: Tag[];
  selectedProjectId: number | null;
  selectedTagIds: number[];
  contextLocked: boolean;
  actionBusy: boolean;
  onToggleTimer: () => void;
  onSkip: () => void;
  onProjectChange: (projectId: number | null) => void;
  onTagToggle: (tagId: number) => void;
}

export function TimerTab({
  timer,
  summary,
  projects,
  tags,
  selectedProjectId,
  selectedTagIds,
  contextLocked,
  actionBusy,
  onToggleTimer,
  onSkip,
  onProjectChange,
  onTagToggle,
}: TimerTabProps) {
  return (
    <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-6">
      <section className="flex flex-col gap-6">
        {timer && (
          <TimerDisplay
            remainingSeconds={timer.remainingSeconds}
            phase={timer.phase}
            cycleIndex={timer.cycleIndex}
            interruptions={timer.interruptions}
          />
        )}

        <Controls
          timer={timer}
          onToggle={onToggleTimer}
          onSkip={onSkip}
          busy={actionBusy}
        />

        <div className="grid gap-4 sm:grid-cols-2">
          <label className="flex flex-col gap-2">
            <span className="text-sm font-medium">Focus Project</span>
            <Select
              value={
                selectedProjectId === null
                  ? "no-project"
                  : selectedProjectId.toString()
              }
              onValueChange={(value) => {
                onProjectChange(value === "no-project" ? null : Number(value));
              }}
              disabled={contextLocked}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select a project" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="no-project">No Project</SelectItem>
                {projects.map((project) => (
                  <SelectItem value={project.id.toString()} key={project.id}>
                    {project.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </label>

          <label className="flex flex-col gap-2">
            <span className="text-sm font-medium">Tags</span>
            <div className="flex flex-wrap gap-2">
              {tags.map((tag) => {
                const selected = selectedTagIds.includes(tag.id);
                return (
                  <button
                    type="button"
                    key={tag.id}
                    disabled={contextLocked}
                    className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ${
                      selected
                        ? "border-transparent bg-primary text-primary-foreground hover:bg-primary/80"
                        : "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80"
                    }`}
                    onClick={() => onTagToggle(tag.id)}
                  >
                    {tag.name}
                  </button>
                );
              })}
            </div>
          </label>
        </div>
      </section>

      <div>
        <StatsOverview summary={summary} />
      </div>
    </div>
  );
}
