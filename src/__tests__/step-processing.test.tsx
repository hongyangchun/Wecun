import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import StepProcessing from "../components/StepProcessing";
import { describeDownloadIssues } from "../lib/outcome";
import type { DownloadOutcome } from "../types/contracts";

const outcome = (overrides: Partial<DownloadOutcome>): DownloadOutcome => ({
  saved: 8,
  longTextFailed: 0,
  imageFailed: 0,
  resumedFromPage: null,
  stoppedEarly: null,
  added: null,
  unchanged: 0,
  startedOver: null,
  ...overrides,
});

function renderProcessing(props: Partial<ComponentProps<typeof StepProcessing>> = {}) {
  const noop = () => undefined;
  return render(
    <StepProcessing
      processStatus="downloaded"
      phase="下载完成"
      progress={100}
      current={8}
      total={8}
      logs={[]}
      onStop={noop}
      onReset={noop}
      onGoToLogin={noop}
      onExport={noop}
      onContinueExport={noop}
      onOpenOutputDir={noop}
      onAnalyze={noop}
      sourceType="profile"
      {...props}
    />,
  );
}

describe("describeDownloadIssues", () => {
  it("lists nothing for a clean download", () => {
    expect(describeDownloadIssues(outcome({}))).toEqual([]);
  });

  it("lists every kind of shortfall", () => {
    const issues = describeDownloadIssues(outcome({ longTextFailed: 2, imageFailed: 5, stoppedEarly: "接口异常" }));

    expect(issues).toHaveLength(3);
    expect(issues[0]).toContain("2 条长微博");
    expect(issues[1]).toContain("5 张图片");
    expect(issues[2]).toContain("接口异常");
  });
});

describe("StepProcessing result screen", () => {
  it("warns when a finished download is missing content", () => {
    renderProcessing({ outcome: outcome({ longTextFailed: 2, imageFailed: 5, stoppedEarly: "接口异常" }) });

    expect(screen.getByText(/2 条长微博未能获取全文/)).toBeTruthy();
    expect(screen.getByText(/5 张图片下载失败/)).toBeTruthy();
    expect(screen.getByText(/接口异常/)).toBeTruthy();
  });

  it("says where a resumed download continued from", () => {
    renderProcessing({ outcome: outcome({ resumedFromPage: 4 }) });

    expect(screen.getByText(/第 4 页/)).toBeTruthy();
  });

  it("stays quiet when nothing went wrong", () => {
    renderProcessing({ outcome: outcome({}) });

    expect(screen.queryByText(/未能获取全文/)).toBeNull();
  });

  it("offers to continue a stopped download that saved progress", () => {
    const onResume = vi.fn();
    renderProcessing({
      processStatus: "cancelled",
      resumeHint: "进度已保存（已获取 6 条微博）",
      onResume,
    });

    fireEvent.click(screen.getByText("继续下载"));
    expect(onResume).toHaveBeenCalledTimes(1);
  });

  it("offers to open the log folder on the error screen", () => {
    const onOpenLogDir = vi.fn();
    renderProcessing({ processStatus: "error", errorMessage: "下载失败: 网络错误", onOpenLogDir });

    fireEvent.click(screen.getByText("打开日志文件夹"));
    expect(onOpenLogDir).toHaveBeenCalledTimes(1);
  });

  it("reports an update as new posts on top of what was already there", () => {
    renderProcessing({ outcome: outcome({ saved: 8, added: 2, unchanged: 6 }) });

    expect(screen.getByText("更新完成！")).toBeTruthy();
    expect(screen.getByText(/新增 2 条，原有 6 条未改动/)).toBeTruthy();
    expect(screen.getByText(/共 8 条/)).toBeTruthy();
    expect(screen.queryByText("下载完成！")).toBeNull();
  });

  it("says why it started over instead of updating", () => {
    renderProcessing({ outcome: outcome({ saved: 0, startedOver: "筛选条件变了，已重新完整下载" }) });

    expect(screen.getByText("下载完成！")).toBeTruthy();
    expect(screen.getByText(/筛选条件变了，已重新完整下载/)).toBeTruthy();
  });

  it("shows an existing backup as such, not as a download that just happened", () => {
    renderProcessing({ viewingExisting: true, outcome: outcome({ saved: 12 }) });

    expect(screen.getByText("已有备份")).toBeTruthy();
    expect(screen.queryByText("下载完成！")).toBeNull();
    expect(screen.queryByText("更新完成！")).toBeNull();
  });
});
