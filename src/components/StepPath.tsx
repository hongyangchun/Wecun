import { pickDirectory } from "../lib/dialog";

interface StepPathProps {
  outputDir: string;
  onDirChange: (dir: string) => void;
  onBack: () => void;
  onStart: () => void;
}

export default function StepPath({ outputDir, onDirChange, onBack, onStart }: StepPathProps) {
  const handlePick = async () => {
    const dir = await pickDirectory();
    if (dir) onDirChange(dir);
  };

  return (
    <div className="step-content step-path">
      <div className="step-icon">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <circle className="icon-soft" cx="24" cy="24" r="22" />
          <path className="icon-accent" d="M16 18V32C16 33.1 16.9 34 18 34H30C31.1 34 32 33.1 32 32V20C32 18.9 31.1 18 30 18H24L20 14H18C16.9 14 16 14.9 16 16V18Z" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      </div>
      <h2 className="step-title">选择保存位置</h2>
      <p className="step-desc">选择微博文件的保存目录</p>
      <button className="btn-wizard btn-wizard-outline" onClick={handlePick} type="button">
        选择目录
      </button>
      {outputDir && (
        <div className="path-display">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path className="icon-muted" d="M2 4H5L6.5 6H12V11H2V4Z" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
          <span className="path-text">{outputDir}</span>
        </div>
      )}
      <div className="wizard-actions">
        <button className="btn-wizard btn-wizard-secondary" onClick={onBack} type="button">上一步</button>
        <button className="btn-wizard btn-wizard-primary" onClick={onStart} disabled={!outputDir} type="button">
          开始下载
        </button>
      </div>
    </div>
  );
}
