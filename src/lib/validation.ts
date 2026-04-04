import type { DownloadRequest } from "../types/contracts";

// Extract numeric uid from a Weibo profile URL
// Supports: https://www.weibo.com/u/1234567890, https://weibo.com/u/1234567890, etc.
export function extractUidFromUrl(url: string): string | null {
  const match = url.match(/\/u\/(\d+)/);
  return match ? match[1] : null;
}

// Validate: accepts either a raw uid (5-20 digits) or a full weibo URL containing /u/{uid}
export function isValidProfileUrl(input: string): boolean {
  const trimmed = input.trim();
  if (/^\d{5,20}$/.test(trimmed)) return true;
  return extractUidFromUrl(trimmed) !== null;
}

// Validate UID: must be numeric digits only, 5-20 chars
export function isValidUid(uid: string): boolean {
  return /^\d{5,20}$/.test(uid.trim());
}

// Validate cookie: must contain SUB= substring
export function isValidCookie(cookie: string): boolean {
  return cookie.includes("SUB=");
}

// Convert YYYY-MM-DD to unix timestamp (seconds)
// For start date: 00:00:00 of that day (UTC+8)
export function dateToStartTimestamp(dateStr: string): number {
  if (!dateStr) return 0;
  const d = new Date(dateStr + "T00:00:00+08:00");
  return Math.floor(d.getTime() / 1000);
}

// For end date: 23:59:59 of that day (UTC+8)
export function dateToEndTimestamp(dateStr: string): number {
  if (!dateStr) return 0;
  const d = new Date(dateStr + "T23:59:59+08:00");
  return Math.floor(d.getTime() / 1000);
}

// Build DownloadRequest from form values
export interface FormValues {
  uid: string;
  cookie: string;
  filter: "original" | "all";
  includeImages: boolean;
  dateMode: "all" | "range";
  dateStart: string;
  dateEnd: string;
  exportFormat: "pdf" | "md-single" | "md-multi";
  outputDir: string;
  minTextLength: number;
}

export function buildDownloadRequest(values: FormValues): DownloadRequest {
  return {
    uid: values.uid.trim(),
    cookie: values.cookie.trim(),
    filter: values.filter,
    include_images: values.includeImages,
    date_range: {
      start_timestamp: values.dateMode === "range" ? dateToStartTimestamp(values.dateStart) : null,
      end_timestamp: values.dateMode === "range" ? dateToEndTimestamp(values.dateEnd) : null,
    },
    export_format: values.exportFormat,
    output_dir: values.outputDir,
    min_text_length: values.minTextLength,
  };
}
