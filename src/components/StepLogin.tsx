import { useState } from "react";
import { openLoginWindow } from "../lib/tauri-bridge";

interface StepLoginProps {
  isLoggedIn: boolean;
  restoreError?: string;
}

export default function StepLogin({ isLoggedIn, restoreError }: StepLoginProps) {
  const [isOpening, setIsOpening] = useState(false);

  const handleLogin = async () => {
    setIsOpening(true);
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
          <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
            <path d="M4 8.5L7 11.5L12 4.5" stroke="var(--color-success)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </div>
        <h2 className="processing-title">已登录</h2>
        <p className="form-hint" style={{ textAlign: "center" }}>
          微博账号已就绪，点击下方「下一步」继续
        </p>
      </div>
    );
  }

  return (
    <div className="step-enter center-illustration">
      <div className="center-illustration-icon" style={{ background: "var(--color-accent-subtle)" }}>
        <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
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
        disabled={isOpening}
        type="button"
      >
        {isOpening ? "正在打开..." : "打开登录页面"}
      </button>

      {restoreError && (
        <p style={{ marginTop: 16, fontSize: 12, color: "var(--color-danger)" }}>{restoreError}</p>
      )}

      <p className="form-hint" style={{ marginTop: 16 }}>登录成功后将自动进入下一步</p>
    </div>
  );
}
