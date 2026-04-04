import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DownloadRequest, ProgressEvent } from "../types/contracts";

export async function startDownload(request: DownloadRequest): Promise<string> {
  return await invoke<string>("start_download", { request });
}

export async function cancelDownload(): Promise<void> {
  await invoke("cancel_download");
}

export async function openLoginWindow(): Promise<void> {
  await invoke("open_login_window");
}

export async function openOutputDir(dir: string): Promise<void> {
  await invoke("plugin:opener|open_path", { path: dir });
}

export function onProgress(callback: (event: ProgressEvent) => void) {
  return listen<ProgressEvent>("download-progress", (e) => callback(e.payload));
}

export function onCookieReceived(callback: (cookie: string) => void) {
  return listen<string>("cookie-received", (e) => callback(e.payload));
}

export async function testApi(cookie: string, uid: string): Promise<string> {
  return await invoke<string>("test_api", { cookie, uid });
}
