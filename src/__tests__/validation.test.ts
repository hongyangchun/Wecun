import { describe, expect, it } from "vitest";
import {
  buildDownloadRequest,
  dateToEndTimestamp,
  dateToStartTimestamp,
  extractUidFromUrl,
  isValidProfileUrl,
  isValidUid,
} from "../lib/validation";

const uidExtractionCases: Array<[string, string]> = [
  ["2166767661", "2166767661"],
  [" https://www.weibo.com/u/2166767661 ", "2166767661"],
  ["weibo.com/u/2166767661", "2166767661"],
  ["https://m.weibo.com/profile/2166767661", "2166767661"],
  ["https://weibo.com/2166767661", "2166767661"],
  ["https://weibo.com/p/1005052166767661?uid=2166767661", "2166767661"],
  ["not a url /u/2166767661?foo=bar", "2166767661"],
];

const invalidUidExtractionInputs: string[] = [
  "",
  "abcde",
  "https://weibo.com/u/not-a-uid",
  "https://weibo.com/profile/1234",
  "https://example.com/user/abcdef",
];

const validProfileInputs: string[] = [
  "2166767661",
  "https://www.weibo.com/u/2166767661",
  "weibo.com/profile/2166767661",
  "https://weibo.com/2166767661",
];

const invalidProfileInputs: string[] = ["", "weibo.com/profile/not-valid", "uid=12", "hello world"];

describe("validation utilities", () => {
  describe("extractUidFromUrl", () => {
    it.each(uidExtractionCases)("extracts uid from %s", (input: string, expected: string) => {
      expect(extractUidFromUrl(input)).toBe(expected);
    });

    it.each(invalidUidExtractionInputs)("returns null for invalid input %s", (input: string) => {
      expect(extractUidFromUrl(input)).toBeNull();
    });
  });

  describe("isValidProfileUrl", () => {
    it.each(validProfileInputs)("returns true for %s", (input: string) => {
      expect(isValidProfileUrl(input)).toBe(true);
    });

    it.each(invalidProfileInputs)("returns false for %s", (input: string) => {
      expect(isValidProfileUrl(input)).toBe(false);
    });
  });

  describe("isValidUid", () => {
    it("accepts trimmed numeric uids with 5-20 digits", () => {
      expect(isValidUid(" 2166767661 ")).toBe(true);
      expect(isValidUid("12345")).toBe(true);
      expect(isValidUid("12345678901234567890")).toBe(true);
    });

    it("rejects non-numeric or out-of-range values", () => {
      expect(isValidUid("1234")).toBe(false);
      expect(isValidUid("123456789012345678901")).toBe(false);
      expect(isValidUid("12ab34")).toBe(false);
    });
  });

  describe("date conversion", () => {
    it("converts start dates to 00:00:00 +08:00 timestamps", () => {
      expect(dateToStartTimestamp("")).toBe(0);
      expect(dateToStartTimestamp("2024-01-01")).toBe(1704038400);
    });

    it("converts end dates to 23:59:59 +08:00 timestamps", () => {
      expect(dateToEndTimestamp("")).toBe(0);
      expect(dateToEndTimestamp("2024-01-01")).toBe(1704124799);
    });
  });

  describe("buildDownloadRequest", () => {
    it("builds a ranged request with timestamps and trimmed credentials", () => {
      expect(
        buildDownloadRequest({
          uid: " 2166767661 ",
          cookie: " SUB=cookie; ",
          filter: "original",
          includeImages: true,
          dateMode: "range",
          dateStart: "2024-01-01",
          dateEnd: "2024-01-31",
          outputDir: "/tmp/weibo",
          ignoreDeleted: true,
          sourceType: "profile",
        }),
      ).toEqual({
        uid: "2166767661",
        cookie: "SUB=cookie;",
        filter: "original",
        include_images: true,
        date_range: {
          start_timestamp: 1704038400,
          end_timestamp: 1706716799,
        },
        output_dir: "/tmp/weibo",
        ignore_deleted: true,
        source_type: "profile",
      });
    });

    it("builds an all-date request with null timestamps", () => {
      expect(
        buildDownloadRequest({
          uid: "2166767661",
          cookie: "SUB=cookie;",
          filter: "all",
          includeImages: false,
          dateMode: "all",
          dateStart: "2024-01-01",
          dateEnd: "2024-01-31",
          outputDir: "/tmp/weibo",
          ignoreDeleted: false,
          sourceType: "profile",
        }),
      ).toEqual({
        uid: "2166767661",
        cookie: "SUB=cookie;",
        filter: "all",
        include_images: false,
        date_range: {
          start_timestamp: null,
          end_timestamp: null,
        },
        output_dir: "/tmp/weibo",
        ignore_deleted: false,
        source_type: "profile",
      });
    });
  });
});
