import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { DownloadRequest, ExportRequest, ProgressEvent } from "../types/contracts";

export async function startDownload(request: DownloadRequest): Promise<string> {
  return await invoke<string>("start_download", { request });
}

export async function exportPosts(request: ExportRequest): Promise<string> {
  return await invoke<string>("export_posts", { request });
}

export async function cancelDownload(): Promise<void> {
  await invoke("cancel_download");
}

export async function openLoginWindow(): Promise<void> {
  await invoke("open_login_window");
}

export async function openOutputDir(dir: string): Promise<void> {
  await revealItemInDir(dir);
}

export async function hasSavedCookie(): Promise<boolean> {
  return await invoke<boolean>("has_saved_cookie");
}

export async function loadSavedCookie(): Promise<string> {
  return await invoke<string>("load_saved_cookie_cmd");
}

export async function clearSavedCookie(): Promise<void> {
  await invoke("clear_saved_cookie_cmd");
}

export async function closeWindow(): Promise<void> {
  await getCurrentWindow().close();
}

export function onProgress(callback: (event: ProgressEvent) => void) {
  return listen<ProgressEvent>("download-progress", (e) => callback(e.payload));
}

export function onCookieReceived(callback: (cookie: string) => void) {
  return listen<string>("cookie-received", (e) => callback(e.payload));
}

export function onLoginInvalid(callback: (message: string) => void) {
  return listen<string>("login-invalid", (e) => callback(e.payload));
}
