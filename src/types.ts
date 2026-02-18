export type TimerPhase = "focus" | "short_break" | "long_break";
export type AppTheme = "light" | "dark";

export interface AppSettings {
  focusMin: number;
  shortBreakMin: number;
  longBreakMin: number;
  longBreakEvery: number;
  theme: AppTheme;
  soundEnabled: boolean;
  notificationsEnabled: boolean;
  remoteControlEnabled: boolean;
  remoteControlPort: number;
  remoteControlToken: string;
}

export interface AppSettingsPatch {
  focusMin?: number;
  shortBreakMin?: number;
  longBreakMin?: number;
  longBreakEvery?: number;
  theme?: AppTheme;
  soundEnabled?: boolean;
  notificationsEnabled?: boolean;
  remoteControlEnabled?: boolean;
  remoteControlPort?: number;
  remoteControlToken?: string;
}

export interface TimerState {
  phase: TimerPhase;
  remainingSeconds: number;
  isRunning: boolean;
  cycleIndex: number;
  startedAt: number | null;
  phaseTotalSeconds: number;
  interruptions: number;
  currentProjectId: number | null;
  currentTagIds: number[];
  targetEndsAt: number | null;
}

export interface StartTimerRequest {
  projectId?: number | null;
  tagIds?: number[];
}

export interface SessionRecord {
  id: number;
  startedAt: number;
  endedAt: number;
  phase: TimerPhase;
  durationSec: number;
  completed: boolean;
  interruptions: number;
  projectId: number | null;
  tagIds: number[];
}

export interface AnalyticsRange {
  from?: number;
  to?: number;
  projectId?: number;
  tagId?: number;
}

export interface AnalyticsSummary {
  totalFocusSec: number;
  completedPomodoros: number;
  streakDays: number;
  interruptions: number;
  avgDailyFocusSec: number;
}

export interface TimeseriesPoint {
  date: string;
  focusSeconds: number;
  completedPomodoros: number;
  interruptions: number;
}

export interface Project {
  id: number;
  name: string;
  color?: string | null;
  archived: boolean;
}

export interface ProjectInput {
  id?: number;
  name: string;
  color?: string | null;
  archived?: boolean;
}

export interface Tag {
  id: number;
  name: string;
}

export interface TagInput {
  id?: number;
  name: string;
}

export interface ExportRange {
  from?: number;
  to?: number;
}

export interface ExportResult {
  filename: string;
  content: string;
}

export interface ResetAllResult {
  settings: AppSettings;
  timer: TimerState;
}

export interface PhaseCompletedEvent {
  completedPhase: TimerPhase;
  nextPhase: TimerPhase;
}
