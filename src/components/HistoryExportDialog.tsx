import type { ExportFormat } from "../types/contracts";

interface HistoryExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onExport: (format: ExportFormat) => void;
  isExporting: boolean;
}

const exportOptions: Array<{ format: ExportFormat; label: string; desc: string }> = [
  { format: "html", label: "HTML", desc: "浏览器查看" },
  { format: "md-single", label: "Markdown", desc: "单文件备份" },
  { format: "md-obsidian", label: "Obsidian", desc: "带 frontmatter" },
  { format: "pdf", label: "PDF", desc: "精美排版" },
];

export default function HistoryExportDialog({
  isOpen,
  onClose,
  onExport,
  isExporting,
}: HistoryExportDialogProps) {
  if (!isOpen) return null;

  return (
    <div className="dialog-overlay">
      <div className="dialog-card" style={{ maxWidth: 360 }}>
        <h3 style={{ fontSize: 16, fontWeight: 600, marginBottom: 16 }}>选择导出格式</h3>

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {exportOptions.map((option) => (
            <button
              key={option.format}
              className="btn btn-secondary"
              type="button"
              onClick={() => onExport(option.format)}
              disabled={isExporting}
              style={{
                width: "100%",
                textAlign: "left",
                padding: "12px 16px",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <span style={{ fontSize: 14, fontWeight: 600 }}>{option.label}</span>
              <span style={{ fontSize: 12, opacity: 0.7 }}>{option.desc}</span>
            </button>
          ))}
        </div>

        <div style={{ marginTop: 16 }}>
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onClose}
            disabled={isExporting}
            style={{ width: "100%" }}
          >
            取消
          </button>
        </div>
      </div>
    </div>
  );
}
