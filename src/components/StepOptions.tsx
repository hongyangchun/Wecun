import { useState, useEffect } from "react";
import { SegmentedControl, FormField } from "./ui";
import type { PostFilter, DownloadRange } from "../types/contracts";

export type { PostFilter, DownloadRange } from "../types/contracts";

interface AdvancedOptionsProps {
  postFilter: PostFilter;
  onPostFilterChange: (f: PostFilter) => void;
  minTextLength: number;
  onMinTextLengthChange: (v: number) => void;
  ignoreDeleted: boolean;
  onIgnoreDeletedChange: (v: boolean) => void;
  sourceType: "profile" | "favorites";
}

function MinLengthInput({ value, onChange }: { value: number; onChange: (n: number) => void }) {
  const [str, setStr] = useState(String(value));

  useEffect(() => {
    setStr(String(value));
  }, [value]);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
      <input
        type="number"
        className="input"
        style={{ width: 70, height: 32 }}
        value={str}
        onChange={(e) => setStr(e.target.value)}
        onBlur={() => onChange(Math.max(0, Math.min(500, parseInt(str) || 0)))}
        min={0}
        max={500}
      />
      <span className="form-hint" style={{ margin: 0 }}>字（0 = 不过滤，推荐 20）</span>
    </div>
  );
}

function AdvancedOptions({
  postFilter,
  onPostFilterChange,
  minTextLength,
  onMinTextLengthChange,
  ignoreDeleted,
  onIgnoreDeletedChange,
  sourceType,
}: AdvancedOptionsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div style={{ marginTop: 24 }}>
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          background: "none",
          border: "none",
          padding: 0,
          color: "var(--color-text-tertiary)",
          fontSize: 12,
          fontWeight: 500,
          cursor: "pointer",
          transition: "color 120ms ease",
        }}
        onMouseEnter={(e) => e.currentTarget.style.color = "var(--color-text-secondary)"}
        onMouseLeave={(e) => e.currentTarget.style.color = "var(--color-text-tertiary)"}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          style={{
            transition: "transform 200ms cubic-bezier(0.16, 1, 0.3, 1)",
            transform: isExpanded ? "rotate(90deg)" : "rotate(0deg)",
          }}
        >
          <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        {isExpanded ? "收起" : "高级设置"}
      </button>

      {isExpanded && (
        <div style={{ marginTop: 12, padding: "12px 16px", background: "var(--color-bg-inset)", borderRadius: 8, display: "flex", flexDirection: "column", gap: 12 }}>
          {sourceType === "profile" && (
            <>
              <label className="check-row" style={{ margin: 0, padding: 0 }}>
                <input
                  type="checkbox"
                  checked={postFilter === "original"}
                  onChange={(e) => onPostFilterChange(e.target.checked ? "original" : "all")}
                />
                <div>
                  <span className="check-label">仅原创（不包含转发）</span>
                </div>
              </label>

              <label className="check-row" style={{ margin: 0, padding: 0 }}>
                <input
                  type="checkbox"
                  checked={minTextLength > 0}
                  onChange={(e) => onMinTextLengthChange(e.target.checked ? 20 : 0)}
                />
                <div>
                  <span className="check-label">忽略短微博</span>
                  <span className="check-hint">过滤字数少于</span>
                  <MinLengthInput value={minTextLength} onChange={onMinTextLengthChange} />
                </div>
              </label>
            </>
          )}

          {sourceType === "favorites" && (
            <label className="check-row" style={{ margin: 0, padding: 0 }}>
              <input
                type="checkbox"
                checked={ignoreDeleted}
                onChange={(e) => onIgnoreDeletedChange(e.target.checked)}
              />
              <div>
                <span className="check-label">忽略已删除微博</span>
                <span className="check-hint">跳过在原微博主页已显示为删除的内容</span>
              </div>
            </label>
          )}
        </div>
      )}
    </div>
  );
}

interface StepOptionsProps {
  postFilter: PostFilter;
  onPostFilterChange: (f: PostFilter) => void;
  includeImages: boolean;
  onIncludeImagesChange: (v: boolean) => void;
  downloadRange: DownloadRange;
  onDownloadRangeChange: (r: DownloadRange) => void;
  dateStart: string;
  onDateStartChange: (d: string) => void;
  dateEnd: string;
  onDateEndChange: (d: string) => void;
  ignoreDeleted: boolean;
  onIgnoreDeletedChange: (v: boolean) => void;
  minTextLength: number;
  onMinTextLengthChange: (v: number) => void;
  sourceType?: "profile" | "favorites";
}

export default function StepOptions({
  postFilter,
  onPostFilterChange,
  includeImages,
  onIncludeImagesChange,
  downloadRange,
  onDownloadRangeChange,
  dateStart,
  onDateStartChange,
  dateEnd,
  onDateEndChange,
  ignoreDeleted,
  onIgnoreDeletedChange,
  minTextLength,
  onMinTextLengthChange,
  sourceType = "profile",
}: StepOptionsProps) {
  return (
    <div className="step-enter">
      <h2 className="step-heading" style={{ marginBottom: 20 }}>
        下载选项
      </h2>

      {sourceType === "profile" && (
        <div className="step-section" style={{ marginBottom: 20 }}>
          <FormField
            label="下载范围"
            hint="选择要下载的时间范围"
          >
            <SegmentedControl
              options={[
                { value: "all", label: "全部" },
                { value: "range", label: "指定时间段" },
              ]}
              value={downloadRange}
              onChange={onDownloadRangeChange}
              ariaLabel="下载范围"
            />
            {downloadRange === "range" && (
              <div style={{ marginTop: 12, display: "flex", alignItems: "center", gap: 12 }}>
                <input
                  type="date"
                  className="input"
                  style={{ height: 36, flex: 1 }}
                  value={dateStart}
                  onChange={(e) => onDateStartChange(e.target.value)}
                />
                <span style={{ fontSize: 13, color: "var(--color-text-tertiary)" }}>至</span>
                <input
                  type="date"
                  className="input"
                  style={{ height: 36, flex: 1 }}
                  value={dateEnd}
                  onChange={(e) => onDateEndChange(e.target.value)}
                />
              </div>
            )}
          </FormField>
        </div>
      )}

      <div style={{ marginBottom: 20 }}>
        <label className="check-row">
          <input
            type="checkbox"
            checked={includeImages}
            onChange={(e) => onIncludeImagesChange(e.target.checked)}
          />
          <div>
            <span className="check-label">包含图片</span>
            <span className="check-hint">同时下载微博中的图片</span>
          </div>
        </label>
      </div>

      <AdvancedOptions
        postFilter={postFilter}
        onPostFilterChange={onPostFilterChange}
        minTextLength={minTextLength}
        onMinTextLengthChange={onMinTextLengthChange}
        ignoreDeleted={ignoreDeleted}
        onIgnoreDeletedChange={onIgnoreDeletedChange}
        sourceType={sourceType}
      />
    </div>
  );
}
