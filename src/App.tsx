import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import {
  exportCsv,
  exportJson,
  projectsUpsert,
  resetAllData,
  settingsUpdate,
  tagsUpsert,
  timerPause,
  timerResume,
  timerSetContext,
  timerSkip,
  timerStart,
} from "@/api";
import type { AppSettings, TimerState } from "@/types";
import type { AppTab, StatsPeriod } from "@/app/types";
import { useAppQueries } from "@/app/hooks/useAppQueries";
import { useThemeSync } from "@/app/hooks/useThemeSync";
import { useTimerLifecycle } from "@/app/hooks/useTimerLifecycle";
import { toErrorMessage } from "@/app/utils/error";
import { phaseLabel } from "@/app/utils/timer";
import { SettingsTab } from "@/app/views/SettingsTab";
import { StatsTab } from "@/app/views/StatsTab";
import { TimerTab } from "@/app/views/TimerTab";
import Sidebar from "@/components/Sidebar";
import TitleBar from "@/components/TitleBar";
import "./App.css";

export default function App() {
  const [timer, setTimer] = useState<TimerState | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<number | null>(null);
  const [selectedTagIds, setSelectedTagIds] = useState<number[]>([]);
  const [rangeDays, setRangeDays] = useState(14);
  const [settingsDraft, setSettingsDraft] = useState<AppSettings | null>(null);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectColor, setNewProjectColor] = useState("#f97316");
  const [newTagName, setNewTagName] = useState("");
  const [statusMessage, setStatusMessage] = useState("");
  const [actionBusy, setActionBusy] = useState(false);
  const [activeTab, setActiveTab] = useState<AppTab>("timer");
  const [statsPeriod, setStatsPeriod] = useState<StatsPeriod>("week");

  const {
    queryClient,
    historyRange,
    settingsQuery,
    projectsQuery,
    tagsQuery,
    summaryQuery,
    seriesQuery,
    historyQuery,
    statsHistoryQuery,
    refreshAll,
  } = useAppQueries(statsPeriod, rangeDays);

  useEffect(() => {
    if (settingsQuery.data) {
      setSettingsDraft(settingsQuery.data);
    }
  }, [settingsQuery.data]);

  useThemeSync(settingsDraft);

  const reportActionError = useCallback((title: string, error: unknown) => {
    const details = toErrorMessage(error);
    setStatusMessage(details);
    toast.error(title, {
      description: details,
      position: "top-center",
      duration: 2500,
    });
  }, []);

  useTimerLifecycle({
    timer,
    queryClient,
    setTimer,
    setSelectedProjectId,
    setSelectedTagIds,
    onError: reportActionError,
    notificationsEnabled: settingsDraft?.notificationsEnabled,
    soundEnabled: settingsDraft?.soundEnabled,
  });

  const onProjectChange = useCallback(
    (nextProjectId: number | null) => {
      setSelectedProjectId(nextProjectId);
      void timerSetContext({
        projectId: nextProjectId,
        tagIds: selectedTagIds,
      }).catch((error) => {
        reportActionError("Failed to update timer context.", error);
      });
    },
    [reportActionError, selectedTagIds],
  );

  const onTagToggle = useCallback(
    (tagId: number) => {
      const selected = selectedTagIds.includes(tagId);
      const nextTagIds = selected
        ? selectedTagIds.filter((id) => id !== tagId)
        : [...selectedTagIds, tagId];

      setSelectedTagIds(nextTagIds);
      void timerSetContext({
        projectId: selectedProjectId,
        tagIds: nextTagIds,
      }).catch((error) => {
        reportActionError("Failed to update timer context.", error);
      });
    },
    [reportActionError, selectedProjectId, selectedTagIds],
  );

  async function onToggleTimer() {
    if (!timer) {
      return;
    }

    setActionBusy(true);
    setStatusMessage("");

    try {
      let next: TimerState;

      if (timer.isRunning) {
        next = await timerPause();
        toast.success("Timer paused.", {
          position: "top-center",
          duration: 1500,
        });
      } else {
        if (timer.startedAt) {
          next = await timerResume({
            projectId: selectedProjectId,
            tagIds: selectedTagIds,
          });
        } else {
          next = await timerStart({
            projectId: selectedProjectId,
            tagIds: selectedTagIds,
          });
        }

        toast.success(
          next.phase === "focus"
            ? timer.startedAt
              ? "Timer resumed."
              : "Timer started."
            : "Break time started.",
          {
            description: `Current phase: ${phaseLabel(next.phase)}`,
            position: "top-center",
            duration: 1500,
          },
        );
      }

      setTimer(next);
    } catch (error) {
      reportActionError("Failed to update timer.", error);
    } finally {
      setActionBusy(false);
    }
  }

  async function onSkip() {
    setActionBusy(true);
    setStatusMessage("");

    try {
      const next = await timerSkip();
      setTimer(next);
      await refreshAll();
      toast.success("Timer skipped.", {
        description: `Next phase: ${phaseLabel(next.phase)}`,
        position: "top-center",
        duration: 1500,
      });
    } catch (error) {
      reportActionError("Failed to skip timer.", error);
    } finally {
      setActionBusy(false);
    }
  }

  async function onSaveSettings() {
    if (!settingsDraft) {
      return;
    }

    setStatusMessage("");

    try {
      const updated = await settingsUpdate(settingsDraft);
      setSettingsDraft(updated);
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      setStatusMessage("Settings saved.");
      toast.success("Settings saved.", {
        position: "top-center",
        duration: 1500,
      });
    } catch (error) {
      const details = toErrorMessage(error).toLowerCase();
      if (details.includes("remote control server")) {
        reportActionError(
          "Failed to save settings.",
          new Error(
            `Remote port ${settingsDraft.remoteControlPort} is unavailable. Choose another port and try again.`,
          ),
        );
        return;
      }
      reportActionError("Failed to save settings.", error);
    }
  }

  async function onAddProject() {
    const name = newProjectName.trim();
    if (!name) {
      toast.info("Project name is required.", {
        position: "top-center",
        duration: 1500,
      });
      return;
    }

    setStatusMessage("");
    try {
      await projectsUpsert({
        name,
        color: newProjectColor,
        archived: false,
      });
      setNewProjectName("");
      await queryClient.invalidateQueries({ queryKey: ["projects"] });
      toast.success("Project added.", {
        description: name,
        position: "top-center",
        duration: 1500,
      });
    } catch (error) {
      reportActionError("Failed to add project.", error);
    }
  }

  async function onAddTag() {
    const name = newTagName.trim();
    if (!name) {
      toast.info("Tag name is required.", {
        position: "top-center",
        duration: 1500,
      });
      return;
    }

    setStatusMessage("");
    try {
      await tagsUpsert({ name });
      setNewTagName("");
      await queryClient.invalidateQueries({ queryKey: ["tags"] });
      toast.success("Tag added.", {
        description: name,
        position: "top-center",
        duration: 1500,
      });
    } catch (error) {
      reportActionError("Failed to add tag.", error);
    }
  }

  async function onExportCsv() {
    setStatusMessage("");
    try {
      await exportCsv(historyRange);
      setStatusMessage("CSV export saved.");
      toast.success("CSV exported.", {
        position: "top-center",
        duration: 1500,
      });
    } catch (error) {
      reportActionError("Failed to export CSV.", error);
    }
  }

  async function onExportJson() {
    setStatusMessage("");
    try {
      await exportJson(historyRange);
      setStatusMessage("JSON export saved.");
      toast.success("JSON exported.", {
        position: "top-center",
        duration: 1500,
      });
    } catch (error) {
      reportActionError("Failed to export JSON.", error);
    }
  }

  async function onResetAllData() {
    const confirmed = await window.confirm(
      "This permanently deletes all sessions, projects, tags, and resets settings. Continue?",
    );
    if (!confirmed) {
      return;
    }

    setActionBusy(true);
    setStatusMessage("");

    try {
      const result = await resetAllData();
      setTimer(result.timer);
      setSettingsDraft(result.settings);
      setSelectedProjectId(result.timer.currentProjectId ?? null);
      setSelectedTagIds(result.timer.currentTagIds ?? []);
      await refreshAll();
      setStatusMessage("All app data has been reset.");
      toast.success("All app data has been reset.", {
        position: "top-center",
        duration: 1800,
      });
    } catch (error) {
      reportActionError("Failed to reset app data.", error);
    } finally {
      setActionBusy(false);
    }
  }

  const projectById = useMemo(() => {
    const map = new Map<number, string>();
    for (const project of projectsQuery.data ?? []) {
      map.set(project.id, project.name);
    }
    return map;
  }, [projectsQuery.data]);

  const contextLocked = Boolean(timer?.isRunning);

  return (
    <div className="flex h-screen w-full flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />

      <div className="flex flex-1 overflow-hidden">
        <Sidebar activeTab={activeTab} onChange={setActiveTab} />

        <main className="flex-1 overflow-auto p-6 bg-gradient-to-b from-background via-card to-card">
          <div className="mx-auto max-w-4xl space-y-6">
            {activeTab === "timer" && (
              <TimerTab
                timer={timer}
                summary={summaryQuery.data}
                projects={projectsQuery.data ?? []}
                tags={tagsQuery.data ?? []}
                selectedProjectId={selectedProjectId}
                selectedTagIds={selectedTagIds}
                contextLocked={contextLocked}
                actionBusy={actionBusy}
                onToggleTimer={onToggleTimer}
                onSkip={onSkip}
                onProjectChange={onProjectChange}
                onTagToggle={onTagToggle}
              />
            )}

            {activeTab === "stats" && (
              <StatsTab
                summary={summaryQuery.data}
                statsPeriod={statsPeriod}
                onStatsPeriodChange={setStatsPeriod}
                timeseriesData={seriesQuery.data ?? []}
                statsDayHistory={statsHistoryQuery.data ?? []}
                historyData={historyQuery.data ?? []}
                rangeDays={rangeDays}
                onRangeDaysChange={setRangeDays}
                onExportCsv={onExportCsv}
                onExportJson={onExportJson}
                getProjectName={(id) => (id ? projectById.get(id) ?? "Unknown" : "-")}
              />
            )}

            {activeTab === "settings" && (
              <SettingsTab
                settings={settingsDraft}
                onSettingsUpdate={setSettingsDraft}
                onSaveSettings={onSaveSettings}
                newProjectName={newProjectName}
                onNewProjectNameChange={setNewProjectName}
                newProjectColor={newProjectColor}
                onNewProjectColorChange={setNewProjectColor}
                newTagName={newTagName}
                onNewTagNameChange={setNewTagName}
                onAddProject={onAddProject}
                onAddTag={onAddTag}
                onResetAllData={onResetAllData}
                actionBusy={actionBusy}
              />
            )}
          </div>
        </main>
      </div>

      <footer className="border-t bg-muted/40 px-4 py-2 text-xs text-muted-foreground">
        <p>{statusMessage || "Ready to focus."}</p>
      </footer>
    </div>
  );
}
