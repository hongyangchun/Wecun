import type { HistoryEntry } from "../types/contracts";

interface HistoryListItemProps {
  entry: HistoryEntry;
  onExport: () => void;
  onDelete: () => void;
  onOpenDir: () => void;
}

function formatDate(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return "今天";
  if (diffDays === 1) return "昨天";
  if (diffDays < 7) return `${diffDays} 天前`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)} 周前`;
  return `${Math.floor(diffDays / 30)} 月前`;
}

function formatFilter(filter?: string): string {
  if (!filter) return "全部";
  if (filter === "original") return "仅原创";
  return "全部";
}

function formatDirPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export default function HistoryListItem({
  entry,
  onExport,
  onDelete,
  onOpenDir,
}: HistoryListItemProps) {
  return (
    <div className="history-list-item">
      <div className="history-item-header">
        <span className="history-item-name">{entry.screen_name}</span>
        <span className="history-item-time">{formatDate(entry.last_download)}</span>
      </div>

      <div className="history-item-meta">
        <span>{entry.post_count} 条微博</span>
        <span>·</span>
        <span className="history-item-path" title={entry.output_dir}>
          {formatDirPath(entry.output_dir)}
        </span>
      </div>

      <div className="history-item-options">
        选项: {formatFilter(entry.filter)}
        {entry.include_images && " · 含图片"}
        · {entry.date_mode === "all" ? "全部时间" : "指定时间范围"}
      </div>

      <div className="history-item-actions">
        <button
          className="btn-small btn-small--primary"
          type="button"
          onClick={onExport}
          title="导出"
        >
          导出
        </button>
        <button
          className="btn-small"
          type="button"
          onClick={onOpenDir}
          title="打开目录"
        >
          打开
        </button>
        <button
          className="btn-small btn-small--danger"
          type="button"
          onClick={onDelete}
          title="删除记录"
        >
          删除
        </button>
      </div>
    </div>
  );
}
