import type { DownloadOutcome } from "../types/contracts";

/** Plain-language warnings for everything a finished download could not get. Empty means nothing went wrong. */
export function describeDownloadIssues(outcome: DownloadOutcome): string[] {
  const issues: string[] = [];
  if (outcome.longTextFailed > 0) {
    issues.push(`${outcome.longTextFailed} 条长微博未能获取全文，已保留截断内容，可通过「原文链接」查看`);
  }
  if (outcome.imageFailed > 0) {
    issues.push(`${outcome.imageFailed} 张图片下载失败，导出文件中将使用原始链接`);
  }
  if (outcome.stoppedEarly) {
    issues.push(`备份可能不完整：${outcome.stoppedEarly}`);
  }
  return issues;
}
