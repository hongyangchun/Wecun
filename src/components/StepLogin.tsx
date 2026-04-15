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

export default function StepLogin({ isLoggedIn, isLoggingIn, username, usernameFetchFailed, restoreError, dispatch }: StepLoginProps) {
  const [isOpening, setIsOpening] = useState(false);

  const handleLogin = async () => {
    setIsOpening(true);
    dispatch({ type: "LOGIN_START" });
    try {
      await openLoginWindow();
    } catch {
      void 0;
    } finally {
      setIsOpening(false);
    }
  };

  if (isLoggedIn) {
    return (
      <div className="step-enter center-illustration">
        <div className="center-illustration-icon" style={{ background: "var(--color-success-subtle)" }}>
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M4 8.5L7 11.5L12 4.5" stroke="var(--color-success)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </div>
        <h2 className="processing-title">已登录</h2>
        <p className="form-hint" style={{ textAlign: "center" }}>
          {username ? `@${username}` : "微博账号已就绪"}，点击下方「下一步」继续
        </p>
        {usernameFetchFailed && (
          <p style={{ marginTop: 12, fontSize: 12, color: "var(--color-warning)", textAlign: "center", display: "flex", alignItems: "center", justifyContent: "center", gap: 6 }}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path d="M8 1L15 14H1L8 1Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"/>
              <path d="M8 6V9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
              <circle cx="8" cy="11.5" r="0.75" fill="currentColor"/>
            </svg>
            无法获取账号信息，登录状态可能尚未完全生效。建议稍等片刻后再进行下载操作。
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="step-enter center-illustration">
        <div className="center-illustration-icon" style={{ background: "var(--color-accent-subtle)" }}>
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <circle cx="8" cy="6" r="3" stroke="var(--color-accent)" strokeWidth="1.5" />
          <path d="M3 14C3 11.24 5.24 9 8 9C10.76 9 13 11.24 13 14" stroke="var(--color-accent)" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      </div>
      <h2 className="processing-title">登录微博账号</h2>
      <p className="form-hint" style={{ textAlign: "center", maxWidth: 260 }}>
        登录后即可下载目标博主的全部微博
      </p>

      <button
        className="btn btn-primary"
        style={{ marginTop: 32, minWidth: 160 }}
        onClick={handleLogin}
        disabled={isOpening || isLoggingIn}
        type="button"
      >
        {isLoggingIn ? "登录中，请在窗口中扫码..." : isOpening ? "正在打开..." : "打开登录页面"}
      </button>

      <div style={{ marginTop: 24, padding: "12px", background: "var(--color-bg-inset)", borderRadius: "var(--radius-md)", border: "1px solid var(--color-border-subtle)", textAlign: "left" }}>
        <p className="form-hint" style={{ margin: 0, color: "var(--color-text-secondary)", fontSize: 12, lineHeight: 1.5, display: "flex", gap: 6 }}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style={{ flexShrink: 0, marginTop: 2 }} aria-hidden="true">
            <path d="M8 1L15 14H1L8 1Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"/>
            <path d="M8 6V9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
            <circle cx="8" cy="11.5" r="0.75" fill="currentColor"/>
          </svg>
          <span><b>免责声明：</b><br/>
          本工具仅供<b>备份个人数字记录</b>使用。为保护账号安全，建议控制单次下载量，或使用辅助账号进行操作。</span>
        </p>
      </div>

      {restoreError && (
        <p style={{ marginTop: 16, fontSize: 12, color: "var(--color-danger)" }}>{restoreError}</p>
      )}
    </div>
  );
}
