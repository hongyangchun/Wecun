import type {
  BackupMeta,
  PostFilter,
  DownloadRange,
  DownloadOutcome,
  ExportFormat,
  ProgressPhase,
} from "../types/contracts";

function sameBackup(left: BackupMeta, right: BackupMeta): boolean {
  return (
    left.uid === right.uid
    && left.source_type === right.source_type
    && left.post_count === right.post_count
    && left.author === right.author
    && left.filter === right.filter
    && left.min_text_length === right.min_text_length
    && left.ignore_deleted === right.ignore_deleted
    && left.start_timestamp === right.start_timestamp
    && left.end_timestamp === right.end_timestamp
  );
}

// Get today's date in YYYY-MM-DD format for the default end date
function getTodayDate(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export type ProcessStatus = "idle" | "downloading" | "downloaded" | "exporting" | "done" | "cancelled" | "error";

export interface WizardState {
  step: number;
  isLoggedIn: boolean;
  isLoggingIn: boolean;
  cookie: string;
  username: string;
  currentUid: string;
  usernameFetchFailed: boolean;
  loginError?: string;
  profileUrl: string;
  postFilter: PostFilter;
  includeImages: boolean;
  downloadRange: DownloadRange;
  dateStart: string;
  dateEnd: string;
  ignoreDeleted: boolean;
  minTextLength: number;
  limit: number;
  exportFormat: ExportFormat;
  exportedFormats: ExportFormat[];
  outputDir: string;
  processStatus: ProcessStatus;
  progress: number;
  phase: string;
  current: number;
  total: number;
  errorMessage?: string;
  logs: string[];
  sourceType: "profile" | "favorites";
  /** The finished backup in the chosen directory, when it is the current target's. */
  existingBackup: BackupMeta | null;
  /** The user asked for a download from the first page instead of an update of `existingBackup`. */
  freshDownload: boolean;
  /** The export screen is showing `existingBackup`, not a download that has just happened. */
  viewingExisting: boolean;
  /** What the last finished download actually produced. */
  outcome?: DownloadOutcome | null;
  /** The backend's "your progress is saved" message for a stopped download that can be continued. */
  resumeHint?: string | null;
}

export const INITIAL_STATE: WizardState = {
  step: 0,
  isLoggedIn: false,
  isLoggingIn: false,
  cookie: "",
  username: "",
  currentUid: "",
  usernameFetchFailed: false,
  loginError: undefined,
  profileUrl: "",
  postFilter: "original",
  includeImages: true,
  downloadRange: "all",
  dateStart: "",
  dateEnd: getTodayDate(),
  ignoreDeleted: true,
  minTextLength: 20,
  limit: 1000,
  exportFormat: "html",
  exportedFormats: [],
  outputDir: "",
  processStatus: "idle",
  progress: 0,
  phase: "",
  current: 0,
  total: 0,
  errorMessage: undefined,
  logs: [],
  sourceType: "profile",
  existingBackup: null,
  freshDownload: false,
  viewingExisting: false,
};

export type WizardAction =
  | { type: "SET_STEP"; step: number }
  | { type: "NEXT_STEP" }
  | { type: "PREV_STEP" }
  | { type: "LOGIN_START" }
  | { type: "LOGIN_CANCEL" }
  | { type: "SET_COOKIE"; cookie: string }
  | { type: "LOGIN_SUCCESS"; cookie: string; username?: string; uid?: string }
  | { type: "SET_USERNAME"; username: string; uid?: string }
  | { type: "USERNAME_FETCH_FAILED" }
  | { type: "LOGIN_INVALID"; message: string }
  | { type: "LOGOUT" }
  | { type: "SET_PROFILE_URL"; url: string }
  | { type: "SET_POST_FILTER"; filter: PostFilter }
  | { type: "SET_INCLUDE_IMAGES"; value: boolean }
  | { type: "SET_DOWNLOAD_RANGE"; range: DownloadRange }
  | { type: "SET_DATE_START"; date: string }
  | { type: "SET_DATE_END"; date: string }
  | { type: "SET_IGNORE_DELETED"; value: boolean }
  | { type: "SET_MIN_TEXT_LENGTH"; value: number }
  | { type: "SET_LIMIT"; value: number }
  | { type: "SET_EXPORT_FORMAT"; format: ExportFormat }
  | { type: "MARK_EXPORTED"; format: ExportFormat }
  | { type: "SET_OUTPUT_DIR"; dir: string }
  | { type: "SET_SOURCE_TYPE"; sourceType: "profile" | "favorites" }
  | { type: "START_PROCESSING" }
  | { type: "UPDATE_PROGRESS"; phase: ProgressPhase; current: number; total: number; message: string }
  | { type: "DOWNLOAD_COMPLETE"; outcome?: DownloadOutcome }
  | { type: "SET_EXISTING_BACKUP"; backup: BackupMeta | null }
  | { type: "SET_FRESH_DOWNLOAD"; value: boolean }
  | { type: "OPEN_EXISTING_BACKUP"; postCount: number }
  | { type: "EXPORT_START" }
  | { type: "PROCESS_COMPLETE" }
  | { type: "PROCESS_ERROR"; message: string }
  | { type: "PROCESS_CANCEL" }
  | { type: "CLOSE_PROCESSING" }
  | { type: "ADD_LOG"; message: string }
  | { type: "RESET" };

const MAX_LOGS = 200;

function appendLog(logs: string[], message: string): string[] {
  const next = [...logs, message];
  return next.length > MAX_LOGS ? next.slice(next.length - MAX_LOGS) : next;
}

const PHASE_TEXT: Record<ProgressPhase, string> = {
  FetchingUserInfo: "正在获取用户信息",
  FetchingPostList: "正在获取微博列表",
  FetchingLongText: "正在获取长文内容",
  DownloadingImages: "正在下载图片",
  Exporting: "正在导出文件",
  Complete: "导出完成",
  Error: "出错了",
  Cancelled: "已取消",
  Resuming: "正在恢复下载",
  Retrying: "正在重试",
};

export function wizardReducer(state: WizardState, action: WizardAction): WizardState {
  switch (action.type) {
    case "SET_STEP":
      return { ...state, step: action.step };

    case "NEXT_STEP":
      return { ...state, step: state.step + 1 };

    case "PREV_STEP":
      return { ...state, step: Math.max(0, state.step - 1) };

    case "LOGIN_START":
      return { ...state, isLoggingIn: true };

    case "LOGIN_CANCEL":
      return { ...state, isLoggingIn: false };

    case "SET_COOKIE":
      return { ...state, cookie: action.cookie };

    case "LOGIN_SUCCESS":
      return {
        ...state,
        cookie: action.cookie,
        isLoggedIn: true,
        isLoggingIn: false,
        username: action.username ?? state.username,
        currentUid: action.uid ?? state.currentUid,
        loginError: undefined,
        step: state.step === 0 ? 1 : state.step,
      };

    case "SET_USERNAME":
      return {
        ...state,
        username: action.username,
        currentUid: action.uid ?? state.currentUid,
        usernameFetchFailed: false,
      };

    case "USERNAME_FETCH_FAILED":
      return { ...state, usernameFetchFailed: true };

    case "LOGIN_INVALID":
      return {
        ...state,
        cookie: "",
        username: "",
        currentUid: "",
        isLoggedIn: false,
        isLoggingIn: false,
        loginError: action.message,
        step: 0,
        processStatus: "error",
        errorMessage: action.message,
      };

    case "LOGOUT":
      return {
        ...state,
        cookie: "",
        isLoggedIn: false,
        isLoggingIn: false,
        username: "",
        currentUid: "",
        usernameFetchFailed: false,
        loginError: undefined,
        step: 0,
      };

    case "SET_PROFILE_URL":
      return { ...state, profileUrl: action.url };

    case "SET_POST_FILTER":
      return { ...state, postFilter: action.filter };

    case "SET_INCLUDE_IMAGES":
      return { ...state, includeImages: action.value };

    case "SET_DOWNLOAD_RANGE":
      return { ...state, downloadRange: action.range };

    case "SET_DATE_START":
      return { ...state, dateStart: action.date };

    case "SET_DATE_END":
      return { ...state, dateEnd: action.date };

    case "SET_IGNORE_DELETED":
      return { ...state, ignoreDeleted: action.value };

    case "SET_MIN_TEXT_LENGTH":
      return { ...state, minTextLength: action.value };

    case "SET_LIMIT":
      return { ...state, limit: action.value };

    case "SET_EXPORT_FORMAT":
      return { ...state, exportFormat: action.format };

    case "MARK_EXPORTED":
      return state.exportedFormats.includes(action.format)
        ? state
        : { ...state, exportedFormats: [...state.exportedFormats, action.format] };

    case "SET_OUTPUT_DIR":
      return { ...state, outputDir: action.dir };

    case "SET_SOURCE_TYPE":
      return { ...state, sourceType: action.sourceType };

    case "START_PROCESSING":
      return {
        ...state,
        processStatus: "downloading",
        progress: 0,
        phase: "正在获取用户信息",
        current: 0,
        total: 0,
        errorMessage: undefined,
        outcome: null,
        resumeHint: null,
        viewingExisting: false,
        // "重新完整下载" was for the download that starts now. It sits in a collapsed section, where a
        // setting left on would quietly turn every later download, 继续下载 included, into a full one.
        freshDownload: false,
        exportedFormats: [],
        logs: ["开始下载..."],
      };

    case "UPDATE_PROGRESS": {
      const { phase: p, current: c, total: t, message } = action;
      const phaseLabel = PHASE_TEXT[p] || message;

      if (p === "Retrying") {
        // A retried request is worth a line in the log, but it says nothing about how far the walk is.
        return { ...state, phase: message, logs: appendLog(state.logs, message) };
      }

      if (p === "Complete") {
        return {
          ...state,
          progress: 100,
          phase: phaseLabel,
          current: c,
          total: t,
          logs: appendLog(state.logs, message),
        };
      }

      if (p === "Cancelled") {
        return {
          ...state,
          processStatus: "cancelled",
          phase: phaseLabel,
          logs: appendLog(state.logs, message),
          // `total` is the backend's flag that a checkpoint is on disk and the download can be continued.
          resumeHint: t > 0 ? message : null,
        };
      }

      if (p === "Error") {
        return {
          ...state,
          processStatus: "error",
          errorMessage: message,
          phase: phaseLabel,
          logs: appendLog(state.logs, message),
        };
      }

      // Pages are fetched and processed one by one, so the list phase has a real total whenever
      // Weibo reports one. `total` is 0, and no percentage is shown, when a date range makes it meaningless.
      const pct = t > 0 ? Math.round((c / t) * 100) : 0;
      return {
        ...state,
        progress: pct,
        phase: p === "FetchingPostList" ? message : phaseLabel,
        current: c,
        total: t,
        logs: appendLog(state.logs, message),
      };
    }

    case "EXPORT_START":
      return {
        ...state,
        processStatus: "exporting",
        phase: "正在导出...",
        logs: appendLog(state.logs, "正在导出..."),
      };

    case "DOWNLOAD_COMPLETE":
      return {
        ...state,
        processStatus: "downloaded",
        progress: 100,
        phase: "下载完成",
        // Returning to the export grid, or finishing a profile analysis, must not hide what went wrong.
        outcome: action.outcome ?? state.outcome ?? null,
        viewingExisting: false,
      };

    case "PROCESS_COMPLETE":
      return {
        ...state,
        processStatus: "done",
        progress: 100,
        phase: "导出完成",
      };

    case "SET_EXISTING_BACKUP": {
      // The directory is inspected again after every few keystrokes. An unchanged backup must not
      // make a new state, or the inspection would never settle.
      const current = state.existingBackup;
      const next = action.backup;
      if (current && next && sameBackup(current, next)) {
        return state;
      }
      return { ...state, existingBackup: next, freshDownload: next ? state.freshDownload : false };
    }

    case "SET_FRESH_DOWNLOAD":
      return { ...state, freshDownload: action.value };

    case "OPEN_EXISTING_BACKUP":
      return {
        ...state,
        processStatus: "downloaded",
        progress: 100,
        phase: "已有备份",
        outcome: {
          saved: action.postCount,
          longTextFailed: 0,
          imageFailed: 0,
          resumedFromPage: null,
          stoppedEarly: null,
          added: null,
          unchanged: 0,
          startedOver: null,
        },
        viewingExisting: true,
        errorMessage: undefined,
        exportedFormats: [],
        logs: [],
      };

    case "PROCESS_ERROR":
      return {
        ...state,
        processStatus: "error",
        errorMessage: action.message,
      };

    case "PROCESS_CANCEL":
      return {
        ...state,
        processStatus: "cancelled",
        phase: "已取消",
      };

    case "CLOSE_PROCESSING":
      return {
        ...state,
        processStatus: "idle",
        progress: 0,
        phase: "",
        current: 0,
        total: 0,
        errorMessage: undefined,
        outcome: null,
        resumeHint: null,
        viewingExisting: false,
        exportedFormats: [],
        logs: [],
      };

    case "ADD_LOG":
      return { ...state, logs: appendLog(state.logs, action.message) };

    case "RESET":
      return {
        ...INITIAL_STATE,
        isLoggedIn: state.isLoggedIn,
        cookie: state.cookie,
        username: state.username,
        currentUid: state.currentUid,
      };

    default:
      return state;
  }
}
