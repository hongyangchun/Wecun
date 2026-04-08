import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { UpdateProgressEvent } from "../lib/tauri-bridge";

vi.mock("../lib/tauri-bridge", () => ({
  startDownload: vi.fn(),
  cancelDownload: vi.fn(),
  exportPosts: vi.fn(),
  openOutputDir: vi.fn(),
  clearSavedCookie: vi.fn(),
  onProgress: vi.fn().mockResolvedValue(vi.fn()),
  onCookieReceived: vi.fn().mockResolvedValue(vi.fn()),
  onLoginInvalid: vi.fn().mockResolvedValue(vi.fn()),
  hasSavedCookie: vi.fn().mockResolvedValue(false),
  loadSavedCookie: vi.fn().mockResolvedValue(""),
  checkForAppUpdate: vi.fn(),
  relaunchApp: vi.fn(),
}));

const bridgeModule = await import("../lib/tauri-bridge");
const checkForAppUpdateMock = vi.mocked(bridgeModule.checkForAppUpdate);
const relaunchAppMock = vi.mocked(bridgeModule.relaunchApp);

describe("App updater flow", () => {
  beforeEach(() => {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation(() => ({
        matches: false,
        media: "(prefers-reduced-motion: reduce)",
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    checkForAppUpdateMock.mockResolvedValue(null);
    relaunchAppMock.mockResolvedValue();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("checks for updates on startup", async () => {
    render(<App />);

    await waitFor(() => {
      expect(checkForAppUpdateMock).toHaveBeenCalledTimes(1);
    });
  });

  it("shows a toast when an update is available", async () => {
    checkForAppUpdateMock.mockResolvedValue({
      version: "0.2.0",
      currentVersion: "0.1.0",
      body: "修复更新流程",
      date: "2026-04-05T00:00:00Z",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    });

    render(<App />);

    expect(await screen.findByText("发现新版本 0.2.0")).toBeTruthy();
    expect(screen.getByRole("button", { name: "下载并安装" })).toBeTruthy();
    expect(screen.getByText("修复更新流程")).toBeTruthy();
  });

  it("downloads, shows progress, and relaunches after install", async () => {
    const downloadAndInstall = vi.fn(
      async (onEvent: (event: UpdateProgressEvent) => void) => {
        onEvent({ event: "Started", data: { contentLength: 100 } });
        onEvent({ event: "Progress", data: { chunkLength: 25 } });
        onEvent({ event: "Progress", data: { chunkLength: 75 } });
        onEvent({ event: "Finished" });
      },
    );

    checkForAppUpdateMock.mockResolvedValue({
      version: "0.2.0",
      currentVersion: "0.1.0",
      downloadAndInstall,
    });

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "下载并安装" }));

    await waitFor(() => {
      expect(downloadAndInstall).toHaveBeenCalledTimes(1);
    });

    expect(await screen.findByText("正在安装更新…")).toBeTruthy();
    expect(screen.getByText("下载完成，正在应用更新…")).toBeTruthy();

    await waitFor(() => {
      expect(relaunchAppMock).toHaveBeenCalledTimes(1);
    });
  });
});
