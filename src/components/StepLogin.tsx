import { useState } from "react";
import { openLoginWindow } from "../lib/tauri-bridge";
import type { Dispatch } from "react";
import type { WizardAction } from "../state/wizard-reducer";

interface StepLoginProps {
  isLoggedIn: boolean;
  isLoggingIn: boolean;
  username: string;
  usernameFetchFailed: boolean;
  restoreError?: string;
  dispatch: Dispatch<WizardAction>;
}

export default function StepLogin({ isLoggingIn, restoreError, dispatch }: StepLoginProps) {
  const [isOpening, setIsOpening] = useState(false);

  const handleLogin = async () => {
    setIsOpening(true);
    try {
      await openLoginWindow();
      dispatch({ type: "LOGIN_START" });
    } catch {
      dispatch({ type: "LOGIN_CANCEL" });
    } finally {
      setIsOpening(false);
    }
  };

  const handleCancel = () => {
    dispatch({ type: "LOGIN_CANCEL" });
  };

  return (
    <div className="step-enter center-illustration">
      <div className="center-illustration-icon" style={{ background: "var(--color-accent-subtle)" }}>
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <rect x="3" y="3" width="7" height="7" rx="1.5" stroke="var(--color-accent)" strokeWidth="1.8" />
          <rect x="14" y="3" width="7" height="7" rx="1.5" stroke="var(--color-accent)" strokeWidth="1.8" />
          <rect x="3" y="14" width="7" height="7" rx="1.5" stroke="var(--color-accent)" strokeWidth="1.8" />
          <path d="M14 14h3v3h-3zM18 18h3v3h-3zM14 19h2v2h-2zM19 14h2v2h-2z" fill="var(--color-accent)" />
        </svg>
      </div>
      <h2 className="processing-title">微博扫码登录</h2>
      <p className="form-hint" style={{ textAlign: "center", maxWidth: 300, marginTop: 0 }}>
        打开微博官方版 App 扫一扫即可安全登录，本地运行不上传任何账号信息
      </p>

      <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 12 }}>
        <button
          className="btn btn-primary"
          style={{ minWidth: 168 }}
          onClick={handleLogin}
          disabled={isOpening}
          type="button"
        >
          {isOpening
            ? "正在打开..."
            : isLoggingIn
              ? "重新打开扫码窗口"
              : "打开微博扫码登录"}
        </button>
        {isLoggingIn && (
          <button
            className="btn btn-secondary"
            onClick={handleCancel}
            type="button"
          >
            取消
          </button>
        )}
      </div>

      {isLoggingIn && (
        <p className="form-hint" style={{ textAlign: "center", marginTop: 0, color: "var(--color-accent)" }}>
          请在弹出的扫码窗口中使用微博官方版扫码确认，关闭窗口将自动返回
        </p>
      )}

      <div className="login-notice-box">
        <p className="form-hint" style={{ margin: 0, color: "var(--color-text-secondary)", fontSize: 12, lineHeight: 1.6 }}>
          <strong style={{ color: "var(--color-text)" }}>扫码提示：</strong>请使用<b>微博官方版 App</b> 扫码（暂不支持微博轻享版）。<br />
          <strong style={{ color: "var(--color-text)" }}>免责声明：</strong>本工具仅供备份个人数字记录使用，建议控制单次下载量或使用辅助账号操作。
        </p>
      </div>

      {restoreError && (
        <p style={{ marginTop: 4, fontSize: 12, color: "var(--color-danger)" }}>{restoreError}</p>
      )}
    </div>
  );
}
