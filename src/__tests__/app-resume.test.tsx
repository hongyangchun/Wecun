import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import type { ProgressEvent } from "../types/contracts";

vi.mock("../lib/tauri-bridge", () => ({
  startDownload: vi.fn(),
  getAiKeyStatus: vi.fn().mockResolvedValue(false),
  setAiKey: vi.fn(),
  clearAiKey: vi.fn(),
  inspectBackup: vi.fn(),
  cancelDownload: vi.fn(),
  exportPosts: vi.fn(),
  openOutputDir: vi.fn(),
  clearSavedCookie: vi.fn(),
  onProgress: vi.fn().mockResolvedValue(vi.fn()),
  onCookieReceived: vi.fn().mockResolvedValue(vi.fn()),
  onLoginInvalid: vi.fn().mockResolvedValue(vi.fn()),
  onLoginWindowClosed: vi.fn().mockResolvedValue(vi.fn()),
  hasSavedCookie: vi.fn().mockResolvedValue(false),
  getSavedCookie: vi.fn().mockResolvedValue(""),
  getSavedSession: vi.fn().mockResolvedValue(null),
  getCurrentUserInfo: vi.fn().mockResolvedValue(null),
  lookupUserInfo: vi.fn().mockResolvedValue({ uid: "1", screen_name: "测试" }),
  setCookie: vi.fn().mockResolvedValue(undefined),
  loadSavedCookie: vi.fn().mockResolvedValue(""),
  analyzeProfile: vi.fn(),
  exportProfile: vi.fn(),
  checkForAppUpdate: vi.fn(),
  relaunchApp: vi.fn(),
}));

const bridgeModule = await import("../lib/tauri-bridge");
const startDownloadMock = vi.mocked(bridgeModule.startDownload);
const onProgressMock = vi.mocked(bridgeModule.onProgress);

async function renderSignedIn() {
  render(<App />);
  await screen.findByText("备份来源");
}

/** Delivers one backend progress event the way the Tauri listener would. */
async function reportFromBackend(event: ProgressEvent) {
  await waitFor(() => expect(onProgressMock).toHaveBeenCalledTimes(1));
  const listener = onProgressMock.mock.calls[0][0];
  act(() => listener(event));
}

describe("continuing a stopped download", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation(() => ({
        matches: false,
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
    vi.mocked(bridgeModule.getSavedSession).mockResolvedValue({ cookie: "SUB=session", uid: "1", screen_name: "测试" });
    vi.mocked(bridgeModule.getCurrentUserInfo).mockResolvedValue({ uid: "1", screen_name: "测试" });
    startDownloadMock.mockResolvedValue({ saved: 6, longTextFailed: 0, imageFailed: 0, resumedFromPage: 3, stoppedEarly: null });
  });

  it("starts the download again, with the same form, when 继续下载 is clicked", async () => {
    await renderSignedIn();
    fireEvent.change(screen.getByPlaceholderText(/粘贴微博主页链接/), { target: { value: "https://weibo.com/u/2166767661" } });

    await reportFromBackend({ phase: "Cancelled", current: 6, total: 1, message: "进度已保存（已获取 6 条微博）" });
    fireEvent.click(await screen.findByText("继续下载"));

    await waitFor(() => expect(startDownloadMock).toHaveBeenCalledTimes(1));
    expect(startDownloadMock.mock.calls[0][0].uid, "the form is reused as it was").toBe("2166767661");
    expect(screen.queryByText("继续下载")).toBeNull();
  });

  it("recognises the backup already in the directory and exports it without downloading", async () => {
    localStorage.setItem("weibo-dl-settings", JSON.stringify({ outputDir: "/tmp/weibo" }));
    vi.mocked(bridgeModule.inspectBackup).mockResolvedValue({
      uid: "2166767661",
      source_type: "profile",
      post_count: 12,
      author: "测试博主",
      filter: "all",
      min_text_length: 0,
      ignore_deleted: false,
      start_timestamp: null,
      end_timestamp: null,
    });
    await renderSignedIn();
    fireEvent.change(screen.getByPlaceholderText(/粘贴微博主页链接/), { target: { value: "https://weibo.com/u/2166767661" } });

    expect(await screen.findByText(/已有 @测试博主 的备份，共 12 条/)).toBeTruthy();

    fireEvent.click(screen.getByText("导出已有备份"));

    expect(await screen.findByRole("heading", { name: "已有备份" })).toBeTruthy();
    expect(startDownloadMock).not.toHaveBeenCalled();
  });

  it("offers nothing to continue when the stop saved no progress", async () => {
    await renderSignedIn();

    await reportFromBackend({ phase: "Cancelled", current: 0, total: 0, message: "已取消" });

    expect(await screen.findByRole("heading", { name: "已取消" })).toBeTruthy();
    expect(screen.queryByText("继续下载")).toBeNull();
  });

  it("names the signed-in account when backing up favourites, so that another account's backup is never continued", async () => {
    localStorage.setItem("weibo-dl-settings", JSON.stringify({ outputDir: "/tmp/weibo" }));
    await renderSignedIn();

    fireEvent.click(screen.getByRole("radio", { name: "我的收藏" }));
    fireEvent.click(screen.getByRole("button", { name: /开始下载/ }));

    await waitFor(() => expect(startDownloadMock).toHaveBeenCalledTimes(1));
    const request = startDownloadMock.mock.calls[0][0];
    expect(request.source_type).toBe("favorites");
    expect(request.uid, "the account whose favourites these are").toBe("1");
  });
});
