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
  minTextLength: number;
  onMinTextLengthChange: (n: number) => void;
}

function MinLengthInput({ value, onChange }: { value: number; onChange: (n: number) => void }) {
  const [str, setStr] = useState(String(value));

  useEffect(() => {
    setStr(String(value));
  }, [value]);

  return (
    <div className="flex items-center gap-10">
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
  minTextLength,
  onMinTextLengthChange,
}: StepOptionsProps) {
  return (
    <div className="step-enter">
      <h2 className="step-heading" style={{ marginBottom: 20 }}>
        下载选项
      </h2>

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
        <label className="check-row" style={{ marginTop: 12 }}>
          <input
            type="checkbox"
            checked={includeImages}
            onChange={(e) => onIncludeImagesChange(e.target.checked)}
          />
          <div>
            <span className="check-label">包含图片</span>
          </div>
        </label>
      </div>

      <div className="step-section">
        <FormField label="时间范围">
          <SegmentedControl
            options={[
              { value: "all", label: "全部" },
              { value: "range", label: "指定时间段" },
            ]}
            value={dateMode}
            onChange={onDateModeChange}
            ariaLabel="时间范围"
          />
          {dateMode === "range" && (
            <div className="flex items-center gap-10" style={{ marginTop: 12 }}>
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
          )}
        </FormField>
      </div>

      <div className="step-section">
        <FormField label="忽略短微博" hint="推荐默认值 20">
          <MinLengthInput value={minTextLength} onChange={onMinTextLengthChange} />
        </FormField>
      </div>
    </div>
  );
}
