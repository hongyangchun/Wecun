import { describe, expect, it } from "vitest";
import { canUpdateBackup, dateToStartTimestamp, type FormValues } from "../lib/validation";
import type { BackupMeta } from "../types/contracts";

const form: FormValues = {
  uid: "2166767661",
  cookie: "",
  filter: "original",
  includeImages: true,
  downloadRange: "all",
  dateStart: "",
  dateEnd: "",
  outputDir: "/tmp/weibo",
  ignoreDeleted: true,
  minTextLength: 20,
  limit: 1000,
  sourceType: "profile",
  fresh: false,
};

const backup: BackupMeta = {
  uid: "2166767661",
  source_type: "profile",
  post_count: 40,
  author: "测试",
  filter: "original",
  min_text_length: 20,
  ignore_deleted: true,
  start_timestamp: null,
  end_timestamp: null,
};

describe("canUpdateBackup", () => {
  it("updates a backup that was made the same way, even when the user now wants a narrower window", () => {
    expect(canUpdateBackup(backup, form)).toBe(true);
    expect(canUpdateBackup(backup, { ...form, downloadRange: "range", dateStart: "2024-01-10", dateEnd: "2024-01-20" })).toBe(true);
  });

  it("starts over when the content rules differ", () => {
    expect(canUpdateBackup(backup, { ...form, filter: "all" })).toBe(false);
    expect(canUpdateBackup(backup, { ...form, minTextLength: 0 })).toBe(false);
    expect(canUpdateBackup(backup, { ...form, ignoreDeleted: false })).toBe(false);
  });

  it("starts over when the window reaches further back than the backup did", () => {
    const ranged: BackupMeta = {
      ...backup,
      start_timestamp: dateToStartTimestamp("2024-01-10"),
      end_timestamp: dateToStartTimestamp("2024-01-20"),
    };

    expect(canUpdateBackup(ranged, form), "all-time now, a window before").toBe(false);
    expect(canUpdateBackup(ranged, { ...form, downloadRange: "range", dateStart: "2024-01-05", dateEnd: "2024-01-20" })).toBe(false);
    expect(canUpdateBackup(ranged, { ...form, downloadRange: "range", dateStart: "2024-01-10", dateEnd: "2024-01-25" })).toBe(true);
  });

  it("promises nothing while a window is selected but its start date is missing", () => {
    expect(canUpdateBackup(backup, { ...form, downloadRange: "range", dateStart: "", dateEnd: "2024-01-20" })).toBe(false);
  });
});
