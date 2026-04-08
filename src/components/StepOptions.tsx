import { useState, useEffect } from "react";
import { SegmentedControl, FormField } from "./ui";
import type { PostFilter, DateMode } from "../types/contracts";

export type { PostFilter, DateMode } from "../types/contracts";

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
  ignoreDeleted: boolean;
  onIgnoreDeletedChange: (v: boolean) => void;
  minTextLength: number;
  onMinTextLengthChange: (v: number) => void;
  sourceType?: "profile" | "favorites";
}

function MinLengthInput({ value, onChange }: { value: number; onChange: (n: number) => void }) {
  const [str, setStr] = useState(String(value));

  useEffect(() => {
    setStr(String(value));
  }, [value]);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
      <input
        type="number"
        className="input"
        style={{ width: 80, height: 36 }}
        value={str}
        onChange={(e) => setStr(e.target.value)}
        onBlur={() => onChange(Math.max(0, Math.min(500, parseInt(str) || 0)))}
        min={0}
        max={500}
      />
      <span className="form-hint" style={{ margin: 0 }}>
        字数少于该值的微博将被跳过（0 = 不过滤）
      </span>
    </div>
  );
}

export default function StepOptions({
  postFilter,
  onPostFilterChange,
  includeImages,
  onIncludeImagesChange,
  dateMode,
  onDateModeChange,
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
          <FormField label="发布时间范围">
            <SegmentedControl
              options={[
                { value: "all", label: "全部" },
                { value: "range", label: "指定时间段" },
              ]}
              value={dateMode}
              onChange={onDateModeChange}
              ariaLabel="时间范围"
            />
            {dateMode === "all" && (
              <div style={{ marginTop: 12, padding: "10px 12px", borderRadius: 8, background: "rgba(255, 149, 0, 0.1)", border: "1px solid rgba(255, 149, 0, 0.3)" }}>
                <p className="form-hint" style={{ margin: 0, color: "var(--color-text)", display: "flex", gap: 6 }}>
                  <span style={{ fontSize: 14 }}>⚠️</span>
                  <span>
                    <b>安全建议</b>：一次性下载海量微博（如超过 2000 条）可能触发风控，导致账号被限制访问。建议按年份或月份分批下载。
                  </span>
                </p>
              </div>
            )}
            {dateMode === "range" && (
              <div style={{ marginTop: 12, display: "flex", flexDirection: "column", gap: 12 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
                  <input
                    type="date"
                    className="input"
                    style={{ height: 36 }}
                    value={dateStart}
                    onChange={(e) => onDateStartChange(e.target.value)}
                  />
                  <span style={{ fontSize: 13, color: "var(--color-text-tertiary)" }}>至</span>
                  <input
                    type="date"
                    className="input"
                    style={{ height: 36 }}
                    value={dateEnd}
                    onChange={(e) => onDateEndChange(e.target.value)}
                  />
                </div>
              </div>
            )}
          </FormField>
        </div>
      )}

      {sourceType === "profile" && (
        <div style={{ marginBottom: 20 }}>
          <FormField label="微博类型">
            <SegmentedControl
              options={[
                { value: "original", label: "仅原创" },
                { value: "all", label: "全部" },
              ]}
              value={postFilter}
              onChange={onPostFilterChange}
              ariaLabel="微博类型"
            />
          </FormField>
        </div>
      )}

      <div style={{ display: "flex", gap: 32, marginBottom: 20 }}>
        <label className="check-row" style={{ margin: 0 }}>
          <input
            type="checkbox"
            checked={includeImages}
            onChange={(e) => onIncludeImagesChange(e.target.checked)}
          />
          <div>
            <span className="check-label">包含图片</span>
          </div>
        </label>

        <label className="check-row" style={{ margin: 0 }}>
          <input
            type="checkbox"
            checked={ignoreDeleted}
            onChange={(e) => onIgnoreDeletedChange(e.target.checked)}
          />
          <div>
            <span className="check-label">忽略已删除微博</span>
          </div>
        </label>
      </div>

      {sourceType === "profile" && (
        <div className="step-section">
          <FormField label="忽略短微博" hint="推荐默认值 20">
            <MinLengthInput value={minTextLength} onChange={onMinTextLengthChange} />
          </FormField>
        </div>
      )}

      {sourceType === "favorites" && (
        <div className="step-section" style={{ border: "1px dashed var(--color-border)", padding: "12px 16px", borderRadius: 8 }}>
          <p className="form-hint" style={{ margin: 0, color: "var(--color-text-secondary)" }}>
            收藏模式下将尝试下载账号内的<b>全部</b>收藏微博（不限发布时间）。
          </p>
        </div>
      )}
    </div>
  );
}
