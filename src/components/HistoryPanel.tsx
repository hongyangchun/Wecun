import { useEffect, useState } from "react";
import HistoryListItem from "./HistoryListItem";
import HistoryExportDialog from "./HistoryExportDialog";
import {
  listDownloadHistory,
  deleteHistoryEntry,
  exportFromHistory,
  openOutputDir,
} from "../lib/tauri-bridge";
import type { HistoryEntry, ExportFormat } from "../types/contracts";

interface HistoryPanelProps {
  isOpen: boolean;
  onClose: () => void;
  onLog: (message: string) => void;
}

export default function HistoryPanel({ isOpen, onClose, onLog }: HistoryPanelProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [exportingUid, setExportingUid] = useState<string | null>(null);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [selectedUid, setSelectedUid] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadHistory();
    }
  }, [isOpen]);

  async function loadHistory() {
    setLoading(true);
    try {
      const history = await listDownloadHistory();
      setEntries(history);
    } catch (error) {
      onLog(`加载历史记录失败: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(uid: string) {
    if (!confirm("确定要删除这条历史记录吗？这不会删除已下载的文件。")) {
      return;
    }

    try {
      await deleteHistoryEntry(uid);
      setEntries((prev) => prev.filter((e) => e.uid !== uid));
      onLog("历史记录已删除");
    } catch (error) {
      onLog(`删除失败: ${error}`);
    }
  }

  async function handleExport(format: ExportFormat) {
    if (!selectedUid) return;

    setExportingUid(selectedUid);
    setShowExportDialog(false);

    try {
      onLog(`正在导出 ${format}...`);
      const result = await exportFromHistory(selectedUid, format);
      onLog(result);
    } catch (error) {
      onLog(String(error));
    } finally {
      setExportingUid(null);
      setSelectedUid(null);
    }
  }

  function openExportDialog(uid: string) {
    setSelectedUid(uid);
    setShowExportDialog(true);
  }

  async function handleOpenDir(outputDir: string) {
    try {
      await openOutputDir(outputDir);
    } catch (error) {
      onLog(`打开目录失败: ${error}`);
    }
  }

  if (!isOpen) return null;

  return (
    <>
      <div className="history-overlay" onClick={onClose} />
      <aside className="history-panel">
        <div className="history-header">
          <h2>历史记录</h2>
          <button className="history-close" onClick={onClose} type="button">
            ×
          </button>
        </div>

        {loading ? (
          <div className="history-loading">加载中...</div>
        ) : entries.length === 0 ? (
          <div className="history-empty">
            <p>暂无下载历史</p>
            <p style={{ fontSize: 12, opacity: 0.7 }}>先去下载一些微博吧</p>
          </div>
        ) : (
          <div className="history-list">
            {entries.map((entry) => (
              <HistoryListItem
                key={entry.uid}
                entry={entry}
                onExport={() => openExportDialog(entry.uid)}
                onDelete={() => handleDelete(entry.uid)}
                onOpenDir={() => handleOpenDir(entry.output_dir)}
              />
            ))}
          </div>
        )}
      </aside>

      <HistoryExportDialog
        isOpen={showExportDialog}
        onClose={() => {
          setShowExportDialog(false);
          setSelectedUid(null);
        }}
        onExport={handleExport}
        isExporting={exportingUid !== null}
      />
    </>
  );
}
