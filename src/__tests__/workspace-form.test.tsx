import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import WorkspaceForm from "../components/WorkspaceForm";

const noop = () => undefined;

const backup = {
  uid: "2166767661",
  source_type: "profile",
  post_count: 12,
  author: "测试博主",
  filter: "all",
  min_text_length: 0,
  ignore_deleted: false,
  start_timestamp: null,
  end_timestamp: null,
} as const;

function renderForm(props: Partial<ComponentProps<typeof WorkspaceForm>> = {}) {
  const base: ComponentProps<typeof WorkspaceForm> = {
    sourceType: "profile",
    onSourceTypeChange: noop,
    profileUrl: "https://weibo.com/u/2166767661",
    onProfileUrlChange: noop,
    currentUsername: "",
    currentUid: "",
    recentTargets: [],
    onRemoveRecentTarget: noop,
    onResolvedAuthor: noop,
    postFilter: "all",
    onPostFilterChange: noop,
    includeImages: true,
    onIncludeImagesChange: noop,
    downloadRange: "all",
    onDownloadRangeChange: noop,
    dateStart: "",
    onDateStartChange: noop,
    dateEnd: "",
    onDateEndChange: noop,
    ignoreDeleted: false,
    onIgnoreDeletedChange: noop,
    minTextLength: 0,
    onMinTextLengthChange: noop,
    outputDir: "/tmp/weibo",
    onDirChange: noop,
    onOpenOutputDir: noop,
    onStartDownload: noop,
    canStart: true,
    existingBackup: null,
    freshDownload: false,
    onFreshDownloadChange: noop,
    onExportExisting: noop,
    ...props,
  };
  return render(<WorkspaceForm {...base} />);
}

describe("WorkspaceForm with a backup already in the directory", () => {
  it("says whose backup is there and offers to export it", () => {
    const onExportExisting = vi.fn();
    renderForm({ existingBackup: backup, onExportExisting });

    expect(screen.getByText(/已有 @测试博主 的备份，共 12 条/)).toBeTruthy();
    fireEvent.click(screen.getByText("导出已有备份"));
    expect(onExportExisting).toHaveBeenCalledTimes(1);
  });

  it("lets the user ask for a full download instead, under the advanced options", () => {
    const onFreshDownloadChange = vi.fn();
    renderForm({ existingBackup: backup, onFreshDownloadChange });

    fireEvent.click(screen.getByText("高级筛选"));
    fireEvent.click(screen.getByRole("checkbox", { name: /重新完整下载/ }));

    expect(onFreshDownloadChange).toHaveBeenCalledWith(true);
  });

  it("mentions no backup when the directory holds none", () => {
    renderForm({ existingBackup: null });

    fireEvent.click(screen.getByText("高级筛选"));
    expect(screen.queryByText("导出已有备份")).toBeNull();
    expect(screen.queryByRole("checkbox", { name: /重新完整下载/ })).toBeNull();
  });

  it("warns that the backup will be replaced when the rules differ from the backup's", () => {
    renderForm({ existingBackup: { ...backup, filter: "original" } });

    expect(screen.getByText(/会重新完整下载并替换它/)).toBeTruthy();
    expect(screen.queryByText(/只会补上新微博/)).toBeNull();
  });

  it("says a full download was asked for, instead of promising an update", () => {
    renderForm({ existingBackup: backup, freshDownload: true });

    expect(screen.getByText(/已勾选「重新完整下载」/)).toBeTruthy();
    expect(screen.queryByText(/只会补上新微博/)).toBeNull();
  });
});
