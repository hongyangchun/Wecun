import { useState, useCallback, memo } from "react";
import { LogPanel, ProgressRing } from "./ui";
import { DONATION_CONFIG } from "../lib/app-config";
import type { ProcessStatus } from "../state/wizard-reducer";
import type { ExportFormat } from "../types/contracts";

// 复用样式常量 - 避免每次渲染创建新对象
const riskWarningStyle: React.CSSProperties = {
  marginTop: 16,
  padding: "10px 12px",
  borderRadius: 8,
  background: "rgba(255, 69, 58, 0.1)",
  border: "1px solid rgba(255, 69, 58, 0.3)",
};

const statBoxStyle: React.CSSProperties = {
  padding: "12px 16px",
  borderRadius: "var(--radius-md)",
  background: "var(--color-bg-inset)",
  marginBottom: 16,
  border: "1px solid var(--color-border-subtle)",
};

const exportButtonStyle: React.CSSProperties = {
  width: "100%",
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  textAlign: "center",
  padding: "12px 8px",
  height: "auto",
  gap: 4,
};

interface StepProcessingProps {
  processStatus: Exclude<ProcessStatus, "idle">;
  phase: string;
  progress: number;
  current: number;
  total: number;
  logs: string[];
  onStop: () => void;
  onReset: () => void;
  onGoToLogin: () => void;
  onExport: (format: ExportFormat) => void;
  onContinueExport: () => void;
  onOpenOutputDir: () => void;
  onAnalyze: () => void;
  sourceType: "profile" | "favorites";
}



