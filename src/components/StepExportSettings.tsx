import { pickDirectory } from "../lib/dialog";
import type { ExportFormat } from "../types/contracts";

const ALL_FORMATS: { value: ExportFormat; label: string; desc: string }[] = [
  { value: "html", label: "HTML", desc: "适合直接在浏览器中查看和分享" },
  { value: "md-single", label: "Markdown（单文件）", desc: "适合整理成一份完整备份" },
  { value: "md-multi", label: "Markdown（分文件）", desc: "每条微博一个独立文件，适合进一步整理" },
];

interface StepExportSettingsProps {
  exportFormat: ExportFormat;
  onExportFormatChange: (f: ExportFormat) => void;
  outputDir: string;
  onDirChange: (dir: string) => void;
}

export default function StepExportSettings({
  exportFormat,
  onExportFormatChange,
  outputDir,
  onDirChange,
}: StepExportSettingsProps) {
  const handlePick = async () => {
    const dir = await pickDirectory();
    if (dir) onDirChange(dir);
  };

  return (
    <div className="step-enter">
      <h2 style={{ fontSize: 17, fontWeight: 700, color: "var(--color-text)", marginBottom: 6, letterSpacing: "-0.01em" }}>
        导出设置
      </h2>
      <p className="form-hint" style={{ marginBottom: 20 }}>选择导出格式和保存位置</p>

      <div style={{ marginBottom: 20 }}>
        {ALL_FORMATS.map((fmt) => {
          const isActive = exportFormat === fmt.value;
          return (
            <label
              key={fmt.value}
              className={`radio-card ${isActive ? "radio-card--active" : ""}`}
            >
              <input
                type="radio"
                name="export-format"
                checked={isActive}
                onChange={() => onExportFormatChange(fmt.value)}
              />
              <div>
                <div className="radio-card-title">{fmt.label}</div>
                <div className="radio-card-desc">{fmt.desc}</div>
              </div>
            </label>
          );
        })}
      </div>

      <div className="divider" />

      <div className="form-field">
        <span className="form-label">保存位置</span>
        <button className="btn btn-outline-accent" style={{ width: "100%", height: 40 }} onClick={handlePick} type="button">
          选择目录
        </button>
        {outputDir && (
          <div className="dir-display">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" style={{ flexShrink: 0, marginTop: 2 }}>
              <path d="M2 4H5L6.5 6H12V11H2V4Z" stroke="var(--color-text-tertiary)" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="dir-display-path">{outputDir}</span>
          </div>
        )}
      </div>
    </div>
  );
}
