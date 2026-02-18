import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  analyticsGetSummary,
  analyticsGetTimeseries,
  exportCsv,
  exportJson,
  projectsList,
  projectsUpsert,
  // sessionHistory, // Using historyQuery still
  sessionHistory,
  settingsGet,
  settingsUpdate,
  resetAllData,
  tagsList,
  tagsUpsert,
  timerGetState,
  timerPause,
  timerResume,
  timerSkip,
  timerStart,
  timerSetContext,
} from "./api";
import type {
  AnalyticsRange,
  AppSettings,
  PhaseCompletedEvent,
  SessionRecord,
  TimerState,
} from "./types";
import "./App.css";

// Components
import TimerDisplay from "./components/TimerDisplay";
import Controls from "./components/Controls";
import StatsOverview from "./components/StatsOverview";
import SettingsPanel from "./components/SettingsPanel";
import HistoryList from "./components/HistoryList";
import TitleBar from "./components/TitleBar";
import Sidebar from "./components/Sidebar";

const StatsChart = lazy(() => import("./components/StatsChart"));
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const DAY_SECONDS = 86_400;

export default function App() {
  const queryClient = useQueryClient();
  const audioContextRef = useRef<AudioContext | null>(null);
  const contextDirtyRef = useRef(false);

  const [timer, setTimer] = useState<TimerState | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<number | null>(null);
  const [selectedTagIds, setSelectedTagIds] = useState<number[]>([]);
  const [filterProjectId] = useState<number | undefined>();
  const [filterTagId] = useState<number | undefined>();
  const [settingsDraft, setSettingsDraft] = useState<AppSettings | null>(null);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectColor, setNewProjectColor] = useState("#f97316");
  const [newTagName, setNewTagName] = useState("");
  const [statusMessage, setStatusMessage] = useState("");
  const [actionBusy, setActionBusy] = useState(false);
  const [contextDirty, setContextDirty] = useState(false);

  // Navigation State
  const [activeTab, setActiveTab] = useState<"timer" | "stats" | "settings">("timer");
  const [statsPeriod, setStatsPeriod] = useState<"day" | "week" | "month">("week");

  const analyticsRange: AnalyticsRange = useMemo(() => {
    const to = Math.floor(Date.now() / 1000);
    let days = 7;
    if (activeTab === "stats") {
      if (statsPeriod === "day") days = 1;
      else if (statsPeriod === "week") days = 7;
      else if (statsPeriod === "month") days = 30;
    }

    const from = to - days * DAY_SECONDS;
    return {
      from,
      to,
      projectId: filterProjectId,
      tagId: filterTagId,
    };
  }, [filterProjectId, filterTagId, statsPeriod, activeTab]);

  const settingsQuery = useQuery({
    queryKey: ["settings"],
    queryFn: settingsGet,
  });

  const projectsQuery = useQuery({
    queryKey: ["projects"],
    queryFn: projectsList,
  });

  const tagsQuery = useQuery({
    queryKey: ["tags"],
    queryFn: tagsList,
  });

  const summaryQuery = useQuery({
    queryKey: ["summary", analyticsRange],
    queryFn: () => analyticsGetSummary(analyticsRange),
  });

  // Now capturing the data for the chart
  const seriesQuery = useQuery({
    queryKey: ["series", analyticsRange],
    queryFn: () => analyticsGetTimeseries(analyticsRange),
  });

  const historyQuery = useQuery({
    queryKey: ["history", analyticsRange],
    queryFn: () => sessionHistory(analyticsRange),
  });

  useEffect(() => {
    if (settingsQuery.data) {
      setSettingsDraft(settingsQuery.data);
    }
  }, [settingsQuery.data]);

  useEffect(() => {
    contextDirtyRef.current = contextDirty;
  }, [contextDirty]);

  useEffect(() => {
    // Request notifications permission proactively (Rust emits system notifications).
    if (!settingsDraft?.notificationsEnabled) {
      return;
    }
    (async () => {
      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      return granted;
    })().catch(() => {});
  }, [settingsDraft?.notificationsEnabled]);

  useEffect(() => {
    timerGetState()
      .then((nextState) => {
        setTimer(nextState);
        setSelectedProjectId(nextState.currentProjectId ?? null);
        setSelectedTagIds(nextState.currentTagIds ?? []);
      })
      .catch((error) => {
        setStatusMessage(String(error));
      });
  }, []);

  // Update body attribute for theme theming
  useEffect(() => {
    if (timer?.phase) {
      document.body.setAttribute("data-phase", timer.phase);
    }
  }, [timer?.phase]);

  useEffect(() => {
    let unlistenState: (() => void) | undefined;
    let unlistenPhase: (() => void) | undefined;
    let unlistenSession: (() => void) | undefined;
    // Window close is now handled by TitleBar, but we can keep listener if needed for other things?
    // Actually standard close behavior is fine since we use hide() in TitleBar.

    async function setupListeners() {
      unlistenState = await listen<TimerState>("timer://state", (event) => {
        setTimer(event.payload);
        if (!contextDirtyRef.current || event.payload.isRunning) {
          setSelectedProjectId(event.payload.currentProjectId ?? null);
          setSelectedTagIds(event.payload.currentTagIds ?? []);
        }
      });

      unlistenPhase = await listen<PhaseCompletedEvent>(
        "timer://phase-completed",
        async () => {
          if (settingsDraft?.soundEnabled) {
            try {
              let ctx = audioContextRef.current;
              if (!ctx || ctx.state === "closed") {
                ctx = new AudioContext();
                audioContextRef.current = ctx;
              }
              const oscillator = ctx.createOscillator();
              const gain = ctx.createGain();
              oscillator.connect(gain);
              gain.connect(ctx.destination);
              oscillator.type = "triangle";
              oscillator.frequency.value = 880;
              gain.gain.setValueAtTime(0.15, ctx.currentTime);
              gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.4);
              oscillator.start();
              oscillator.stop(ctx.currentTime + 0.4);
            } catch {
              // Ignore audio failures (autoplay restrictions, etc).
            }
          }

          queryClient.invalidateQueries({ queryKey: ["summary"] });
          queryClient.invalidateQueries({ queryKey: ["series"] });
          queryClient.invalidateQueries({ queryKey: ["history"] });
        },
      );

      unlistenSession = await listen<SessionRecord>("session://completed", () => {
        queryClient.invalidateQueries({ queryKey: ["summary"] });
        queryClient.invalidateQueries({ queryKey: ["series"] });
        queryClient.invalidateQueries({ queryKey: ["history"] });
      });
    }

    setupListeners().catch((error) => {
      setStatusMessage(String(error));
    });

    return () => {
      unlistenState?.();
      unlistenPhase?.();
      unlistenSession?.();
      audioContextRef.current?.close().catch(() => {});
      audioContextRef.current = null;
    };
  }, [queryClient, settingsDraft?.notificationsEnabled, settingsDraft?.soundEnabled]);

  async function refreshAll() {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["summary"] }),
      queryClient.invalidateQueries({ queryKey: ["series"] }),
      queryClient.invalidateQueries({ queryKey: ["history"] }),
      queryClient.invalidateQueries({ queryKey: ["projects"] }),
      queryClient.invalidateQueries({ queryKey: ["tags"] }),
      queryClient.invalidateQueries({ queryKey: ["settings"] }),
    ]);
  }

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
      } else if (timer.startedAt) {
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

      setTimer(next);
    } catch (error) {
      setStatusMessage(String(error));
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
    } catch (error) {
      setStatusMessage(String(error));
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
    } catch (error) {
      setStatusMessage(String(error));
    }
  }

  async function onAddProject() {
    const name = newProjectName.trim();
    if (!name) {
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
    } catch (error) {
      setStatusMessage(String(error));
    }
  }

  async function onAddTag() {
    const name = newTagName.trim();
    if (!name) {
      return;
    }

    setStatusMessage("");
    try {
      await tagsUpsert({ name });
      setNewTagName("");
      await queryClient.invalidateQueries({ queryKey: ["tags"] });
    } catch (error) {
      setStatusMessage(String(error));
    }
  }

  async function onExportCsv() {
    setStatusMessage("");
    try {
      await exportCsv(analyticsRange);
      setStatusMessage("CSV export saved.");
    } catch (error) {
      setStatusMessage(String(error));
    }
  }

  async function onExportJson() {
    setStatusMessage("");
    try {
      await exportJson(analyticsRange);
      setStatusMessage("JSON export saved.");
    } catch (error) {
      setStatusMessage(String(error));
    }
  }

  async function onResetAllData() {
    const confirmed = window.confirm(
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
    } catch (error) {
      setStatusMessage(String(error));
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

  // Lock context only while the timer is actively running.
  // Otherwise a previously-started (but paused) session would prevent selecting a project.
  const contextLocked = Boolean(timer?.isRunning);

  return (
    <div className="flex h-screen w-full flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />

      <div className="flex flex-1 overflow-hidden">
        <Sidebar activeTab={activeTab} onChange={setActiveTab} />

        <main className="flex-1 overflow-auto p-6 bg-gradient-to-b from-background via-card to-card">
          <div className="mx-auto max-w-4xl space-y-6">
            {activeTab === "timer" && (
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

                  {/* Task Selection */}
                  <div className="grid gap-4 sm:grid-cols-2">
                    <label className="flex flex-col gap-2">
                      <span className="text-sm font-medium">Focus Project</span>
                      <Select
                        value={selectedProjectId === null ? "no-project" : selectedProjectId.toString()}
                        onValueChange={(value) => {
                          const next = value === "no-project" ? null : Number(value);
                          setSelectedProjectId(next);
                          setContextDirty(true);
                          void timerSetContext({ projectId: next, tagIds: selectedTagIds })
                            .then(() => setContextDirty(false))
                            .catch((error) => {
                              setContextDirty(false);
                              setStatusMessage(String(error));
                            });
                        }}
                        disabled={contextLocked}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder="Select a project" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="no-project">No Project</SelectItem>
                          {(projectsQuery.data ?? []).map((project) => (
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
                        {(tagsQuery.data ?? []).map((tag) => {
                          const selected = selectedTagIds.includes(tag.id);
                          return (
                            <button
                              type="button"
                              key={tag.id}
                              disabled={contextLocked}
                              className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ${selected
                                ? "border-transparent bg-primary text-primary-foreground hover:bg-primary/80"
                                : "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80"
                                }`}
                              onClick={() => {
                                if (selected) {
                                  const next = selectedTagIds.filter((id) => id !== tag.id);
                                  setSelectedTagIds(next);
                                  setContextDirty(true);
                                  void timerSetContext({ projectId: selectedProjectId, tagIds: next })
                                    .then(() => setContextDirty(false))
                                    .catch((error) => {
                                      setContextDirty(false);
                                      setStatusMessage(String(error));
                                    });
                                } else {
                                  const next = [...selectedTagIds, tag.id];
                                  setSelectedTagIds(next);
                                  setContextDirty(true);
                                  void timerSetContext({ projectId: selectedProjectId, tagIds: next })
                                    .then(() => setContextDirty(false))
                                    .catch((error) => {
                                      setContextDirty(false);
                                      setStatusMessage(String(error));
                                    });
                                }
                              }}
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
                  <StatsOverview summary={summaryQuery.data} />
                </div>
              </div>
            )}

            {activeTab === "stats" && (
              <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-6">
                <h2 className="text-2xl font-bold tracking-tight">Statistics</h2>
                <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
                  <div className="p-6">
                    <StatsOverview summary={summaryQuery.data} />
                  </div>
                </div>

                <Suspense fallback={<div className="rounded-xl border bg-card p-6">Loading chart...</div>}>
                  <StatsChart
                    period={statsPeriod}
                    onPeriodChange={setStatsPeriod}
                    timeseriesData={seriesQuery.data ?? []}
                    sessionData={historyQuery.data ?? []}
                  />
                </Suspense>

                <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
                  <div className="flex flex-row items-center justify-between p-6 pb-2">
                    <h3 className="text-lg font-semibold leading-none tracking-tight">History</h3>
                    <div className="flex items-center gap-2">
                      <div className="flex gap-1">
                        <button className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-9 w-9" onClick={onExportCsv} title="Export CSV">📄</button>
                        <button className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-9 px-3" onClick={onExportJson} title="Export JSON">{ } JSON</button>
                      </div>
                    </div>
                  </div>
                  <div className="p-6 pt-0">
                    <HistoryList
                      history={historyQuery.data ?? []}
                      getProjectName={(id) => (id ? projectById.get(id) ?? "Unknown" : "—")}
                    />
                  </div>
                </div>
              </div>
            )}

            {activeTab === "settings" && (
              <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-6">
                <h2 className="text-2xl font-bold tracking-tight">Settings</h2>
                <SettingsPanel
                  settings={settingsDraft}
                  onUpdate={setSettingsDraft}
                  onSave={onSaveSettings}
                />

                <div className="rounded-xl border bg-card text-card-foreground shadow-sm">
                  <div className="flex flex-col space-y-1.5 p-6">
                    <h3 className="text-lg font-semibold leading-none tracking-tight">Manage Projects & Tags</h3>
                  </div>
                  <div className="p-6 pt-0 space-y-4">
                    <div className="space-y-2">
                      <h4 className="text-sm font-medium leading-none">Add Project</h4>
                      <div className="flex gap-2">
                        <Input
                          value={newProjectName}
                          placeholder="Project Name"
                          onChange={(event) => setNewProjectName(event.currentTarget.value)}
                        />
                        <input
                          type="color"
                          className="h-10 w-12 rounded-md border border-input bg-background p-1 cursor-pointer"
                          value={newProjectColor}
                          onChange={(event) => setNewProjectColor(event.currentTarget.value)}
                        />
                        <Button variant="secondary" onClick={onAddProject}>Add</Button>
                      </div>
                    </div>

                    <div className="space-y-2">
                      <h4 className="text-sm font-medium leading-none">Add Tag</h4>
                      <div className="flex gap-2">
                        <Input
                          value={newTagName}
                          placeholder="Tag Name"
                          onChange={(event) => setNewTagName(event.currentTarget.value)}
                        />
                        <Button variant="secondary" onClick={onAddTag}>Add</Button>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="rounded-xl border border-destructive/20 bg-destructive/10 text-destructive shadow-sm">
                  <div className="flex flex-col space-y-1.5 p-6">
                    <h4 className="text-lg font-semibold leading-none tracking-tight">Danger Zone</h4>
                    <p className="text-sm text-muted-foreground">Delete all sessions, projects, tags, and restore default settings.</p>
                  </div>
                  <div className="p-6 pt-0">
                    <button className="inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 h-9 px-4 py-2" onClick={onResetAllData} disabled={actionBusy}>
                      {actionBusy ? "Working..." : "Reset Everything"}
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </main>
      </div>

      <footer className="border-t bg-muted/40 px-4 py-2 text-xs text-muted-foreground">
        <p role="status" aria-live="polite">{statusMessage || "Ready to focus."}</p>
      </footer>
    </div>
  );
}
