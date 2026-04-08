import { describe, expect, it } from "vitest";
import { INITIAL_STATE, wizardReducer, type WizardAction, type WizardState } from "../state/wizard-reducer";
import type { DateMode, ExportFormat, PostFilter, ProgressPhase } from "../types/contracts";

function reduce(action: WizardAction, state: WizardState = INITIAL_STATE) {
  return wizardReducer(state, action);
}

const setActionCases: Array<{
  label: string;
  action: WizardAction;
  key: keyof WizardState;
  expected: WizardState[keyof WizardState];
}> = [
  { label: "SET_PROFILE_URL", action: { type: "SET_PROFILE_URL", url: "https://weibo.com/u/1" }, key: "profileUrl", expected: "https://weibo.com/u/1" },
  { label: "SET_POST_FILTER", action: { type: "SET_POST_FILTER", filter: "all" satisfies PostFilter }, key: "postFilter", expected: "all" },
  { label: "SET_INCLUDE_IMAGES", action: { type: "SET_INCLUDE_IMAGES", value: false }, key: "includeImages", expected: false },
  { label: "SET_DATE_MODE", action: { type: "SET_DATE_MODE", mode: "range" satisfies DateMode }, key: "dateMode", expected: "range" },
  { label: "SET_DATE_START", action: { type: "SET_DATE_START", date: "2024-01-01" }, key: "dateStart", expected: "2024-01-01" },
  { label: "SET_DATE_END", action: { type: "SET_DATE_END", date: "2024-01-31" }, key: "dateEnd", expected: "2024-01-31" },
  { label: "SET_IGNORE_DELETED", action: { type: "SET_IGNORE_DELETED", value: false }, key: "ignoreDeleted", expected: false },
  { label: "SET_EXPORT_FORMAT", action: { type: "SET_EXPORT_FORMAT", format: "md-obsidian" satisfies ExportFormat }, key: "exportFormat", expected: "md-obsidian" },
  { label: "SET_OUTPUT_DIR", action: { type: "SET_OUTPUT_DIR", dir: "/tmp/weibo" }, key: "outputDir", expected: "/tmp/weibo" },
];

const progressCases: Array<{
  phase: ProgressPhase;
  current: number;
  total: number;
  message: string;
  progress: number;
  phaseLabel: string;
}> = [
  { phase: "FetchingUserInfo", current: 1, total: 4, message: "获取用户信息中", progress: 25, phaseLabel: "正在获取用户信息" },
  { phase: "FetchingPostList", current: 3, total: 9, message: "已抓取第 3 页", progress: 0, phaseLabel: "已抓取第 3 页" },
  { phase: "FetchingLongText", current: 2, total: 5, message: "长文处理中", progress: 40, phaseLabel: "正在获取长文内容" },
  { phase: "DownloadingImages", current: 3, total: 6, message: "图片处理中", progress: 50, phaseLabel: "正在下载图片" },
  { phase: "Exporting", current: 1, total: 2, message: "导出中", progress: 50, phaseLabel: "正在导出文件" },
  { phase: "Resuming", current: 1, total: 3, message: "恢复中", progress: 33, phaseLabel: "正在恢复下载" },
];

