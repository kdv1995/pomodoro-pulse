import { useEffect } from "react";
import type { AppSettings } from "@/types";

export function useThemeSync(settingsDraft: AppSettings | null) {
  useEffect(() => {
    const root = document.documentElement;
    const theme = settingsDraft?.theme || "light";

    const customTheme = settingsDraft?.customThemes?.find((candidate) => {
      return candidate.id === theme;
    });

    let styleEl = document.getElementById("custom-theme-style");
    if (!styleEl) {
      styleEl = document.createElement("style");
      styleEl.id = "custom-theme-style";
      document.head.appendChild(styleEl);
    }

    if (customTheme) {
      root.classList.remove("dark");
      styleEl.innerHTML = `
            :root {
                ${customTheme.cssVars}
            }
        `;
      root.style.colorScheme = "normal";
      return;
    }

    styleEl.innerHTML = "";
    root.classList.toggle("dark", theme === "dark");
    root.style.colorScheme = theme === "dark" ? "dark" : "light";
  }, [settingsDraft?.theme, settingsDraft?.customThemes]);
}
