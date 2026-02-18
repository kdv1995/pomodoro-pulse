
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Maximize, X, Minus } from "lucide-react";
import { useEffect } from "react";

export default function TitleBar() {

    useEffect(() => {
        // Optional: Listen for maximize events if needed to update state icon
        // For now we just toggle local stateoptimistically or check on mount
    }, []);

    const minimize = () => getCurrentWindow().minimize();
    const toggleMaximize = async () => {
        const win = getCurrentWindow();
        const max = await win.isMaximized();
        if (max) {
            win.unmaximize();
        } else {
            win.maximize();
        }
    };
    const close = () => getCurrentWindow().hide(); // Using hide as per plan/existing behavior

    return (
        <div data-tauri-drag-region className="flex h-9 w-full items-center justify-between border-b bg-background px-4 select-none">
            <div className="text-sm font-semibold">Pomodoro Pulse</div>
            <div className="flex items-center gap-1">
                <button
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md hover:bg-muted"
                    onClick={minimize}
                    aria-label="Minimize window"
                >
                    <Minus size={14} />
                </button>
                <button
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md hover:bg-muted"
                    onClick={toggleMaximize}
                    aria-label="Toggle maximize window"
                >
                    <Maximize size={14} />
                </button>
                <button
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md hover:bg-destructive hover:text-destructive-foreground"
                    onClick={close}
                    aria-label="Close window"
                >
                    <X size={14} />
                </button>
            </div>
        </div>
    );
}
