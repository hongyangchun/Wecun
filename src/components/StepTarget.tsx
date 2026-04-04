import { isValidProfileUrl } from "../lib/validation";

interface StepTargetProps {
  profileUrl: string;
  onProfileUrlChange: (url: string) => void;
  onNext: () => void;
}

export default function StepTarget({ profileUrl, onProfileUrlChange, onNext }: StepTargetProps) {
  const valid = profileUrl.length > 0 && isValidProfileUrl(profileUrl);

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && valid) onNext();
  };

  return (
    <div className="step-content step-target">
      <div className="step-icon">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <circle cx="24" cy="24" r="22" fill="#E8F0FE" />
          <path d="M17 18C17 15.79 18.79 14 21 14H27C29.21 14 31 15.79 31 18V22C31 24.21 29.21 26 27 26H21C18.79 26 17 24.21 17 22V18Z" stroke="#4F6EF7" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
          <circle cx="30" cy="30" r="5" fill="#4F6EF7"/>
          <path d="M28 30H32" stroke="white" strokeWidth="1.5" strokeLinecap="round"/>
        </svg>
      </div>
      <h2 className="step-title">输入博主主页地址</h2>
      <p className="step-desc">粘贴你想下载微博的用户主页链接</p>
      <div className="input-group">
        <input
          className={`wizard-input${profileUrl.length > 0 && !isValidProfileUrl(profileUrl) ? " error" : ""}${valid ? " success" : ""}`}
          type="text"
          placeholder="https://www.weibo.com/u/2166767661"
          value={profileUrl}
          onChange={(e) => onProfileUrlChange(e.target.value.trim())}
          onKeyDown={handleKey}
          autoFocus
        />
        {valid && <span className="input-check">✓</span>}
      </div>
      {profileUrl.length > 0 && !isValidProfileUrl(profileUrl) && (
        <p className="input-error">请输入有效的微博主页地址</p>
      )}
      <p className="step-example">示例：https://www.weibo.com/u/2166767661</p>
      <button
        className="btn-wizard btn-wizard-primary"
        onClick={onNext}
        disabled={!valid}
        type="button"
      >
        下一步
      </button>
    </div>
  );
}
