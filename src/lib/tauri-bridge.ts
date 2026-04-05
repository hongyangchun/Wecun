import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { DownloadRequest, ExportRequest, ProgressEvent } from "../types/contracts";

export interface HistoryEntry {
  uid: string;
  screen_name: string;
  output_dir: string;
  last_download: string;
  post_count: number;
}

export interface UpdateProgressStartedEvent {
  event: "Started";
  data: {
    contentLength?: number;
  };
}

export interface UpdateProgressChunkEvent {
  event: "Progress";
  data: {
    chunkLength: number;
  };
}

export interface UpdateProgressFinishedEvent {
  event: "Finished";
}

export type UpdateProgressEvent =
  | UpdateProgressStartedEvent
  | UpdateProgressChunkEvent
  | UpdateProgressFinishedEvent;

export interface AppUpdate {
  version: string;
  currentVersion: string;
  body?: string;
  date?: string;
  downloadAndInstall: (onEvent: (event: UpdateProgressEvent) => void) => Promise<void>;
}

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

export async function checkForAppUpdate(): Promise<AppUpdate | null> {
  const update = await check();

  if (!update) return null;

  return {
    version: update.version,
    currentVersion: update.currentVersion,
    body: update.body,
    date: update.date,
    downloadAndInstall: async (onEvent) => {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          onEvent({ event: "Started", data: { contentLength: event.data.contentLength } });
          return;
        }

        if (event.event === "Progress") {
          onEvent({ event: "Progress", data: { chunkLength: event.data.chunkLength } });
          return;
        }

        onEvent({ event: "Finished" });
      });
    },
  };
}

export async function relaunchApp(): Promise<void> {
  await relaunch();
}

export async function listDownloadHistory(): Promise<HistoryEntry[]> {
  return await invoke<HistoryEntry[]>("list_download_history");
}

export async function deleteHistoryEntry(uid: string): Promise<void> {
  await invoke("delete_history_entry", { uid });
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
