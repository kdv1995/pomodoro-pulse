import { Timer, BarChart2, Settings } from "lucide-react";
import { Button } from "@/components/ui/button";


interface SidebarProps {
    activeTab: "timer" | "stats" | "settings";
    onChange: (tab: "timer" | "stats" | "settings") => void;
}

export default function Sidebar({ activeTab, onChange }: SidebarProps) {
    return (
        <div className="flex w-16 flex-col border-r bg-muted/20 py-4 h-full">
            <div className="mb-8 flex justify-center">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary text-primary-foreground font-bold">
                    P
                </div>
            </div>
            <nav className="flex flex-col items-center gap-4 px-2">
                <Button
                    variant={activeTab === "timer" ? "secondary" : "ghost"}
                    size="icon"
                    onClick={() => onChange("timer")}
                    title="Timer"
                    aria-label="Timer"
                >
                    <Timer className="h-5 w-5" />
                </Button>
                <Button
                    variant={activeTab === "stats" ? "secondary" : "ghost"}
                    size="icon"
                    onClick={() => onChange("stats")}
                    title="Statistics"
                    aria-label="Statistics"
                >
                    <BarChart2 className="h-5 w-5" />
                </Button>
                <Button
                    variant={activeTab === "settings" ? "secondary" : "ghost"}
                    size="icon"
                    onClick={() => onChange("settings")}
                    title="Settings"
                    aria-label="Settings"
                >
                    <Settings className="h-5 w-5" />
                </Button>
            </nav>
        </div>
    );
}
