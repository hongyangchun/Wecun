import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type {
  BackupMeta,
  DownloadOutcome,
  DownloadRequest,
  ExportRequest,
  ProgressEvent,
  ProfileResult,
  SavedSession,
  UserProfile,
} from "../types/contracts";

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

export async function startDownload(request: DownloadRequest): Promise<DownloadOutcome> {
  return await invoke<DownloadOutcome>("start_download", { request });
}

export async function exportPosts(request: ExportRequest): Promise<string> {
  return await invoke<string>("export_posts", { request });
}

/** Folder that holds the app's log file. */
export async function getLogDir(): Promise<string> {
  return await invoke<string>("get_log_dir");
}

/** The finished backup in a directory, when there is one. */
export async function inspectBackup(outputDir: string): Promise<BackupMeta | null> {
  return await invoke<BackupMeta | null>("inspect_backup", { outputDir });
}

/** Whether the user has set a key for the AI analysis. */
export async function getAiKeyStatus(): Promise<boolean> {
  return await invoke<boolean>("get_ai_key_status");
}

/** Keeps the key for the AI analysis in the app's data directory. */
export async function setAiKey(key: string): Promise<void> {
  await invoke("set_ai_key", { key });
}

export async function clearAiKey(): Promise<void> {
  await invoke("clear_ai_key");
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

/** Returns the saved cookie if exists, empty string otherwise. More efficient than hasSavedCookie + loadSavedCookie. */
export async function getSavedCookie(): Promise<string> {
  return await invoke<string>("get_saved_cookie");
}

/** Returns the saved session (cookie + cached user profile) if exists. */
export async function getSavedSession(): Promise<SavedSession | null> {
  return await invoke<SavedSession | null>("get_saved_session");
}

/** Get current logged-in user info (uid + screen_name). Returns null if not logged in. */
export async function getCurrentUserInfo(): Promise<UserProfile | null> {
  return await invoke<UserProfile | null>("get_current_user_info");
}

/** Look up a target blogger's profile for preview before downloading. */
export async function lookupUserInfo(uid: string): Promise<UserProfile> {
  return await invoke<UserProfile>("lookup_user_info", { uid });
}

export async function loadSavedCookie(): Promise<string> {
  return await invoke<string>("load_saved_cookie_cmd");
}

export async function clearSavedCookie(): Promise<void> {
  await invoke("clear_saved_cookie_cmd");
}

export async function setCookie(cookie: string): Promise<void> {
  await invoke("set_cookie_cmd", { cookie });
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

export function onProgress(callback: (event: ProgressEvent) => void) {
  return listen<ProgressEvent>("download-progress", (e) => callback(e.payload));
}

export function onCookieReceived(callback: (cookie: string) => void) {
  return listen<string>("cookie-received", (e) => callback(e.payload));
}

export function onLoginInvalid(callback: (message: string) => void) {
  return listen<string>("login-invalid", (e) => callback(e.payload));
}

export function onLoginWindowClosed(callback: () => void) {
  return listen<string>("login-window-closed", () => callback());
}

export async function analyzeProfile(outputDir: string): Promise<ProfileResult> {
  return await invoke<ProfileResult>("analyze_profile", { outputDir });
}

export async function exportProfile(outputDir: string, profile: ProfileResult): Promise<string> {
  return await invoke<string>("export_profile", { outputDir, profile });
}
