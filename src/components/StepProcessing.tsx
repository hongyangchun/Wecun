import { useState } from "react";
import { LogPanel, ProgressRing } from "./ui";
import { DONATION_CONFIG } from "../lib/app-config";
import type { ProcessStatus } from "../state/wizard-reducer";
import type { ExportFormat } from "../types/contracts";

interface StepProcessingProps {
  processStatus: Exclude<ProcessStatus, "idle">;
  phase: string;
  progress: number;
  current: number;
  total: number;
  errorMessage?: string;
  logs: string[];
  onStop: () => void;
  onReset: () => void;
  onExport: (format: ExportFormat) => void;
  onContinueExport: () => void;
  onOpenOutputDir: () => void;
}

function friendlyError(msg: string): string {
  if (msg.includes("403") || msg.includes("Forbidden")) return "登录已失效，请重新登录后再试";
  if (msg.includes("404") || msg.includes("not found")) return "未找到该用户，请检查链接是否正确";
  if (msg.includes("网络") || msg.includes("Network")) return "网络连接失败，请检查网络后重试";
  return msg;
}

export default function StepProcessing({
  processStatus,
  phase,
  progress,
  current,
  total,
  errorMessage,
  logs,
  onStop,
  onReset,
  onExport,
  onContinueExport,
  onOpenOutputDir,
}: StepProcessingProps) {
  const [showDonation, setShowDonation] = useState(false);

  const exportActions: Array<{ format: ExportFormat; label: string; desc: string }> = [
    { format: "html", label: "HTML", desc: "适合直接在浏览器中查看和分享" },
    { format: "md-single", label: "Markdown（单文件）", desc: "适合整理成一份完整备份" },
    { format: "md-multi", label: "Markdown（分文件）", desc: "每条微博一个独立文件，适合进一步整理" },
  ];

  const isRiskWarning = logs.slice(-10).some((log) =>
    log.includes("重试") || log.includes("网络异常") || log.includes("频繁") || log.includes("拦截") || log.includes("失败")
  );

  const renderLog = () =>
    logs.length > 0 ? <LogPanel logs={logs} /> : null;

  return (
    <div className="processing-overlay">
      <div className="processing-dialog">
        {processStatus === "downloading" && (
          <div className="step-enter">
            <div className="processing-header">
              <span className="processing-icon processing-icon--accent animate-spin-slow">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <circle cx="8" cy="8" r="6" stroke="var(--color-accent)" strokeWidth="1.5" strokeDasharray="4 3" fill="none" />
                </svg>
              </span>
              <h2 className="processing-title">{phase || "正在下载..."}</h2>
            </div>

            {isRiskWarning && (
              <div style={{ marginTop: 16, padding: "10px 12px", borderRadius: 8, background: "rgba(255, 69, 58, 0.1)", border: "1px solid rgba(255, 69, 58, 0.3)" }}>
                <p className="form-hint" style={{ margin: 0, color: "var(--color-text)", display: "flex", gap: 6 }}>
                  <span style={{ fontSize: 14 }}>⚠️</span>
                  <span>
                    <b>风控预警</b>：检测到接口请求失败或异常，程序正尝试重试。为保护账号安全，若持续报错建议<b>停止下载</b>，明天再试。
                  </span>
                </p>
              </div>
            )}

            {progress > 0 ? (
              <div className="center-illustration" style={{ padding: "20px 0" }}>
                <ProgressRing progress={progress} />
                {total > 0 && (
                  <p style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>已处理 {current} / {total}</p>
                )}
              </div>
            ) : (
              <div style={{ padding: "24px 0", display: "flex", flexDirection: "column", alignItems: "center", gap: 12 }}>
                <div style={{ width: "100%", height: 6, borderRadius: 3, background: "var(--color-border)", overflow: "hidden" }}>
                  <div className="progress-shimmer" style={{ height: "100%", width: "33%", borderRadius: 3 }} />
                </div>
                <p style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>{phase}</p>
              </div>
            )}

            {renderLog()}

            <div style={{ marginTop: 16 }}>
              <button className="btn btn-danger" style={{ width: "100%" }} onClick={onStop} type="button">
                停止下载
              </button>
            </div>
          </div>
        )}

        {processStatus === "exporting" && (
          <div className="step-enter">
            <div className="processing-header">
              <span className="processing-icon processing-icon--accent">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <rect x="2" y="2" width="12" height="12" rx="2" stroke="var(--color-accent)" strokeWidth="1.5" />
                  <path d="M5 6H11M5 8H11M5 10H9" stroke="var(--color-accent)" strokeWidth="1.2" strokeLinecap="round" />
                </svg>
              </span>
              <h2 className="processing-title">正在导出...</h2>
            </div>

            <div className="center-illustration" style={{ padding: "24px 0" }}>
              <svg width="72" height="72" viewBox="0 0 80 80" className="animate-spin-slow" style={{ animationDuration: "3s" }} aria-hidden="true">
                <circle cx="40" cy="40" r="32" fill="none" stroke="var(--color-border)" strokeWidth="5" />
                <circle cx="40" cy="40" r="32" fill="none" stroke="var(--color-accent)" strokeWidth="5" strokeLinecap="round" strokeDasharray="60 140" />
              </svg>
              <p style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>{phase || "正在导出..."}</p>
            </div>

            {renderLog()}
          </div>
        )}

        {processStatus === "downloaded" && (
          <div className="step-enter">
            <div className="processing-header">
              <span className="processing-icon processing-icon--success">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path className="animate-check-draw" d="M4 8.5L7 11.5L12 4.5" stroke="var(--color-success)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </span>
              <h2 className="processing-title">下载完成！</h2>
            </div>

            <p style={{ fontSize: 13, color: "var(--color-text-secondary)", marginBottom: 20 }}>
              微博已下载完成，可选择格式导出
            </p>

            {total > 0 && (
              <div style={{
                padding: "12px 16px",
                borderRadius: "var(--radius-md)",
                background: "var(--color-bg-inset)",
                marginBottom: 16,
                border: "1px solid var(--color-border-subtle)"
              }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text)" }}>
                  共处理 {total} 条微博
                </div>
              </div>
            )}

            <div style={{ display: "grid", gap: 10, marginBottom: 16 }}>
              {exportActions.map((action) => (
                <button
                  key={action.format}
                  className="btn btn-secondary"
                  type="button"
                  onClick={() => onExport(action.format)}
                  style={{
                    width: "100%",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                    textAlign: "left",
                    padding: "14px 16px",
                    height: "auto",
                    display: "flex",
                    gap: 16,
                  }}
                >
                  <span style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    <span style={{ fontSize: 14, fontWeight: 600, color: "var(--color-text)" }}>{action.label}</span>
                    <span style={{ fontSize: 12, color: "var(--color-text-secondary)", lineHeight: 1.5 }}>{action.desc}</span>
                  </span>
                  <span style={{ fontSize: 14, color: "var(--color-text-tertiary)", flexShrink: 0 }}>导出</span>
                </button>
              ))}
            </div>

            {renderLog()}

            <div style={{ marginTop: 16 }}>
              <button className="btn btn-secondary" style={{ width: "100%" }} onClick={onReset} type="button">
                返回
              </button>
            </div>
          </div>
        )}

        {processStatus === "done" && (
          <div className="step-enter">
            <div className="processing-header">
              <span className="processing-icon processing-icon--success">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path className="animate-check-draw" d="M4 8.5L7 11.5L12 4.5" stroke="var(--color-success)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </span>
              <h2 className="processing-title">导出完成！</h2>
            </div>

            <p style={{ fontSize: 13, color: "var(--color-text-secondary)", marginBottom: 20 }}>
              导出文件已生成，可继续导出其他格式
            </p>

            {total > 0 && (
              <div style={{
                padding: "12px 16px",
                borderRadius: "var(--radius-md)",
                background: "var(--color-bg-inset)",
                marginBottom: 16,
                border: "1px solid var(--color-border-subtle)"
              }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text)" }}>
                  共处理 {total} 条微博
                </div>
              </div>
            )}

            {renderLog()}

            <div style={{ display: "grid", gap: 8, marginTop: 16 }}>
              <button className="btn btn-secondary" style={{ width: "100%" }} onClick={onContinueExport} type="button">
                继续导出其他格式
              </button>
              <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-primary" style={{ flex: 1 }} onClick={onOpenOutputDir} type="button">
                打开目录
              </button>
              <button className="btn btn-secondary" style={{ flex: 1 }} onClick={() => setShowDonation(true)} type="button">
                关闭
              </button>
              </div>
            </div>
          </div>
        )}

        {processStatus === "error" && (
          <div className="step-enter">
            <div className="processing-header">
              <span className="processing-icon processing-icon--danger">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path d="M4 4L12 12M12 4L4 12" stroke="var(--color-danger)" strokeWidth="2" strokeLinecap="round" />
                </svg>
              </span>
              <h2 className="processing-title">下载出错</h2>
            </div>

            <div className="error-box" style={{ marginBottom: 20 }}>
              {friendlyError(errorMessage || "")}
            </div>

            {renderLog()}

            <div style={{ marginTop: 16 }}>
              <button className="btn btn-secondary" style={{ width: "100%" }} onClick={onReset} type="button">
                返回重试
              </button>
            </div>
          </div>
        )}

        {processStatus === "cancelled" && (
          <div className="step-enter">
            <div className="processing-header">
              <span className="processing-icon processing-icon--warning">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                  <rect x="3" y="3" width="8" height="8" rx="1.5" stroke="var(--color-warning)" strokeWidth="1.5" />
                </svg>
              </span>
              <h2 className="processing-title">已取消</h2>
            </div>

            {renderLog()}

            <div style={{ marginTop: 16 }}>
              <button className="btn btn-primary" style={{ width: "100%" }} onClick={onReset} type="button">
                返回
              </button>
            </div>
          </div>
        )}

        {processStatus === "done" && showDonation && (
          <div className="donation-overlay" role="dialog" aria-modal="true" aria-labelledby="donation-title">
            <div className="donation-card">
              <button className="donation-close" type="button" onClick={() => setShowDonation(false)} aria-label="关闭">
                ×
              </button>

              <span className="donation-badge">导出完成</span>

              <h3 id="donation-title" style={{ fontSize: 15, fontWeight: 700, color: "var(--color-text)", marginBottom: 6 }}>
                {DONATION_CONFIG.title}
              </h3>
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 16 }}>
                {DONATION_CONFIG.subtitle}
              </p>

              {DONATION_CONFIG.qrImagePath && (
                <div style={{ display: "flex", justifyContent: "center", marginBottom: 12 }}>
                  <img src={DONATION_CONFIG.qrImagePath} alt="打赏二维码" className="donation-qr" />
                </div>
              )}

              {DONATION_CONFIG.wechatId && (
                <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 4 }}>
                  个人微信：{DONATION_CONFIG.wechatId}
                </p>
              )}

              <button
                className="btn btn-primary"
                style={{ width: "100%", marginTop: 12 }}
                type="button"
                onClick={() => {
                  setShowDonation(false);
                  onReset();
                }}
              >
                我知道了
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
