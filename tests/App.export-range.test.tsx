import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const { exportCsvMock, exportJsonMock, baseSettings, baseTimer } = vi.hoisted(
  () => ({
    exportCsvMock: vi.fn().mockResolvedValue(undefined),
    exportJsonMock: vi.fn().mockResolvedValue(undefined),
    baseSettings: {
      focusMin: 25,
      shortBreakMin: 5,
      longBreakMin: 15,
      longBreakEvery: 4,
      theme: "light",
      soundEnabled: true,
      notificationsEnabled: true,
      remoteControlEnabled: false,
      remoteControlPort: 48484,
      remoteControlToken: "token",
    },
    baseTimer: {
      phase: "focus",
      remainingSeconds: 1500,
      isRunning: false,
      cycleIndex: 0,
      startedAt: null,
      phaseTotalSeconds: 1500,
      interruptions: 0,
      currentProjectId: null,
      currentTagIds: [],
      targetEndsAt: null,
    },
  }),
);

vi.mock("@/api", () => ({
  analyticsGetSummary: vi.fn().mockResolvedValue({
    totalFocusSec: 0,
    completedPomodoros: 0,
    streakDays: 0,
    interruptions: 0,
    avgDailyFocusSec: 0,
  }),
  analyticsGetTimeseries: vi.fn().mockResolvedValue([]),
  exportCsv: exportCsvMock,
  exportJson: exportJsonMock,
  projectsList: vi.fn().mockResolvedValue([]),
  projectsUpsert: vi.fn().mockResolvedValue({ id: 1, name: "x", archived: false }),
  sessionHistory: vi.fn().mockResolvedValue([]),
  settingsGet: vi.fn().mockResolvedValue(baseSettings),
  settingsUpdate: vi.fn().mockResolvedValue(baseSettings),
  resetAllData: vi.fn().mockResolvedValue({
    settings: baseSettings,
    timer: baseTimer,
  }),
  tagsList: vi.fn().mockResolvedValue([]),
  tagsUpsert: vi.fn().mockResolvedValue({ id: 1, name: "x" }),
  timerGetState: vi.fn().mockResolvedValue(baseTimer),
  timerPause: vi.fn().mockResolvedValue(baseTimer),
  timerResume: vi.fn().mockResolvedValue(baseTimer),
  timerSkip: vi.fn().mockResolvedValue(baseTimer),
  timerStart: vi.fn().mockResolvedValue(baseTimer),
  timerSetContext: vi.fn().mockResolvedValue(baseTimer),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue("granted"),
  sendNotification: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock("@/components/TimerDisplay", () => ({
  default: () => <div>timer-display</div>,
}));

vi.mock("@/components/Controls", () => ({
  default: () => <div>controls</div>,
}));

vi.mock("@/components/StatsOverview", () => ({
  default: () => <div>stats-overview</div>,
}));

vi.mock("@/components/SettingsPanel", () => ({
  default: () => <div>settings-panel</div>,
}));

vi.mock("@/components/HistoryList", () => ({
  default: () => <div>history-list</div>,
}));

vi.mock("@/components/TitleBar", () => ({
  default: () => <div>title-bar</div>,
}));

vi.mock("@/components/Sidebar", () => ({
  default: ({ onChange }: { onChange: (tab: "timer" | "stats" | "settings") => void }) => (
    <button type="button" onClick={() => onChange("stats")}>
      Open Stats
    </button>
  ),
}));

vi.mock("@/components/StatsChart", () => ({
  default: ({ onPeriodChange }: { onPeriodChange: (period: "day" | "week" | "month") => void }) => (
    <button type="button" onClick={() => onPeriodChange("day")}>
      Set Day Period
    </button>
  ),
}));

vi.mock("@/components/ui/select", async () => {
  const ReactModule = await import("react");
  const SelectContext = ReactModule.createContext<
    | {
        onValueChange?: (value: string) => void;
      }
    | undefined
  >(undefined);

  function Select({
    onValueChange,
    children,
  }: {
    onValueChange?: (value: string) => void;
    children: React.ReactNode;
  }) {
    return (
      <SelectContext.Provider value={{ onValueChange }}>
        <div>{children}</div>
      </SelectContext.Provider>
    );
  }

  function SelectTrigger({ children }: { children: React.ReactNode }) {
    return <div>{children}</div>;
  }

  function SelectValue({ placeholder }: { placeholder?: string }) {
    return <span>{placeholder ?? ""}</span>;
  }

  function SelectContent({ children }: { children: React.ReactNode }) {
    return <div>{children}</div>;
  }

  function SelectItem({
    value,
    children,
  }: {
    value: string;
    children: React.ReactNode;
  }) {
    const context = ReactModule.useContext(SelectContext);
    return (
      <button type="button" onClick={() => context?.onValueChange?.(value)}>
        {children}
      </button>
    );
  }

  return {
    Select,
    SelectTrigger,
    SelectValue,
    SelectContent,
    SelectItem,
  };
});

import App from "@/App";

describe("App export range", () => {
  let dateNowSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    dateNowSpy = vi
      .spyOn(Date, "now")
      .mockReturnValue(new Date("2026-02-19T12:00:00.000Z").getTime());
    exportCsvMock.mockClear();
    exportJsonMock.mockClear();
  });

  afterEach(() => {
    dateNowSpy.mockRestore();
  });

  it("keeps export range tied to history range when stats period changes", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    render(
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Open Stats" }));
    fireEvent.click(await screen.findByRole("button", { name: "Set Day Period" }));
    fireEvent.click(await screen.findByTitle("Export CSV"));

    await waitFor(() => expect(exportCsvMock).toHaveBeenCalledTimes(1));

    const range = exportCsvMock.mock.calls[0][0] as { from: number; to: number };
    expect(range.to - range.from).toBe(14 * 86_400);
    expect(range.to - range.from).not.toBe(86_400);
  });
});
