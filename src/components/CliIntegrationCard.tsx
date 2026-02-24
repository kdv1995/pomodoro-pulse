import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { ppCliStatus, ppCliUninstall } from "@/api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { CliInstallStatus } from "@/types";

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function CliIntegrationCard() {
  const [status, setStatus] = useState<CliInstallStatus | null>(null);
  const [statusBusy, setStatusBusy] = useState(false);
  const [uninstallBusy, setUninstallBusy] = useState(false);

  const loadStatus = useCallback(async () => {
    setStatusBusy(true);
    try {
      const next = await ppCliStatus();
      setStatus(next);
    } catch (error) {
      toast.error("Failed to check CLI status.", {
        description: toErrorMessage(error),
        position: "top-center",
      });
    } finally {
      setStatusBusy(false);
    }
  }, []);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  async function onUninstall() {
    setUninstallBusy(true);
    try {
      const next = await ppCliUninstall();
      setStatus(next);
      toast.success("CLI integration removed.", {
        position: "top-center",
      });
    } catch (error) {
      toast.error("Failed to uninstall CLI integration.", {
        description: toErrorMessage(error),
        position: "top-center",
      });
    } finally {
      setUninstallBusy(false);
    }
  }

  const busy = statusBusy || uninstallBusy;

  return (
    <div className="flex flex-col gap-3 rounded-lg border p-3 shadow-sm">
      <div className="flex items-center justify-between gap-3">
        <div className="space-y-0.5">
          <label className="text-sm font-medium leading-none">
            Standalone CLI (pp)
          </label>
          <p className="text-xs text-muted-foreground">
            Check install/PATH state and remove the managed CLI integration.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void loadStatus()}
          disabled={busy}
        >
          {statusBusy ? "Refreshing..." : "Refresh"}
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-4 text-sm">
        <div className="flex items-center gap-2">
          <span>Binary</span>
          <Badge
            data-testid="cli-binary-status"
            variant={status?.binaryInstalled ? "default" : "secondary"}
          >
            {status?.binaryInstalled ? "Installed" : "Not installed"}
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <span>PATH</span>
          <Badge
            data-testid="cli-path-status"
            variant={status?.pathConfigured ? "default" : "secondary"}
          >
            {status?.pathConfigured ? "Configured" : "Not configured"}
          </Badge>
        </div>
      </div>

      {status && (
        <p className="text-xs text-muted-foreground">
          Install dir:{" "}
          <span className="font-mono text-foreground">{status.installDir}</span>
        </p>
      )}

      <div>
        <Button
          variant="destructive"
          size="sm"
          onClick={onUninstall}
          disabled={busy || status === null}
        >
          {uninstallBusy ? "Uninstalling..." : "Uninstall CLI"}
        </Button>
      </div>
    </div>
  );
}
