import { useEffect, useMemo, useState } from "react";
import { SegmentedControl } from "./ui";
import { pickDirectory } from "../lib/dialog";
import { lookupUserInfo } from "../lib/tauri-bridge";
import { canUpdateBackup, extractUidFromUrl, isValidProfileUrl } from "../lib/validation";
import type { BackupMeta, DownloadRange, PostFilter, RecentTarget } from "../types/contracts";

interface WorkspaceFormProps {
  sourceType: "profile" | "favorites";
  onSourceTypeChange: (type: "profile" | "favorites") => void;
  profileUrl: string;
  onProfileUrlChange: (url: string) => void;
  currentUsername: string;
  currentUid: string;
  recentTargets: RecentTarget[];
  onRemoveRecentTarget: (uid: string) => void;
  onResolvedAuthor: (uid: string, screenName: string) => void;
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
  outputDir: string;
  onDirChange: (dir: string) => void;
  onOpenOutputDir: () => void;
  onStartDownload: () => void;
  canStart: boolean;
  /** The finished backup in the chosen directory, when it is the current target's. */
  existingBackup: BackupMeta | null;
  freshDownload: boolean;
  onFreshDownloadChange: (value: boolean) => void;
  onExportExisting: () => void;
}

function formatDate(d: Date): string {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function getDatePresets(): Array<{ id: string; label: string; start: string; end: string }> {
  const now = new Date();
  const today = formatDate(now);

  const monthAgo = new Date(now);
  monthAgo.setMonth(monthAgo.getMonth() - 1);

  const threeMonthsAgo = new Date(now);
  threeMonthsAgo.setMonth(threeMonthsAgo.getMonth() - 3);

  const yearAgo = new Date(now);
  yearAgo.setFullYear(yearAgo.getFullYear() - 1);

  const startOfYear = `${now.getFullYear()}-01-01`;

  return [
    { id: "1m", label: "近 1 个月", start: formatDate(monthAgo), end: today },
    { id: "3m", label: "近 3 个月", start: formatDate(threeMonthsAgo), end: today },
    { id: "1y", label: "近 1 年", start: formatDate(yearAgo), end: today },
    { id: "ytd", label: "今年", start: startOfYear, end: today },
  ];
}

function MinLengthInput({ value, onChange }: { value: number; onChange: (n: number) => void }) {
  const [str, setStr] = useState(String(value));

  useEffect(() => {
    setStr(String(value));
  }, [value]);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 6 }}>
      <input
        type="number"
        className="input"
        style={{ width: 68, height: 30, padding: "0 10px" }}
        value={str}
        onChange={(e) => setStr(e.target.value)}
        onBlur={() => onChange(Math.max(0, Math.min(500, parseInt(str, 10) || 0)))}
        min={0}
        max={500}
      />
      <span className="form-hint" style={{ margin: 0 }}>字（0 = 不过滤）</span>
    </div>
  );
}

type PreviewState =
  | { status: "idle" }
  | { status: "loading"; uid: string }
  | { status: "found"; uid: string; screenName: string }
  | { status: "error"; uid: string };

