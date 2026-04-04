import { useRef, useEffect } from "react";

interface StepDownloadProps {
  phase: string;
  progress: number;
  current: number;
  total: number;
  status: "downloading" | "done" | "cancelled" | "error";
  errorMessage?: string;
  logs: string[];
  onStop: () => void;
  onReset: () => void;
  onOpenOutputDir: () => void;
  outputDir?: string;
}

function friendlyError(msg: string): string {
  if (msg.includes("403") || msg.includes("Forbidden")) {
    return "登录已失效，请重新登录后再试";
  }
  if (msg.includes("404") || msg.includes("not found")) {
    return "未找到该用户，请检查链接是否正确";
  }
  if (msg.includes("网络") || msg.includes("Network")) {
    return "网络连接失败，请检查网络后重试";
  }
  return msg;
}

export default function StepDownload({
  phase, progress, current, total,
  status, errorMessage, logs,
  onStop, onReset, onOpenOutputDir,
}: StepDownloadProps) {
  const isRunning = status === "downloading";
  const isDone = status === "done";
  const isError = status === "error";
  const isCancelled = status === "cancelled";
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div className="step-content step-download">
      <div className="step-icon">
        {isRunning && (
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle className="icon-soft" cx="24" cy="24" r="22" />
            <path className="icon-accent" d="M24 14V26L32 30" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        )}
        {isDone && (
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle className="icon-success-soft" cx="24" cy="24" r="22" />
            <path className="icon-success" d="M15 24L21 30L33 18" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        )}
        {isError && (
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle className="icon-danger-soft" cx="24" cy="24" r="22" />
            <path className="icon-danger" d="M18 18L30 30M30 18L18 30" strokeWidth="2.5" strokeLinecap="round"/>
          </svg>
        )}
        {isCancelled && (
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle className="icon-warning-soft" cx="24" cy="24" r="22" />
            <rect className="icon-warning" x="18" y="18" width="12" height="12" rx="2" />
          </svg>
        )}
      </div>

      {isRunning && (
        <>
          <h2 className="step-title">{phase || "正在下载..."}</h2>
          {progress > 0 && (
            <div className="progress-ring-wrap">
              <svg width="120" height="120" viewBox="0 0 120 120">
                <circle cx="60" cy="60" r="52" fill="none" stroke="#E8EBF0" strokeWidth="8" />
                <circle
                  cx="60" cy="60" r="52"
                  fill="none"
                  stroke="url(#progressGradient)"
                  strokeWidth="8"
                  strokeLinecap="round"
                  strokeDasharray={`${2 * Math.PI * 52}`}
                  strokeDashoffset={`${2 * Math.PI * 52 * (1 - progress / 100)}`}
                  transform="rotate(-90 60 60)"
                  className="progress-ring"
                />
                <defs>
                  <linearGradient id="progressGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stopColor="#4F6EF7" />
                    <stop offset="100%" stopColor="#7C5CFC" />
                  </linearGradient>
                </defs>
              </svg>
              <span className="progress-pct">{progress}%</span>
            </div>
          )}
          {total > 0 && progress > 0 && (
            <p className="progress-detail">已处理 {current} / {total}</p>
          )}
          <button className="btn-wizard btn-wizard-danger" onClick={onStop} type="button">
            停止下载
          </button>
        </>
      )}

      {isDone && (
        <>
          <h2 className="step-title">下载完成</h2>
          <p className="step-desc">所有文件已保存至输出目录</p>
          <div className="wizard-actions">
            <button className="btn-wizard btn-wizard-secondary" onClick={onReset} type="button">
              重新下载
            </button>
            <button className="btn-wizard btn-wizard-primary" onClick={onOpenOutputDir} type="button">
              打开文件目录
            </button>
          </div>
        </>
      )}

      {isError && (
        <>
          <h2 className="step-title">下载出错</h2>
          <p className="error-text">{friendlyError(errorMessage || "")}</p>
          <div className="wizard-actions">
            <button className="btn-wizard btn-wizard-secondary" onClick={onReset} type="button">
              返回重试
            </button>
          </div>
        </>
      )}

      {isCancelled && (
        <>
          <h2 className="step-title">已取消</h2>
          <div className="wizard-actions">
            <button className="btn-wizard btn-wizard-primary" onClick={onReset} type="button">
              返回
            </button>
          </div>
        </>
      )}

      {logs.length > 0 && (
        <div className="log-area" ref={logRef}>
          {logs.slice(-8).map((log, i) => (
            <div key={i} className="log-line">{log}</div>
          ))}
        </div>
      )}
    </div>
  );
}
