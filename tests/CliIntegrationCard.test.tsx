import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

const { ppCliStatusMock, ppCliUninstallMock } = vi.hoisted(() => ({
  ppCliStatusMock: vi.fn(),
  ppCliUninstallMock: vi.fn(),
}));

vi.mock("@/api", () => ({
  ppCliStatus: ppCliStatusMock,
  ppCliUninstall: ppCliUninstallMock,
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import CliIntegrationCard from "@/components/CliIntegrationCard";

describe("CliIntegrationCard", () => {
  beforeEach(() => {
    ppCliStatusMock.mockResolvedValue({
      binaryInstalled: true,
      pathConfigured: true,
      installDir: "/home/test/.local/bin",
      binaryPath: "/home/test/.local/bin/pp",
    });
    ppCliUninstallMock.mockResolvedValue({
      binaryInstalled: false,
      pathConfigured: false,
      installDir: "/home/test/.local/bin",
      binaryPath: "/home/test/.local/bin/pp",
    });
  });

  it("renders status and wires uninstall action", async () => {
    render(<CliIntegrationCard />);

    await waitFor(() => expect(ppCliStatusMock).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId("cli-binary-status")).toHaveTextContent("Installed");
    expect(screen.getByTestId("cli-path-status")).toHaveTextContent("Configured");

    fireEvent.click(screen.getByRole("button", { name: "Uninstall CLI" }));
    await waitFor(() => expect(ppCliUninstallMock).toHaveBeenCalledTimes(1));

    expect(screen.getByTestId("cli-binary-status")).toHaveTextContent(
      "Not installed",
    );
    expect(screen.getByTestId("cli-path-status")).toHaveTextContent(
      "Not configured",
    );
  });
});
