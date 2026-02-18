import { AppSettings } from "../types";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
// Label removed
// If label is not installed, I use standard label with classes.
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";

interface SettingsPanelProps {
    settings: AppSettings | null;
    onUpdate: (newSettings: AppSettings) => void;
    onSave: () => void;
}

export default function SettingsPanel({ settings, onUpdate, onSave }: SettingsPanelProps) {
    if (!settings) return null;

    const handleChange = (field: keyof AppSettings, value: number | boolean | string) => {
        onUpdate({ ...settings, [field]: value });
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
                    <div className="flex items-center justify-between rounded-lg border p-3 shadow-sm">
                        <div className="space-y-0.5">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Enable macOS Notifications
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
                                Enable iPhone Remote Control (LAN)
                            </label>
                            <p className="text-xs text-muted-foreground">
                                Exposes a local HTTP control page on your Mac so your iPhone can Start/Pause/Skip.
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
                                iPhone URL: <span className="font-mono break-all">http://YOUR_MAC_IP:{settings.remoteControlPort}/</span>
                            </p>
                        </div>
                    </div>

                    <div className="flex items-center justify-between rounded-lg border border-destructive/20 bg-destructive/5 p-3 shadow-sm">
                        <div className="space-y-0.5">
                            <label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
                                Allow remote access from public networks (unsafe)
                            </label>
                            <p className="text-xs text-muted-foreground">
                                Not recommended. Enabling this may expose your remote control to untrusted devices and Wi-Fi sniffing.
                            </p>
                        </div>
                        <Switch
                            checked={settings.remoteControlAllowPublicNetwork}
                            onCheckedChange={(checked) => handleChange("remoteControlAllowPublicNetwork", checked)}
                            disabled={!settings.remoteControlEnabled}
                        />
                    </div>
                </div>

                <div className="pt-4">
                    <Button className="w-full" onClick={onSave}>
                        Save Settings
                    </Button>
                </div>
            </CardContent>
        </Card>
    );
}
