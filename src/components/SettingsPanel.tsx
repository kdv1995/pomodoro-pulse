import { useEffect, useState } from "react";
import { AppSettings } from "../types";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { invoke } from "@tauri-apps/api/core";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { toast } from "sonner";
import CliIntegrationCard from "@/components/CliIntegrationCard";

function hexToHsl(hex: string): string {
    let r = parseInt(hex.slice(1, 3), 16) / 255;
    let g = parseInt(hex.slice(3, 5), 16) / 255;
    let b = parseInt(hex.slice(5, 7), 16) / 255;
    let max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h = 0, s = 0, l = (max + min) / 2;
    if (max !== min) {
        let d = max - min;
        s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
        switch (max) {
            case r: h = (g - b) / d + (g < b ? 6 : 0); break;
            case g: h = (b - r) / d + 2; break;
            case b: h = (r - g) / d + 4; break;
        }
        h /= 6;
    }
    return `${(h * 360).toFixed(1)} ${(s * 100).toFixed(1)}% ${(l * 100).toFixed(1)}%`;
}

interface SettingsPanelProps {
    settings: AppSettings | null;
    onUpdate: (newSettings: AppSettings) => void;
    onSave: () => void;
}

export default function SettingsPanel({ settings, onUpdate, onSave }: SettingsPanelProps) {
    const [localIp, setLocalIp] = useState("YOUR_LOCAL_IP");

    const [editThemeId, setEditThemeId] = useState<string | null>(null);
    const [newThemeName, setNewThemeName] = useState("");
    const [newThemeBg, setNewThemeBg] = useState("#000000");
    const [newThemeFg, setNewThemeFg] = useState("#ffffff");
    const [newThemePrimary, setNewThemePrimary] = useState("#0071e3");
    const [newThemeMutedFg, setNewThemeMutedFg] = useState("#86868b");
    const [newThemeAccent, setNewThemeAccent] = useState("#27272a");
    const [newThemeBorder, setNewThemeBorder] = useState("#3f3f46");

    if (!settings) return null;

    useEffect(() => {
        let active = true;

        invoke<string>("get_local_ip")
            .then((ip) => {
                if (active && ip) {
                    setLocalIp(ip);
                }
            })
            .catch(() => {
                if (active) {
                    setLocalIp("YOUR_LOCAL_IP");
                }
            });

        return () => {
            active = false;
        };
    }, []);

    const handleChange = (field: keyof AppSettings, value: number | boolean | string) => {
        onUpdate({ ...settings, [field]: value });
    };

    const remoteUrl = `http://${localIp}:${settings.remoteControlPort}/?token=${settings.remoteControlToken}`;

    const handleCopyIP = async () => {
        if (!settings.remoteControlEnabled) return;

        try {
            await navigator.clipboard.writeText(remoteUrl);
            toast.success("Remote control URL copied to clipboard!", {
                position: "top-center",
            });
        } catch {
            toast.error("Failed to copy remote control URL.", {
                position: "top-center",
            });
        }
    };

    const handleAddCustomTheme = () => {
        if (!newThemeName.trim()) {
            toast.error("Theme name is required.");
            return;
        }

        const cssVars = `
          --background: ${hexToHsl(newThemeBg)};
          --foreground: ${hexToHsl(newThemeFg)};
          --card: ${hexToHsl(newThemeBg)};
          --card-foreground: ${hexToHsl(newThemeFg)};
          --popover: ${hexToHsl(newThemeBg)};
          --popover-foreground: ${hexToHsl(newThemeFg)};
          --primary: ${hexToHsl(newThemePrimary)};
          --primary-foreground: ${hexToHsl(newThemeBg)};
          --secondary: ${hexToHsl(newThemeAccent)};
          --secondary-foreground: ${hexToHsl(newThemeFg)};
          --muted: ${hexToHsl(newThemeAccent)};
          --muted-foreground: ${hexToHsl(newThemeMutedFg)};
          --accent: ${hexToHsl(newThemeAccent)};
          --accent-foreground: ${hexToHsl(newThemeFg)};
          --destructive: 3 100% 50%;
          --destructive-foreground: ${hexToHsl(newThemeFg)};
          --border: ${hexToHsl(newThemeBorder)};
          --input: ${hexToHsl(newThemeBorder)};
          --ring: ${hexToHsl(newThemePrimary)};
        `;

        const newTheme = {
            id: editThemeId || 'custom-' + Date.now().toString(),
            name: newThemeName.trim(),
            cssVars: cssVars,
            colors: {
                bg: newThemeBg,
                fg: newThemeFg,
                primary: newThemePrimary,
                mutedFg: newThemeMutedFg,
                accent: newThemeAccent,
                border: newThemeBorder,
            }
        };

        const customThemes = settings.customThemes || [];

        if (editThemeId) {
            onUpdate({
                ...settings,
                customThemes: customThemes.map(t => t.id === editThemeId ? newTheme : t),
                theme: settings.theme === editThemeId ? newTheme.id : settings.theme
            });
            setEditThemeId(null);
            toast.success("Custom theme updated!", { position: "top-center" });
        } else {
            onUpdate({
                ...settings,
                customThemes: [...customThemes, newTheme],
                theme: newTheme.id
            });
            toast.success("Custom theme created and applied!", { position: "top-center" });
        }
        setNewThemeName("");
    };

    const handleEditTheme = (t: any, e: React.MouseEvent) => {
        e.stopPropagation();
        setEditThemeId(t.id);
        setNewThemeName(t.name);
        setNewThemeBg(t.colors?.bg ?? "#000000");
        setNewThemeFg(t.colors?.fg ?? "#ffffff");
        setNewThemePrimary(t.colors?.primary ?? "#0071e3");
        setNewThemeMutedFg(t.colors?.mutedFg ?? "#86868b");
        setNewThemeAccent(t.colors?.accent ?? "#27272a");
        setNewThemeBorder(t.colors?.border ?? "#3f3f46");
    };

    const handleDeleteTheme = (id: string, e: React.MouseEvent) => {
        e.stopPropagation();
        const customThemes = settings.customThemes || [];
        onUpdate({
            ...settings,
            customThemes: customThemes.filter(t => t.id !== id),
            theme: settings.theme === id ? "light" : settings.theme
        });
    };


    return (
        <Card>
            <CardHeader>
                <CardTitle>Timer Settings</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6">
                <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                        <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                            Focus (min)
                        </label>
                        <Input
                            type="number"
                            min={1}
                            max={180}
                            value={settings.focusMin}
                            onChange={(e) => handleChange("focusMin", Number(e.target.value))}
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                            Short Break (min)
                        </label>
                        <Input
                            type="number"
                            min={1}
                            max={60}
                            value={settings.shortBreakMin}
                            onChange={(e) => handleChange("shortBreakMin", Number(e.target.value))}
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                            Long Break (min)
                        </label>
                        <Input
                            type="number"
                            min={1}
                            max={90}
                            value={settings.longBreakMin}
                            onChange={(e) => handleChange("longBreakMin", Number(e.target.value))}
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                            Long Break Interval
                        </label>
                        <Input
                            type="number"
                            min={2}
                            max={10}
                            value={settings.longBreakEvery}
                            onChange={(e) => handleChange("longBreakEvery", Number(e.target.value))}
                        />
                    </div>
                </div>

                <div className="space-y-4">
                    <div className="flex flex-col gap-3 rounded-lg border p-3 shadow-sm">
                        <div className="space-y-0.5">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Application Theme
                            </label>
                            <p className="text-xs text-muted-foreground">
                                Select light, dark, or one of your custom themes.
                            </p>
                        </div>
                        <Select
                            value={settings.theme || "light"}
                            onValueChange={(val) => handleChange("theme", val)}
                        >
                            <SelectTrigger>
                                <SelectValue placeholder="Select a theme" />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="light">Light Default</SelectItem>
                                <SelectItem value="dark">Dark Default</SelectItem>
                                {settings.customThemes?.map(t => (
                                    <SelectItem key={t.id} value={t.id}>
                                        <div className="flex items-center justify-between w-full">
                                            <span>{t.name}</span>
                                        </div>
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>

                        {(settings.customThemes?.length ?? 0) > 0 && (
                            <div className="flex flex-col gap-2 mt-2 pt-2">
                                <span className="text-xs font-semibold">Saved Custom Themes:</span>
                                {settings.customThemes?.map(t => (
                                    <div key={t.id} className="flex justify-between items-center text-sm border px-2 py-1 rounded">
                                        <span>{t.name}</span>
                                        <div className="flex gap-2">
                                            <button className="text-primary hover:underline text-xs" onClick={(e) => handleEditTheme(t, e)}>Edit</button>
                                            <button className="text-destructive hover:underline text-xs" onClick={(e) => handleDeleteTheme(t.id, e)}>Delete</button>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )}

                        <div className="flex flex-col gap-2 mt-4 pt-4">
                            <span className="text-xs font-semibold">{editThemeId ? "Edit Theme" : "Create New Theme"}</span>
                            <Input placeholder="Theme Name" value={newThemeName} onChange={e => setNewThemeName(e.target.value)} />
                            <div className="grid grid-cols-2 lg:grid-cols-3 gap-2 text-xs mt-2">
                                <label className="flex items-center gap-2 cursor-pointer">
                                    <Input type="color" className="w-8 h-8 rounded border p-0 cursor-pointer" value={newThemeBg} onChange={e => setNewThemeBg(e.target.value)} title="Background" />
                                    <span>Background</span>
                                </label>
                                <label className="flex items-center gap-2 cursor-pointer">
                                    <Input type="color" className="w-8 h-8 rounded border p-0 cursor-pointer" value={newThemeFg} onChange={e => setNewThemeFg(e.target.value)} title="Text" />
                                    <span>Text</span>
                                </label>
                                <label className="flex items-center gap-2 cursor-pointer">
                                    <Input type="color" className="w-8 h-8 rounded border p-0 cursor-pointer" value={newThemePrimary} onChange={e => setNewThemePrimary(e.target.value)} title="Primary" />
                                    <span>Primary</span>
                                </label>
                                <label className="flex items-center gap-2 cursor-pointer">
                                    <Input type="color" className="w-8 h-8 rounded border p-0 cursor-pointer" value={newThemeMutedFg} onChange={e => setNewThemeMutedFg(e.target.value)} title="Muted Text" />
                                    <span>Muted Text</span>
                                </label>
                                <label className="flex items-center gap-2 cursor-pointer">
                                    <Input type="color" className="w-8 h-8 rounded border p-0 cursor-pointer" value={newThemeAccent} onChange={e => setNewThemeAccent(e.target.value)} title="Accent / Active" />
                                    <span>Accent</span>
                                </label>
                                <label className="flex items-center gap-2 cursor-pointer">
                                    <Input type="color" className="w-8 h-8 rounded border p-0 cursor-pointer" value={newThemeBorder} onChange={e => setNewThemeBorder(e.target.value)} title="Border" />
                                    <span>Border</span>
                                </label>
                            </div>
                            <div className="flex gap-2 mt-2">
                                {editThemeId && (
                                    <Button variant="outline" size="sm" onClick={() => {
                                        setEditThemeId(null);
                                        setNewThemeName("");
                                    }} className="w-full">Cancel</Button>
                                )}
                                <Button variant="secondary" size="sm" onClick={handleAddCustomTheme} className="w-full">{editThemeId ? "Update Theme" : "Add Theme"}</Button>
                            </div>
                        </div>
                    </div>

                    <div className="flex items-center justify-between rounded-lg border p-3 shadow-sm">
                        <div className="space-y-0.5">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Enable Notifications
                            </label>
                        </div>
                        <Switch
                            checked={settings.notificationsEnabled}
                            onCheckedChange={(checked) => handleChange("notificationsEnabled", checked)}
                        />
                    </div>
                    <div className="flex items-center justify-between rounded-lg border p-3 shadow-sm">
                        <div className="space-y-0.5">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Enable Sound Alerts
                            </label>
                        </div>
                        <Switch
                            checked={settings.soundEnabled}
                            onCheckedChange={(checked) => handleChange("soundEnabled", checked)}
                        />
                    </div>
                </div>

                <div className="space-y-4">
                    <div className="flex items-center justify-between rounded-lg border p-3 shadow-sm">
                        <div className="space-y-0.5">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Enable Remote Control (LAN)
                            </label>
                            <p className="text-xs text-muted-foreground">
                                Exposes a local HTTP control page on your system so your phone can Start/Pause/Skip.
                            </p>
                        </div>
                        <Switch
                            checked={settings.remoteControlEnabled}
                            onCheckedChange={(checked) => handleChange("remoteControlEnabled", checked)}
                        />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Remote Port
                            </label>
                            <Input
                                type="number"
                                min={1024}
                                max={65535}
                                value={settings.remoteControlPort}
                                onChange={(e) => handleChange("remoteControlPort", Number(e.target.value))}
                                disabled={!settings.remoteControlEnabled}
                            />
                        </div>
                        <div className="space-y-2">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Remote Token
                            </label>
                            <Input
                                value={settings.remoteControlToken}
                                onChange={(e) => handleChange("remoteControlToken", e.target.value)}
                                disabled={!settings.remoteControlEnabled}
                            />
                            <p className="text-xs text-muted-foreground">
                                Remote URL:
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <span
                                            className={`ml-1 rounded px-1 text-xs font-mono bg-muted/50 ${settings.remoteControlEnabled ? "cursor-pointer" : "cursor-not-allowed opacity-70"}`}
                                            onClick={settings.remoteControlEnabled ? () => { void handleCopyIP(); } : undefined}
                                            aria-disabled={!settings.remoteControlEnabled}
                                        >
                                            {settings.remoteControlEnabled ? remoteUrl : "Enable Remote Control to see URL"}
                                        </span>
                                    </TooltipTrigger>
                                    <TooltipContent>
                                        <p className="text-xs">
                                            {settings.remoteControlEnabled ? "Click to copy" : "Enable Remote Control to see URL"}
                                        </p>
                                    </TooltipContent>
                                </Tooltip>
                            </p>
                        </div>
                    </div>
                </div>

                <CliIntegrationCard />

                <div className="pt-4">
                    <Button className="w-full" onClick={onSave}>
                        Save Settings
                    </Button>
                </div>
            </CardContent>
        </Card>
    );
}
