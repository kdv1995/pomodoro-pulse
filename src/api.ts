import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import type {
  AnalyticsRange,
  AnalyticsSummary,
  AppSettings,
  AppSettingsPatch,
  ExportRange,
  ExportResult,
  Project,
  ProjectInput,
  ResetAllResult,
  SessionRecord,
  StartTimerRequest,
  Tag,
  TagInput,
  TimerState,
  TimeseriesPoint,
} from "./types";

export async function timerStart(payload?: StartTimerRequest) {
  return invoke<TimerState>("timer_start", { payload });
}

export async function timerPause() {
  return invoke<TimerState>("timer_pause");
}

export async function timerResume(payload?: StartTimerRequest) {
  return invoke<TimerState>("timer_resume", { payload });
}

export async function timerSkip() {
  return invoke<TimerState>("timer_skip");
}

export async function timerGetState() {
  return invoke<TimerState>("timer_get_state");
}

export async function timerSetContext(payload: StartTimerRequest) {
  return invoke<TimerState>("timer_set_context", { payload });
}

export async function analyticsGetSummary(range: AnalyticsRange) {
  return invoke<AnalyticsSummary>("analytics_get_summary", { range });
}

export async function analyticsGetTimeseries(range: AnalyticsRange) {
  return invoke<TimeseriesPoint[]>("analytics_get_timeseries", { range });
}

export async function sessionHistory(range: AnalyticsRange) {
  return invoke<SessionRecord[]>("session_history", { range });
}

export async function projectsList() {
  return invoke<Project[]>("projects_list");
}

export async function projectsUpsert(input: ProjectInput) {
  return invoke<Project>("projects_upsert", { input });
}

export async function tagsList() {
  return invoke<Tag[]>("tags_list");
}

export async function tagsUpsert(input: TagInput) {
  return invoke<Tag>("tags_upsert", { input });
}

export async function settingsGet() {
  return invoke<AppSettings>("settings_get");
}

export async function settingsUpdate(patch: AppSettingsPatch) {
  return invoke<AppSettings>("settings_update", { patch });
}

export async function resetAllData() {
  return invoke<ResetAllResult>("reset_all_data");
}

async function writeExport(
  command: "export_csv" | "export_json",
  range: ExportRange,
  extension: "csv" | "json",
  filterName: string,
) {
  const file = await invoke<ExportResult>(command, { range });
  const path = await save({
    defaultPath: file.filename,
    filters: [{ name: filterName, extensions: [extension] }],
  });

  if (!path) {
    return;
  }

  await writeTextFile(path, file.content);
}

export async function exportCsv(range: ExportRange) {
  await writeExport("export_csv", range, "csv", "CSV");
}

export async function exportJson(range: ExportRange) {
  await writeExport("export_json", range, "json", "JSON");
}
