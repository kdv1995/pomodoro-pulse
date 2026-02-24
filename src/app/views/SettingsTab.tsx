import SettingsPanel from "@/components/SettingsPanel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { AppSettings } from "@/types";

interface SettingsTabProps {
  settings: AppSettings | null;
  onSettingsUpdate: (next: AppSettings) => void;
  onSaveSettings: () => void;
  newProjectName: string;
  onNewProjectNameChange: (value: string) => void;
  newProjectColor: string;
  onNewProjectColorChange: (value: string) => void;
  newTagName: string;
  onNewTagNameChange: (value: string) => void;
  onAddProject: () => void;
  onAddTag: () => void;
  onResetAllData: () => void;
  actionBusy: boolean;
}

export function SettingsTab({
  settings,
  onSettingsUpdate,
  onSaveSettings,
  newProjectName,
  onNewProjectNameChange,
  newProjectColor,
  onNewProjectColorChange,
  newTagName,
  onNewTagNameChange,
  onAddProject,
  onAddTag,
  onResetAllData,
  actionBusy,
}: SettingsTabProps) {
  return (
    <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-6">
      <h2 className="text-2xl font-bold tracking-tight">Settings</h2>
      <SettingsPanel
        settings={settings}
        onUpdate={onSettingsUpdate}
        onSave={onSaveSettings}
      />

      <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
        <div className="flex flex-col space-y-1.5 p-6">
          <h3 className="text-lg font-semibold leading-none tracking-tight">
            Manage Projects & Tags
          </h3>
        </div>
        <div className="p-6 pt-0 space-y-4">
          <div className="space-y-2">
            <h4 className="text-sm font-medium leading-none">Add Project</h4>
            <div className="flex gap-2">
              <Input
                value={newProjectName}
                placeholder="Project Name"
                onChange={(event) =>
                  onNewProjectNameChange(event.currentTarget.value)
                }
              />
              <input
                type="color"
                className="h-10 w-12 rounded-md border border-input bg-background p-1 cursor-pointer"
                value={newProjectColor}
                onChange={(event) =>
                  onNewProjectColorChange(event.currentTarget.value)
                }
              />
              <Button variant="secondary" onClick={onAddProject}>
                Add
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <h4 className="text-sm font-medium leading-none">Add Tag</h4>
            <div className="flex gap-2">
              <Input
                value={newTagName}
                placeholder="Tag Name"
                onChange={(event) => onNewTagNameChange(event.currentTarget.value)}
              />
              <Button variant="secondary" onClick={onAddTag}>
                Add
              </Button>
            </div>
          </div>
        </div>
      </div>

      <div className="rounded-xl border border-destructive/20 bg-destructive/10 text-destructive shadow-sm">
        <div className="flex flex-col space-y-1.5 p-6">
          <h4 className="text-lg font-semibold leading-none tracking-tight">
            Danger Zone
          </h4>
          <p className="text-sm text-muted-foreground">
            Delete all sessions, projects, tags, and restore default settings.
          </p>
        </div>
        <div className="p-6 pt-0">
          <button
            className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 h-9 px-4 py-2"
            onClick={onResetAllData}
            disabled={actionBusy}
          >
            {actionBusy ? "Working..." : "Reset Everything"}
          </button>
        </div>
      </div>
    </div>
  );
}
