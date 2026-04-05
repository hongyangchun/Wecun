import { useEffect, useState } from "react";

import { listDownloadHistory, type HistoryEntry } from "../lib/tauri-bridge";
import { isValidProfileUrl } from "../lib/validation";

interface StepTargetProps {
  profileUrl: string;
  onProfileUrlChange: (url: string) => void;
  onNext: () => void;
}

export default function StepTarget({ profileUrl, onProfileUrlChange, onNext }: StepTargetProps) {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const valid = profileUrl.length > 0 && isValidProfileUrl(profileUrl);

  useEffect(() => {
    listDownloadHistory().then(setHistory).catch(() => {});
  }, []);

  const handleKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && valid) onNext();
  };

  const inputState = profileUrl.length === 0 ? "" : valid ? "input--valid" : "input--error";

  return (
    <div className="step-enter">
      <h2 className="step-heading">
        博主主页地址
      </h2>
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

      {history.length > 0 && (
        <div style={{ marginTop: 12 }}>
          <p className="step-hint step-hint--inline">最近下载</p>
          <div className="history-list">
            {history.map((entry) => (
              <button
                key={entry.uid}
                className="history-item"
                onClick={() => onProfileUrlChange(`https://weibo.com/u/${entry.uid}`)}
                type="button"
              >
                <span className="history-name">{entry.screen_name}</span>
                <span className="history-meta">{entry.post_count} 条 · {entry.last_download.split("T")[0]}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
