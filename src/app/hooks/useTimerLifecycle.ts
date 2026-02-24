import { useEffect, type Dispatch, type SetStateAction } from "react";
import { listen } from "@tauri-apps/api/event";
import { QueryClient } from "@tanstack/react-query";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

import { timerGetState } from "@/api";
import type { PhaseCompletedEvent, SessionRecord, TimerState } from "@/types";
import { phaseLabel, playTone } from "@/app/utils/timer";

interface UseTimerLifecycleOptions {
  timer: TimerState | null;
  queryClient: QueryClient;
  setTimer: Dispatch<SetStateAction<TimerState | null>>;
  setSelectedProjectId: (projectId: number | null) => void;
  setSelectedTagIds: (tagIds: number[]) => void;
  onError: (title: string, error: unknown) => void;
  notificationsEnabled?: boolean;
  soundEnabled?: boolean;
}

async function refreshAnalytics(queryClient: QueryClient) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: ["summary"] }),
    queryClient.invalidateQueries({ queryKey: ["series"] }),
    queryClient.invalidateQueries({ queryKey: ["history"] }),
    queryClient.invalidateQueries({ queryKey: ["history-stats"] }),
  ]);
}

export function useTimerLifecycle({
  timer,
  queryClient,
  setTimer,
  setSelectedProjectId,
  setSelectedTagIds,
  onError,
  notificationsEnabled,
  soundEnabled,
}: UseTimerLifecycleOptions) {
  useEffect(() => {
    timerGetState()
      .then((nextState) => {
        setTimer(nextState);
        setSelectedProjectId(nextState.currentProjectId ?? null);
        setSelectedTagIds(nextState.currentTagIds ?? []);
      })
      .catch((error) => {
        onError("Failed to load timer state.", error);
      });
  }, [onError, setSelectedProjectId, setSelectedTagIds, setTimer]);

  useEffect(() => {
    if (timer?.phase) {
      document.body.setAttribute("data-phase", timer.phase);
    }
  }, [timer?.phase]);

  useEffect(() => {
    if (!timer?.isRunning || !timer.targetEndsAt) {
      return;
    }

    const updateRemaining = () => {
      const now = Math.floor(Date.now() / 1000);
      setTimer((current) => {
        if (!current || !current.isRunning || !current.targetEndsAt) {
          return current;
        }
        const nextRemaining = Math.max(0, current.targetEndsAt - now);
        if (nextRemaining === current.remainingSeconds) {
          return current;
        }
        return { ...current, remainingSeconds: nextRemaining };
      });
    };

    updateRemaining();
    const intervalId = window.setInterval(updateRemaining, 250);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [setTimer, timer?.isRunning, timer?.targetEndsAt]);

  useEffect(() => {
    let unlistenState: (() => void) | undefined;
    let unlistenPhase: (() => void) | undefined;
    let unlistenSession: (() => void) | undefined;

    async function setupListeners() {
      unlistenState = await listen<TimerState>("timer://state", (event) => {
        setTimer(event.payload);
      });

      unlistenPhase = await listen<PhaseCompletedEvent>(
        "timer://phase-completed",
        async (event) => {
          if (notificationsEnabled) {
            let granted = await isPermissionGranted();
            if (!granted) {
              granted = (await requestPermission()) === "granted";
            }
            if (granted) {
              await sendNotification({
                title: "Pomodoro update",
                body: `${phaseLabel(event.payload.completedPhase)} complete. Next ${phaseLabel(
                  event.payload.nextPhase,
                )}.`,
              });
            }
          }
          if (soundEnabled) {
            playTone();
          }
          await refreshAnalytics(queryClient);
        },
      );

      unlistenSession = await listen<SessionRecord>("session://completed", () => {
        void refreshAnalytics(queryClient);
      });
    }

    setupListeners().catch((error) => {
      onError("Failed to subscribe to timer events.", error);
    });

    return () => {
      unlistenState?.();
      unlistenPhase?.();
      unlistenSession?.();
    };
  }, [notificationsEnabled, onError, queryClient, setTimer, soundEnabled]);
}