const StepProcessingComponent = ({
  processStatus,
  phase,
  progress,
  current,
  total,
  logs,
  onStop,
  onReset,
  onGoToLogin,
  onExport,
  onContinueExport,
  onOpenOutputDir,
  onAnalyze,
  sourceType,
}: StepProcessingProps) => {
  const [showDonation, setShowDonation] = useState(false);

  const handleClose = useCallback(() => {
    setShowDonation(true);
  }, []);

  const handleDonationClose = useCallback(() => {
    setShowDonation(false);
    onReset();
  }, [onReset]);

  const exportActions: Array<{ format: ExportFormat; label: string; desc: string }> = [
    { format: "html", label: "HTML", desc: "浏览器查看" },
    { format: "md-single", label: "Markdown", desc: "单文件" },
    { format: "md-obsidian", label: "Markdown", desc: "分文件+Obsidian" },
  ];

  // 统计最近 10 条日志中疑似风控关键词的出现次数
  const riskKeywordCounts = logs.slice(-10).reduce(
    (acc, log) => {
      if (log.includes("频繁")) acc.frequent++;
      if (log.includes("拦截")) acc.blocked++;
      if (log.includes("403") || log.includes("Forbidden")) acc.forbidden++;
      return acc;
    },
    { frequent: 0, blocked: 0, forbidden: 0 }
  );

  // 只有明确检测到微博风控特征时才显示预警：频繁/拦截关键词出现 2 次以上，或检测到 403
  const isRiskWarning =
    riskKeywordCounts.frequent >= 2 ||
    riskKeywordCounts.blocked >= 2 ||
    riskKeywordCounts.forbidden >= 1;

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
              <div style={riskWarningStyle}>
                <p className="form-hint" style={{ margin: 0, color: "var(--color-text)", display: "flex", gap: 6 }}>
                  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style={{ flexShrink: 0, marginTop: 1 }} aria-hidden="true">
                    <path d="M8 1L15 14H1L8 1Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"/>
                    <path d="M8 6V9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                    <circle cx="8" cy="11.5" r="0.75" fill="currentColor"/>
                  </svg>
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
              <div style={statBoxStyle}>
                <div style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text)" }}>
                  共处理 {total} 条微博
                </div>
              </div>
            )}

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
              {exportActions.map((action) => (
                <button
                  key={action.format}
                  className="btn btn-secondary"
                  type="button"
                  onClick={() => onExport(action.format)}
                  style={exportButtonStyle}
                >
                  <span style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text)" }}>{action.label}</span>
                  <span style={{ fontSize: 11, color: "var(--color-text-tertiary)", lineHeight: 1.4 }}>{action.desc}</span>
                </button>
              ))}
              <button
                key="analyze"
                className="btn btn-outline-accent"
                type="button"
                onClick={() => onAnalyze()}
                disabled={sourceType === "favorites"}
                style={{
                  width: "100%",
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    textAlign: "center",
                    padding: "12px 8px",
                    height: "auto",
                    gap: 4,
                    opacity: sourceType === "favorites" ? 0.5 : 1,
                    cursor: sourceType === "favorites" ? "not-allowed" : "pointer",
                  }}
                >
                  <span style={{ fontSize: 13, fontWeight: 600, color: "var(--color-accent)", display: "flex", alignItems: "center", gap: 4 }}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                      <circle cx="11" cy="11" r="8" stroke="currentColor" strokeWidth="2"/>
                      <path d="M21 21l-4.35-4.35" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
                    </svg>
                    画像分析
                  </span>
                  <span style={{ fontSize: 11, color: "var(--color-text-tertiary)", lineHeight: 1.4 }}>
                    {sourceType === "favorites" ? "仅博主模式" : "查看数据报告"}
                  </span>
                </button>
            </div>

            {renderLog()}

            <div style={{ marginTop: 8 }}>
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
              <div style={statBoxStyle}>
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
              <button className="btn btn-secondary" style={{ flex: 1 }} onClick={handleClose} type="button">
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
              登录已过期，请重新登录
            </div>

            {renderLog()}

            <div style={{ marginTop: 16 }}>
              <button className="btn btn-primary" style={{ width: "100%" }} onClick={onGoToLogin} type="button">
                重新登录
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

        {showDonation && (
          <div className="donation-overlay" role="dialog" aria-modal="true" aria-labelledby="donation-title">
            <div className="donation-card">
              <button className="donation-close" type="button" onClick={handleDonationClose} aria-label="关闭">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                  <path d="M4 4L12 12M12 4L4 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
                </svg>
              </button>

              <span className="donation-badge">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" style={{ verticalAlign: "middle", marginRight: 4 }} aria-hidden="true">
                  <path d="M17 8h1a4 4 0 1 1 0 8h-1M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V8zM6 1v3M10 1v3M14 1v3" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                </svg>
                请我喝杯咖啡
              </span>

              <h3 id="donation-title" style={{ fontSize: 15, fontWeight: 700, color: "var(--color-text)", marginBottom: 6 }}>
                {DONATION_CONFIG.title}
              </h3>
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 16 }}>
                {DONATION_CONFIG.subtitle}
              </p>

              {DONATION_CONFIG.qrImages && DONATION_CONFIG.qrImages.length > 0 && (
                <div style={{ display: "flex", justifyContent: "center", gap: 12, marginBottom: 12 }}>
                  {DONATION_CONFIG.qrImages.map((img) => (
                    <div key={img.path} style={{ textAlign: "center" }}>
                      <img src={img.path} alt={img.name} className="donation-qr" />
                      <div style={{ fontSize: 10, color: "var(--color-text-tertiary)", marginTop: 4 }}>{img.name}</div>
                    </div>
                  ))}
                </div>
              )}

              {DONATION_CONFIG.wechatId && (
                <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 4 }}>
                  个人微信：{DONATION_CONFIG.wechatId}
                </p>
              )}

              <div style={{ marginTop: 12 }}>
                <button
                  className="btn btn-secondary"
                  style={{ width: "100%" }}
                  type="button"
                  onClick={handleDonationClose}
                >
                  关闭
                </button>
              </div>
            </div>
          </div>
        )}

      </div>
    </div>
  );
};

// 使用 React.memo 避免不必要的重渲染
export default memo(StepProcessingComponent);
