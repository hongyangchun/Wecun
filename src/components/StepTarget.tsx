import { useEffect, useState } from "react";

import { listDownloadHistory, type HistoryEntry } from "../lib/tauri-bridge";
import { isValidProfileUrl } from "../lib/validation";

interface StepTargetProps {
  profileUrl: string;
  onProfileUrlChange: (url: string) => void;
  onNext: () => void;
  sourceType: "profile" | "favorites";
  onSourceTypeChange: (type: "profile" | "favorites") => void;
}

export default function StepTarget({ profileUrl, onProfileUrlChange, onNext, sourceType, onSourceTypeChange }: StepTargetProps) {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const valid = profileUrl.length > 0 && isValidProfileUrl(profileUrl);

  useEffect(() => {
    listDownloadHistory().then(setHistory).catch(() => {});
  }, []);

  const handleKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && valid) onNext();
  };

  const handleHistoryClick = (entry: HistoryEntry) => {
    if (entry.uid === "favorites") {
      onSourceTypeChange("favorites");
    } else {
      onSourceTypeChange("profile");
      onProfileUrlChange(`https://weibo.com/u/${entry.uid}`);
    }
  };

  const inputState = profileUrl.length === 0 ? "" : valid ? "input--valid" : "input--error";

  return (
    <div className="step-enter">
      <h2 className="step-heading">
        {sourceType === "favorites" ? "下载来源" : "博主主页地址"}
      </h2>

      <div style={{ marginBottom: 20 }}>
        <p className="form-hint" style={{ marginBottom: 12 }}>
          下载来源：
        </p>
        <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", userSelect: "none" }}>
            <input
              type="radio"
              name="sourceType"
              value="profile"
              checked={sourceType === "profile"}
              onChange={() => onSourceTypeChange("profile")}
              style={{ cursor: "pointer" }}
            />
            <span style={{ color: "var(--color-text)", fontSize: 14 }}>博主主页</span>
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", userSelect: "none" }}>
            <input
              type="radio"
              name="sourceType"
              value="favorites"
              checked={sourceType === "favorites"}
              onChange={() => onSourceTypeChange("favorites")}
              style={{ cursor: "pointer" }}
            />
            <span style={{ color: "var(--color-text)", fontSize: 14 }}>我的收藏</span>
          </label>
        </div>
      </div>

      {sourceType === "favorites" ? (
        <div style={{ padding: 16, borderRadius: 8, background: "var(--color-bg-inset)", border: "1px solid var(--color-border)" }}>
          <p style={{ margin: 0, color: "var(--color-text-secondary)", fontSize: 14 }}>
            将下载当前登录账号的收藏微博
          </p>
        </div>
      ) : (
        <>
          <p className="form-hint" style={{ marginBottom: 20 }}>
            输入要备份的微博博主主页链接，或直接输入 UID
          </p>

          <input
            className={`input ${inputState}`}
            type="text"
            placeholder="https://www.weibo.com/u/2166767661"
            value={profileUrl}
            onChange={(e) => onProfileUrlChange(e.target.value.trim())}
            onKeyDown={handleKey}
            autoFocus
          />

          {profileUrl.length > 0 && !valid && (
            <p style={{ fontSize: 12, color: "var(--color-danger)", marginTop: 8 }}>
              请输入包含 UID 的微博主页地址，或直接输入 UID
            </p>
          )}

          <p className="form-hint" style={{ marginTop: 12 }}>
            示例：https://www.weibo.com/u/2166767661 或直接输入 UID 数字
          </p>

          {history.length > 0 ? (
            <div style={{ marginTop: 12 }}>
              <p className="step-hint step-hint--inline">最近下载</p>
              <div className="history-list">
                {history.map((entry) => (
                  <button
                    key={entry.uid}
                    className="history-item"
                    onClick={() => handleHistoryClick(entry)}
                    type="button"
                  >
                    <span className="history-name">{entry.screen_name}</span>
                    <span className="history-meta">{entry.post_count} 条 · {entry.last_download.split("T")[0]}</span>
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <div style={{ marginTop: 12 }}>
              <p className="step-hint step-hint--inline">最近下载</p>
              <div style={{ padding: "12px 16px", borderRadius: "var(--radius-md)", background: "var(--color-bg-inset)", border: "1px dashed var(--color-border-subtle)" }}>
                <p className="form-hint" style={{ margin: 0, color: "var(--color-text-tertiary)", textAlign: "center" }}>
                  首次使用，暂无下载历史
                </p>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
