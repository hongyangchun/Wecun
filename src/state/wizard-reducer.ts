import type { PostFilter, DateMode, ExportFormat, ProgressPhase } from "../types/contracts";

export type ProcessStatus = "idle" | "downloading" | "downloaded" | "exporting" | "done" | "cancelled" | "error";

export interface WizardState {
  step: number;
  isLoggedIn: boolean;
  cookie: string;
  loginError?: string;
  profileUrl: string;
  postFilter: PostFilter;
  includeImages: boolean;
  dateMode: DateMode;
  dateStart: string;
  dateEnd: string;
  minTextLength: number;
  exportFormat: ExportFormat;
  outputDir: string;
  processStatus: ProcessStatus;
  progress: number;
  phase: string;
  current: number;
  total: number;
  errorMessage?: string;
  logs: string[];
}

export const INITIAL_STATE: WizardState = {
  step: 0,
  isLoggedIn: false,
  cookie: "",
  loginError: undefined,
  profileUrl: "",
  postFilter: "original",
  includeImages: true,
  dateMode: "all",
  dateStart: "",
  dateEnd: "",
  minTextLength: 20,
  exportFormat: "html",
  outputDir: "",
  processStatus: "idle",
  progress: 0,
  phase: "",
  current: 0,
  total: 0,
  errorMessage: undefined,
  logs: [],
};

export type WizardAction =
  | { type: "SET_STEP"; step: number }
  | { type: "NEXT_STEP" }
  | { type: "PREV_STEP" }
  | { type: "LOGIN_SUCCESS"; cookie: string }
  | { type: "LOGIN_INVALID"; message: string }
  | { type: "LOGOUT" }
  | { type: "SET_PROFILE_URL"; url: string }
  | { type: "SET_POST_FILTER"; filter: PostFilter }
  | { type: "SET_INCLUDE_IMAGES"; value: boolean }
  | { type: "SET_DATE_MODE"; mode: DateMode }
  | { type: "SET_DATE_START"; date: string }
  | { type: "SET_DATE_END"; date: string }
  | { type: "SET_MIN_TEXT_LENGTH"; length: number }
  | { type: "SET_EXPORT_FORMAT"; format: ExportFormat }
  | { type: "SET_OUTPUT_DIR"; dir: string }
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

    case "LOGIN_SUCCESS":
      return {
        ...state,
        cookie: action.cookie,
        isLoggedIn: true,
        loginError: undefined,
        step: state.step === 0 ? 1 : state.step,
      };

    case "LOGIN_INVALID":
      return {
        ...state,
        cookie: "",
        isLoggedIn: false,
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
        loginError: undefined,
        step: 0,
      };

    case "SET_PROFILE_URL":
      return { ...state, profileUrl: action.url };

    case "SET_POST_FILTER":
      return { ...state, postFilter: action.filter };

    case "SET_INCLUDE_IMAGES":
      return { ...state, includeImages: action.value };

    case "SET_DATE_MODE":
      return { ...state, dateMode: action.mode };

    case "SET_DATE_START":
      return { ...state, dateStart: action.date };

    case "SET_DATE_END":
      return { ...state, dateEnd: action.date };

    case "SET_MIN_TEXT_LENGTH":
      return { ...state, minTextLength: action.length };

    case "SET_EXPORT_FORMAT":
      return { ...state, exportFormat: action.format };

    case "SET_OUTPUT_DIR":
      return { ...state, outputDir: action.dir };

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