describe("wizardReducer", () => {
  it("handles SET_STEP", () => {
    expect(reduce({ type: "SET_STEP", step: 3 }).step).toBe(3);
  });

  it("handles NEXT_STEP", () => {
    expect(reduce({ type: "NEXT_STEP" }, { ...INITIAL_STATE, step: 1 }).step).toBe(2);
  });

  it("handles PREV_STEP and never goes below zero", () => {
    expect(reduce({ type: "PREV_STEP" }, { ...INITIAL_STATE, step: 2 }).step).toBe(1);
    expect(reduce({ type: "PREV_STEP" }, { ...INITIAL_STATE, step: 0 }).step).toBe(0);
  });

  it("handles LOGIN_SUCCESS and auto-advances from step 0", () => {
    const next = reduce({ type: "LOGIN_SUCCESS", cookie: "SUB=cookie" });

    expect(next.isLoggedIn).toBe(true);
    expect(next.cookie).toBe("SUB=cookie");
    expect(next.loginError).toBeUndefined();
    expect(next.step).toBe(1);
  });

  it("handles LOGIN_SUCCESS without changing later steps", () => {
    const next = reduce(
      { type: "LOGIN_SUCCESS", cookie: "SUB=cookie" },
      { ...INITIAL_STATE, step: 2, loginError: "old" },
    );

    expect(next.step).toBe(2);
    expect(next.loginError).toBeUndefined();
  });

  it("handles LOGIN_INVALID by resetting login state and step", () => {
    const next = reduce(
      { type: "LOGIN_INVALID", message: "登录失效" },
      { ...INITIAL_STATE, step: 3, isLoggedIn: true, cookie: "SUB=abc" },
    );

    expect(next.step).toBe(0);
    expect(next.isLoggedIn).toBe(false);
    expect(next.cookie).toBe("");
    expect(next.loginError).toBe("登录失效");
    expect(next.processStatus).toBe("error");
    expect(next.errorMessage).toBe("登录失效");
  });

  it("handles LOGOUT", () => {
    const next = reduce(
      { type: "LOGOUT" },
      { ...INITIAL_STATE, step: 2, isLoggedIn: true, cookie: "SUB=abc", loginError: "old" },
    );

    expect(next.step).toBe(0);
    expect(next.isLoggedIn).toBe(false);
    expect(next.cookie).toBe("");
    expect(next.loginError).toBeUndefined();
  });

  it.each(setActionCases)("handles $label", ({ action, key, expected }) => {
    const next = reduce(action);
    expect(next[key]).toEqual(expected);
  });

  it("handles START_PROCESSING by resetting progress state", () => {
    const next = reduce(
      { type: "START_PROCESSING" },
      {
        ...INITIAL_STATE,
        processStatus: "error",
        progress: 66,
        phase: "旧阶段",
        current: 3,
        total: 8,
        errorMessage: "old",
        logs: ["before"],
      },
    );

    expect(next.processStatus).toBe("downloading");
    expect(next.progress).toBe(0);
    expect(next.phase).toBe("正在获取用户信息");
    expect(next.current).toBe(0);
    expect(next.total).toBe(0);
    expect(next.errorMessage).toBeUndefined();
    expect(next.logs).toEqual(["开始下载..."]);
  });

  it.each(progressCases)("handles UPDATE_PROGRESS for $phase", ({ phase, current, total, message, progress, phaseLabel }) => {
    const next = reduce(
      { type: "UPDATE_PROGRESS", phase, current, total, message },
      { ...INITIAL_STATE, logs: ["已有日志"] },
    );

    expect(next.processStatus).toBe("idle");
    expect(next.progress).toBe(progress);
    expect(next.phase).toBe(phaseLabel);
    expect(next.current).toBe(current);
    expect(next.total).toBe(total);
    expect(next.logs).toEqual(["已有日志", message]);
  });

  it("handles UPDATE_PROGRESS Complete with 100 percent", () => {
    const next = reduce(
      { type: "UPDATE_PROGRESS", phase: "Complete", current: 8, total: 8, message: "完成了" },
      { ...INITIAL_STATE, logs: [] },
    );

    expect(next.progress).toBe(100);
    expect(next.phase).toBe("导出完成");
    expect(next.current).toBe(8);
    expect(next.total).toBe(8);
    expect(next.logs).toEqual(["完成了"]);
  });

  it("handles UPDATE_PROGRESS Error", () => {
    const next = reduce(
      { type: "UPDATE_PROGRESS", phase: "Error", current: 0, total: 0, message: "下载失败" },
      { ...INITIAL_STATE, logs: [] },
    );

    expect(next.processStatus).toBe("error");
    expect(next.errorMessage).toBe("下载失败");
    expect(next.phase).toBe("出错了");
    expect(next.logs).toEqual(["下载失败"]);
  });

  it("handles UPDATE_PROGRESS Cancelled", () => {
    const next = reduce(
      { type: "UPDATE_PROGRESS", phase: "Cancelled", current: 0, total: 0, message: "已取消下载" },
      { ...INITIAL_STATE, logs: [] },
    );

    expect(next.processStatus).toBe("cancelled");
    expect(next.phase).toBe("已取消");
    expect(next.logs).toEqual(["已取消下载"]);
  });

  it("handles EXPORT_START", () => {
    const next = reduce({ type: "EXPORT_START" }, { ...INITIAL_STATE, logs: ["开始下载..."] });

    expect(next.processStatus).toBe("exporting");
    expect(next.phase).toBe("正在导出...");
    expect(next.logs).toEqual(["开始下载...", "正在导出..."]);
  });

  it("handles DOWNLOAD_COMPLETE", () => {
    const next = reduce(
      { type: "DOWNLOAD_COMPLETE" },
      { ...INITIAL_STATE, processStatus: "downloading", progress: 72, phase: "下载中" },
    );

    expect(next.processStatus).toBe("downloaded");
    expect(next.progress).toBe(100);
    expect(next.phase).toBe("下载完成");
  });

  it("handles PROCESS_COMPLETE", () => {
    const next = reduce({ type: "PROCESS_COMPLETE" }, { ...INITIAL_STATE, processStatus: "exporting" });

    expect(next.processStatus).toBe("done");
    expect(next.progress).toBe(100);
    expect(next.phase).toBe("导出完成");
  });

  it("handles PROCESS_ERROR", () => {
    const next = reduce({ type: "PROCESS_ERROR", message: "导出失败" });

    expect(next.processStatus).toBe("error");
    expect(next.errorMessage).toBe("导出失败");
  });

  it("handles PROCESS_CANCEL", () => {
    const next = reduce({ type: "PROCESS_CANCEL" });

    expect(next.processStatus).toBe("cancelled");
    expect(next.phase).toBe("已取消");
  });

  it("handles ADD_LOG", () => {
    const next = reduce({ type: "ADD_LOG", message: "新增日志" }, { ...INITIAL_STATE, logs: ["旧日志"] });
    expect(next.logs).toEqual(["旧日志", "新增日志"]);
  });

  it("handles RESET while preserving auth state", () => {
    const next = reduce(
      { type: "RESET" },
      {
        ...INITIAL_STATE,
        step: 3,
        isLoggedIn: true,
        cookie: "SUB=abc",
        profileUrl: "https://weibo.com/u/2166767661",
        postFilter: "all",
        includeImages: false,
        dateMode: "range",
        dateStart: "2024-01-01",
        dateEnd: "2024-01-31",
        ignoreDeleted: false,
        exportFormat: "md-obsidian",
        outputDir: "/tmp/out",
        processStatus: "done",
        progress: 100,
        phase: "下载完成",
        current: 10,
        total: 10,
        errorMessage: "old",
        logs: ["完成"],
      },
    );

    expect(next).toEqual({
      ...INITIAL_STATE,
      isLoggedIn: true,
      cookie: "SUB=abc",
    });
  });

  it("returns the same state for unknown actions", () => {
    const state = { ...INITIAL_STATE, step: 2 };
    const next = wizardReducer(state, { type: "UNKNOWN" } as unknown as WizardAction);
    expect(next).toBe(state);
  });
});