export default function WorkspaceForm({
  sourceType,
  onSourceTypeChange,
  profileUrl,
  onProfileUrlChange,
  currentUsername,
  currentUid,
  recentTargets,
  onRemoveRecentTarget,
  onResolvedAuthor,
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
  outputDir,
  onDirChange,
  onOpenOutputDir,
  onStartDownload,
  canStart,
  existingBackup,
  freshDownload,
  onFreshDownloadChange,
  onExportExisting,
}: WorkspaceFormProps) {
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [preview, setPreview] = useState<PreviewState>({ status: "idle" });

  const extractedUid = useMemo(
    () => (sourceType === "profile" ? extractUidFromUrl(profileUrl) : null),
    [sourceType, profileUrl],
  );
  const urlValid = profileUrl.length > 0 && isValidProfileUrl(profileUrl);
  const inputState = profileUrl.length === 0 ? "" : urlValid ? "input--valid" : "input--error";
  const presets = useMemo(() => getDatePresets(), []);

  // Debounced lookup of target blogger nickname
  useEffect(() => {
    if (sourceType !== "profile" || !extractedUid) {
      setPreview({ status: "idle" });
      return;
    }

    // Immediate match if it's the current logged-in user
    if (currentUid && extractedUid === currentUid && currentUsername) {
      setPreview({ status: "found", uid: extractedUid, screenName: currentUsername });
      onResolvedAuthor(extractedUid, currentUsername);
      return;
    }

    let cancelled = false;
    setPreview({ status: "loading", uid: extractedUid });

    const timer = setTimeout(() => {
      void lookupUserInfo(extractedUid)
        .then((user) => {
          if (cancelled) return;
          if (user && user.screen_name) {
            setPreview({ status: "found", uid: user.uid, screenName: user.screen_name });
            onResolvedAuthor(user.uid, user.screen_name);
          } else {
            setPreview({ status: "error", uid: extractedUid });
          }
        })
        .catch(() => {
          if (cancelled) return;
          setPreview({ status: "error", uid: extractedUid });
        });
    }, 400);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [sourceType, extractedUid, currentUid, currentUsername, onResolvedAuthor]);

  const handlePickDir = async () => {
    const dir = await pickDirectory();
    if (dir) onDirChange(dir);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && canStart) {
      onStartDownload();
    }
  };

  const activeFilterSummary: string[] = [];
  if (sourceType === "profile") {
    if (postFilter === "original") activeFilterSummary.push("仅原创");
    if (minTextLength > 0) activeFilterSummary.push(`≥${minTextLength}字`);
  } else if (ignoreDeleted) {
    activeFilterSummary.push("忽略已删除");
  }

  const dateInvalid =
    sourceType === "profile" &&
    downloadRange === "range" &&
    Boolean(dateStart && dateEnd && dateStart > dateEnd);

  // Only promise an update when the download will really be one. The backend decides by the same
  // rules, and a run that starts over replaces the backup that is in the directory.
  const whatStartWillDo = !existingBackup
    ? ""
    : freshDownload
      ? "已勾选「重新完整下载」：会从第一页重新抓取，并替换这份备份。"
      : canUpdateBackup(existingBackup, {
            sourceType,
            filter: postFilter,
            minTextLength,
            ignoreDeleted,
            downloadRange,
            dateStart,
            dateEnd,
          })
        ? "点「开始下载」只会补上新微博。"
        : "当前的筛选条件或时间范围和这份备份不同，点「开始下载」会重新完整下载并替换它。";

  return (
    <div className="step-enter workspace-sections">
      {/* 1. 备份来源 & 目标 */}
      <section className="workspace-card">
        <div className="workspace-card-header">
          <span className="workspace-card-title">备份来源</span>
          {sourceType === "profile" && currentUid && extractedUid !== currentUid && (
            <button
              type="button"
              className="inline-pill-btn"
              onClick={() => onProfileUrlChange(`https://weibo.com/u/${currentUid}`)}
            >
              备份我自己
            </button>
          )}
        </div>

        <SegmentedControl
          options={[
            { value: "profile", label: "博主主页" },
            { value: "favorites", label: "我的收藏" },
          ]}
          value={sourceType}
          onChange={onSourceTypeChange}
          ariaLabel="备份来源"
        />

        {sourceType === "favorites" ? (
          <div className="workspace-info-banner">
            将备份当前账号{currentUsername ? `（@${currentUsername}）` : ""}的全部收藏微博
          </div>
        ) : (
          <div style={{ marginTop: 12 }}>
            <input
              className={`input ${inputState}`}
              type="text"
              placeholder="粘贴微博主页链接（如 weibo.com/u/2166767661）或输入 UID"
              value={profileUrl}
              onChange={(e) => onProfileUrlChange(e.target.value.trim())}
              onKeyDown={handleKeyDown}
            />

            {profileUrl.length > 0 && !urlValid && (
              <p className="field-error-text">请输入有效的微博主页链接或 5~20 位数字 UID</p>
            )}

            {urlValid && preview.status === "loading" && (
              <div className="preview-badge preview-badge--muted">正在验证博主信息...</div>
            )}
            {urlValid && preview.status === "found" && (
              <div className="preview-badge preview-badge--success">
                <span>✓ 将备份：<strong>@{preview.screenName}</strong></span>
                <span className="preview-badge-uid">UID {preview.uid}</span>
              </div>
            )}
            {urlValid && preview.status === "error" && (
              <div className="preview-badge preview-badge--muted">
                暂未能预览博主昵称（UID {preview.uid}），仍可直接尝试下载
              </div>
            )}

            {recentTargets.length > 0 && (
              <div className="recent-targets">
                <span className="recent-targets-label">最近：</span>
                <div className="recent-chips">
                  {recentTargets.map((item) => {
                    const isSelected = extractedUid === item.uid;
                    return (
                      <span
                        key={item.uid}
                        className={`recent-chip ${isSelected ? "recent-chip--active" : ""}`}
                      >
                        <button
                          type="button"
                          className="recent-chip-btn"
                          onClick={() => onProfileUrlChange(`https://weibo.com/u/${item.uid}`)}
                          title={`UID: ${item.uid}`}
                        >
                          @{item.screenName}
                        </button>
                        <button
                          type="button"
                          className="recent-chip-remove"
                          onClick={() => onRemoveRecentTarget(item.uid)}
                          aria-label={`移除 ${item.screenName}`}
                        >
                          ×
                        </button>
                      </span>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        )}
      </section>

      {/* 2. 备份选项 */}
      <section className="workspace-card">
        <div className="workspace-card-header">
          <span className="workspace-card-title">备份选项</span>
        </div>

        {sourceType === "profile" && (
          <div style={{ marginBottom: 12 }}>
            <SegmentedControl
              options={[
                { value: "all", label: "全部时间" },
                { value: "range", label: "指定时间段" },
              ]}
              value={downloadRange}
              onChange={onDownloadRangeChange}
              ariaLabel="时间范围"
            />

            {downloadRange === "range" && (
              <div style={{ marginTop: 10 }}>
                <div className="preset-chips">
                  {presets.map((preset) => {
                    const isActive = dateStart === preset.start && dateEnd === preset.end;
                    return (
                      <button
                        key={preset.id}
                        type="button"
                        className={`preset-chip ${isActive ? "preset-chip--active" : ""}`}
                        onClick={() => {
                          onDateStartChange(preset.start);
                          onDateEndChange(preset.end);
                        }}
                      >
                        {preset.label}
                      </button>
                    );
                  })}
                </div>
                <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8 }}>
                  <input
                    type="date"
                    className="input"
                    style={{ height: 34, flex: 1 }}
                    value={dateStart}
                    onChange={(e) => onDateStartChange(e.target.value)}
                    aria-label="开始日期"
                  />
                  <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>至</span>
                  <input
                    type="date"
                    className="input"
                    style={{ height: 34, flex: 1 }}
                    value={dateEnd}
                    onChange={(e) => onDateEndChange(e.target.value)}
                    aria-label="结束日期"
                  />
                </div>
                {dateInvalid && (
                  <p className="field-error-text">开始日期不能晚于结束日期</p>
                )}
              </div>
            )}
          </div>
        )}

        <label className="check-row" style={{ padding: "4px 0" }}>
          <input
            type="checkbox"
            checked={includeImages}
            onChange={(e) => onIncludeImagesChange(e.target.checked)}
          />
          <div>
            <span className="check-label">包含高清原图</span>
            <span className="check-hint" style={{ display: "block" }}>图片自动保存到 images/ 目录，支持断点跳过</span>
          </div>
        </label>

        <div style={{ marginTop: 10, paddingTop: 10, borderTop: "1px solid var(--color-border-subtle)" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
            <button
              type="button"
              className="disclosure-btn"
              onClick={() => setAdvancedOpen((prev) => !prev)}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 12 12"
                fill="none"
                style={{
                  transition: "transform 180ms cubic-bezier(0.16, 1, 0.3, 1)",
                  transform: advancedOpen ? "rotate(90deg)" : "rotate(0deg)",
                }}
              >
                <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              <span>高级筛选</span>
            </button>
            {!advancedOpen && activeFilterSummary.length > 0 && (
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                {activeFilterSummary.map((label) => (
                  <span key={label} className="filter-pill">{label}</span>
                ))}
              </div>
            )}
          </div>

          {advancedOpen && (
            <div className="advanced-box">
              {sourceType === "profile" ? (
                <>
                  <label className="check-row" style={{ padding: 0 }}>
                    <input
                      type="checkbox"
                      checked={postFilter === "original"}
                      onChange={(e) => onPostFilterChange(e.target.checked ? "original" : "all")}
                    />
                    <span className="check-label">仅原创（不包含转发微博）</span>
                  </label>

                  <label className="check-row" style={{ padding: 0 }}>
                    <input
                      type="checkbox"
                      checked={minTextLength > 0}
                      onChange={(e) => onMinTextLengthChange(e.target.checked ? 20 : 0)}
                    />
                    <div>
                      <span className="check-label">忽略短微博</span>
                      <MinLengthInput value={minTextLength} onChange={onMinTextLengthChange} />
                    </div>
                  </label>
                </>
              ) : (
                <label className="check-row" style={{ padding: 0 }}>
                  <input
                    type="checkbox"
                    checked={ignoreDeleted}
                    onChange={(e) => onIgnoreDeletedChange(e.target.checked)}
                  />
                  <div>
                    <span className="check-label">忽略已删除微博</span>
                    <span className="check-hint" style={{ display: "block" }}>跳过原博主已删除或不可见的内容</span>
                  </div>
                </label>
              )}

              {existingBackup && (
                <label className="check-row" style={{ padding: 0, marginTop: 8 }}>
                  <input
                    type="checkbox"
                    checked={freshDownload}
                    onChange={(e) => onFreshDownloadChange(e.target.checked)}
                  />
                  <div>
                    <span className="check-label">重新完整下载</span>
                    <span className="check-hint" style={{ display: "block" }}>
                      忽略此目录已有的备份，从第一页重新抓取
                    </span>
                  </div>
                </label>
              )}
            </div>
          )}
        </div>
      </section>

      {/* 3. 保存位置 */}
      <section className="workspace-card">
        <div className="workspace-card-header">
          <span className="workspace-card-title">保存目录</span>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            {outputDir && (
              <button
                type="button"
                className="inline-pill-btn"
                onClick={onOpenOutputDir}
                title="在访达 / 资源管理器中打开"
              >
                打开目录
              </button>
            )}
            <button
              type="button"
              className="inline-pill-btn inline-pill-btn--accent"
              onClick={handlePickDir}
            >
              {outputDir ? "更改目录" : "选择目录"}
            </button>
          </div>
        </div>

        <div
          className={`dir-row ${!outputDir ? "dir-row--empty" : ""}`}
          onClick={handlePickDir}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              void handlePickDir();
            }
          }}
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true" style={{ flexShrink: 0, marginTop: 2 }}>
            <path d="M2 4H5L6.5 6H12V11H2V4Z" stroke="var(--color-text-tertiary)" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          <span className="dir-display-path">
            {outputDir || "点击选择微博备份的保存文件夹（会自动记住上次位置）"}
          </span>
        </div>
      </section>

      {/* 4. What the directory already holds */}
      {existingBackup && (
        <section className="workspace-card">
          <div className="workspace-card-header">
            <span className="workspace-card-title">已有备份</span>
            <button type="button" className="inline-pill-btn" onClick={onExportExisting}>
              导出已有备份
            </button>
          </div>

          <p className="form-hint" style={{ margin: 0 }}>
            此目录已有 {existingBackup.source_type === "favorites" ? "当前账号的收藏" : `@${existingBackup.author}`} 的备份，共{" "}
            {existingBackup.post_count} 条。{whatStartWillDo}
          </p>
        </section>
      )}
    </div>
  );
}
