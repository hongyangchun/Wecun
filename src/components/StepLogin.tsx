import { useState } from "react";
import { openLoginWindow } from "../lib/tauri-bridge";

interface StepLoginProps {
  isLoggedIn: boolean;
  onNext: () => void;
  onLogout: () => void;
  restoreError?: string;
}

export default function StepLogin({ isLoggedIn, onNext, onLogout, restoreError }: StepLoginProps) {
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
      <div className="step-content step-login">
        <div className="step-icon">
          <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
            <circle className="icon-success-soft" cx="24" cy="24" r="22" />
            <path className="icon-success" d="M15 24L21 30L33 18" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </div>
        <h2 className="step-title">已登录</h2>
        <p className="step-desc">微博账号已就绪，可以继续操作</p>
        <div className="wizard-actions">
          <button className="btn-wizard btn-wizard-primary" onClick={onNext} type="button">
            下一步
          </button>
          <button className="btn-wizard btn-wizard-outline" onClick={onLogout} type="button">
            退出登录
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="step-content step-login">
      <div className="step-icon">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <circle className="icon-soft" cx="24" cy="24" r="22" />
          <path className="icon-accent" d="M16 24C16 19.58 19.58 16 24 16C28.42 16 32 19.58 32 24" strokeWidth="2.5" strokeLinecap="round"/>
          <circle className="icon-accent" cx="24" cy="28" r="4" />
          <path className="icon-accent" d="M18 36C18 32.69 20.69 30 24 30C27.31 30 30 32.69 30 36" strokeWidth="2.5" strokeLinecap="round"/>
        </svg>
      </div>
      <h2 className="step-title">登录微博账号</h2>
      <p className="step-desc">登录后即可下载目标博主的全部微博</p>
      <button
        className="btn-wizard btn-wizard-primary"
        onClick={handleLogin}
        disabled={isOpening}
        type="button"
      >
        {isOpening ? "正在打开..." : "打开登录页面"}
      </button>
      {restoreError && <p className="input-error step-login-error">{restoreError}</p>}
      <p className="step-hint">登录成功后将自动进入下一步</p>
    </div>
  );
}
