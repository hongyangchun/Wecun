import type { DownloadRequest } from "../types/contracts";

export function extractUidFromUrl(url: string): string | null {
  const trimmed = url.trim();
  if (/^\d{5,20}$/.test(trimmed)) return trimmed;

  try {
    const normalized = /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
    const parsed = new URL(normalized);

    const uidParam = parsed.searchParams.get("uid");
    if (uidParam && /^\d{5,20}$/.test(uidParam)) {
      return uidParam;
    }

    const pathname = parsed.pathname.replace(/\/+$/, "");
    const directPatterns = [
      /\/u\/(\d{5,20})$/i,
      /\/profile\/(\d{5,20})$/i,
      /\/(\d{5,20})$/i,
    ];

    for (const pattern of directPatterns) {
      const match = pathname.match(pattern);
      if (match) return match[1];
    }
  } catch {
    void 0;
  }

  const fallbackMatch = trimmed.match(/(?:\/u\/|uid=|\/profile\/|\/)(\d{5,20})(?:[/?#]|$)/i);
  return fallbackMatch ? fallbackMatch[1] : null;
}

export function isValidProfileUrl(input: string): boolean {
  const trimmed = input.trim();
  if (/^\d{5,20}$/.test(trimmed)) return true;
  return extractUidFromUrl(trimmed) !== null;
}

export function isValidUid(uid: string): boolean {
  return /^\d{5,20}$/.test(uid.trim());
}

export function isValidCookie(cookie: string): boolean {
  return cookie.includes("SUB=");
}

export function dateToStartTimestamp(dateStr: string): number {
  if (!dateStr) return 0;
  const d = new Date(dateStr + "T00:00:00+08:00");
  return Math.floor(d.getTime() / 1000);
}

export function dateToEndTimestamp(dateStr: string): number {
  if (!dateStr) return 0;
  const d = new Date(dateStr + "T23:59:59+08:00");
  return Math.floor(d.getTime() / 1000);
}

export interface FormValues {
  uid: string;
  cookie: string;
  filter: "original" | "all";
  includeImages: boolean;
  downloadRange: "all" | "range" | "limit";
  dateStart: string;
  dateEnd: string;
  outputDir: string;
  ignoreDeleted: boolean;
  minTextLength: number;
  limit: number;
  sourceType: "profile" | "favorites";
}

export function buildDownloadRequest(values: FormValues): DownloadRequest {
  return {
    uid: values.uid.trim(),
    cookie: values.cookie.trim(),
    filter: values.filter,
    include_images: values.includeImages,
    date_range: {
      start_timestamp: values.downloadRange === "range" ? dateToStartTimestamp(values.dateStart) : null,
      end_timestamp: values.downloadRange === "range" ? dateToEndTimestamp(values.dateEnd) : null,
    },
    output_dir: values.outputDir,
    ignore_deleted: values.ignoreDeleted,
    min_text_length: values.minTextLength,
    limit: values.limit,
    source_type: values.sourceType,
  };
}
