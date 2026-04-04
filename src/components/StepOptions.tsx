export type PostFilter = "original" | "all";
export type DateMode = "all" | "range";
export type ExportFormat = "pdf" | "md-single" | "md-multi";

interface StepOptionsProps {
  postFilter: PostFilter;
  onPostFilterChange: (f: PostFilter) => void;
  includeImages: boolean;
  onIncludeImagesChange: (v: boolean) => void;
  dateMode: DateMode;
  onDateModeChange: (m: DateMode) => void;
  dateStart: string;
  onDateStartChange: (d: string) => void;
  dateEnd: string;
  onDateEndChange: (d: string) => void;
  exportFormat: ExportFormat;
  onExportFormatChange: (f: ExportFormat) => void;
  minTextLength: number;
  onMinTextLengthChange: (n: number) => void;
  onBack: () => void;
  onNext: () => void;
}

export default function StepOptions({
  postFilter, onPostFilterChange,
  includeImages, onIncludeImagesChange,
  dateMode, onDateModeChange,
  dateStart, onDateStartChange,
  dateEnd, onDateEndChange,
  exportFormat, onExportFormatChange,
  minTextLength, onMinTextLengthChange,
  onBack, onNext,
}: StepOptionsProps) {
  return (
    <div className="step-content step-options">
      <div className="step-icon">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <circle cx="24" cy="24" r="22" fill="#E8F0FE" />
          <path d="M14 24H34M14 18H34M14 30H26" stroke="#4F6EF7" strokeWidth="2.5" strokeLinecap="round"/>
        </svg>
      </div>
      <h2 className="step-title">选择下载选项</h2>

      <div className="option-group">
        <label className="option-label">微博类型</label>
        <div className="option-toggle">
          <button className={`toggle-btn${postFilter === "original" ? " active" : ""}`} onClick={() => onPostFilterChange("original")} type="button">仅原创</button>
          <button className={`toggle-btn${postFilter === "all" ? " active" : ""}`} onClick={() => onPostFilterChange("all")} type="button">全部微博</button>
        </div>
      </div>

      <div className="option-group">
        <label className="option-checkbox">
          <input type="checkbox" checked={includeImages} onChange={(e) => onIncludeImagesChange(e.target.checked)} />
          <span>包含图片</span>
        </label>
      </div>

      <div className="option-group">
        <label className="option-label">时间范围</label>
        <div className="option-toggle">
          <button className={`toggle-btn${dateMode === "all" ? " active" : ""}`} onClick={() => onDateModeChange("all")} type="button">全部时间</button>
          <button className={`toggle-btn${dateMode === "range" ? " active" : ""}`} onClick={() => onDateModeChange("range")} type="button">指定时间段</button>
        </div>
        {dateMode === "range" && (
          <div className="date-row">
            <input type="date" className="wizard-input" value={dateStart} onChange={(e) => onDateStartChange(e.target.value)} />
            <span className="date-sep">至</span>
            <input type="date" className="wizard-input" value={dateEnd} onChange={(e) => onDateEndChange(e.target.value)} />
          </div>
        )}
      </div>

      <div className="option-group">
        <label className="option-label">导出格式</label>
        <div className="option-toggle option-toggle--three">
          <button className={`toggle-btn${exportFormat === "pdf" ? " active" : ""}`} onClick={() => onExportFormatChange("pdf")} type="button">PDF</button>
          <button className={`toggle-btn${exportFormat === "md-single" ? " active" : ""}`} onClick={() => onExportFormatChange("md-single")} type="button">Markdown</button>
          <button className={`toggle-btn${exportFormat === "md-multi" ? " active" : ""}`} onClick={() => onExportFormatChange("md-multi")} type="button">每博一文</button>
        </div>
        <p className="option-hint">
          {exportFormat === "pdf" && "所有微博合并为一个 PDF 文件"}
          {exportFormat === "md-single" && "所有微博合并为一个 Markdown 文件"}
          {exportFormat === "md-multi" && "每条微博一个独立的 Markdown 文件"}
        </p>
      </div>

      <div className="option-group">
        <label className="option-label">忽略短微博</label>
        <div className="min-length-row">
          <input
            type="number"
            className="wizard-input min-length-input"
            value={minTextLength}
            onChange={(e) => onMinTextLengthChange(Math.max(0, parseInt(e.target.value) || 0))}
            min={0}
            max={500}
          />
          <span className="min-length-hint">字数少于该值的微博将被跳过（0 = 不过滤）</span>
        </div>
      </div>

      <div className="wizard-actions">
        <button className="btn-wizard btn-wizard-secondary" onClick={onBack} type="button">上一步</button>
        <button className="btn-wizard btn-wizard-primary" onClick={onNext} type="button">下一步</button>
      </div>
    </div>
  );
}
