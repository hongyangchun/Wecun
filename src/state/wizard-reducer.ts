import type { PostFilter, DownloadRange, ExportFormat, ProgressPhase } from "../types/contracts";

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
  outputDir: string;
  processStatus: ProcessStatus;
  progress: number;
  phase: string;
  current: number;
  total: number;
  errorMessage?: string;
  logs: string[];
  sourceType: "profile" | "favorites";
}

export const INITIAL_STATE: WizardState = {
  step: 0,
  isLoggedIn: false,
  isLoggingIn: false,
  cookie: "",
  username: "",
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
  outputDir: "",
  processStatus: "idle",
  progress: 0,
  phase: "",
  current: 0,
  total: 0,
  errorMessage: undefined,
  logs: [],
  sourceType: "profile",
};

export type WizardAction =
  | { type: "SET_STEP"; step: number }
  | { type: "NEXT_STEP" }
  | { type: "PREV_STEP" }
  | { type: "LOGIN_START" }
  | { type: "LOGIN_SUCCESS"; cookie: string; username?: string }
  | { type: "SET_USERNAME"; username: string }
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
  | { type: "SET_OUTPUT_DIR"; dir: string }
  | { type: "SET_SOURCE_TYPE"; sourceType: "profile" | "favorites" }
  | { type: "START_PROCESSING" }
  | { type: "UPDATE_PROGRESS"; phase: ProgressPhase; current: number; total: number; message: string }
  | { type: "DOWNLOAD_COMPLETE" }
  | { type: "EXPORT_START" }
  | { type: "PROCESS_COMPLETE" }
  | { type: "PROCESS_ERROR"; message: string }
  | { type: "PROCESS_CANCEL" }
  | { type: "ADD_LOG"; message: string }
  | { type: "RESET" };

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

  case "LOGIN_SUCCESS":
      return {
        ...state,
        cookie: action.cookie,
        isLoggedIn: true,
        isLoggingIn: false,
        username: action.username ?? state.username,
        loginError: undefined,
        step: state.step === 0 ? 1 : state.step,
      };

  case "SET_USERNAME":
      return { ...state, username: action.username, usernameFetchFailed: false };

  case "USERNAME_FETCH_FAILED":
      return { ...state, usernameFetchFailed: true };

    case "LOGIN_INVALID":
      return {
        ...state,
        cookie: "",
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
        username: "",
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
        logs: ["开始下载..."],
      };

    case "UPDATE_PROGRESS": {
      const { phase: p, current: c, total: t, message } = action;
      const phaseLabel = PHASE_TEXT[p] || message;

      if (p === "Complete") {
        return {
          ...state,
          progress: 100,
          phase: phaseLabel,
          current: c,
          total: t,
          logs: [...state.logs, message],
        };
      }

      if (p === "Cancelled") {
        return {
          ...state,
          processStatus: "cancelled",
          phase: phaseLabel,
          logs: [...state.logs, message],
        };
      }

      if (p === "Error") {
        return {
          ...state,
          processStatus: "error",
          errorMessage: message,
          phase: phaseLabel,
          logs: [...state.logs, message],
        };
      }

      const pct = p === "FetchingPostList" ? 0 : (t > 0 ? Math.round((c / t) * 100) : 0);
      return {
        ...state,
        progress: pct,
        phase: p === "FetchingPostList" ? message : phaseLabel,
        current: c,
        total: t,
        logs: [...state.logs, message],
      };
    }

    case "EXPORT_START":
      return {
        ...state,
        processStatus: "exporting",
        phase: "正在导出...",
        logs: [...state.logs, "正在导出..."],
      };

    case "DOWNLOAD_COMPLETE":
      return {
        ...state,
        processStatus: "downloaded",
        progress: 100,
        phase: "下载完成",
      };

    case "PROCESS_COMPLETE":
      return {
        ...state,
        processStatus: "done",
        progress: 100,
        phase: "导出完成",
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

    case "ADD_LOG":
      return { ...state, logs: [...state.logs, action.message] };

    case "RESET":
      return { ...INITIAL_STATE, isLoggedIn: state.isLoggedIn, cookie: state.cookie };

    default:
      return state;
  }
}
