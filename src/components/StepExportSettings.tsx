import { pickDirectory } from "../lib/dialog";

interface StepExportSettingsProps {
  outputDir: string;
  onDirChange: (dir: string) => void;
}

export default function StepExportSettings({
  outputDir,
  onDirChange,
}: StepExportSettingsProps) {
  const handlePick = async () => {
    const dir = await pickDirectory();
    if (dir) onDirChange(dir);
  };

  return (
    <div className="step-enter">
      <h2 className="step-heading">
        保存位置
      </h2>
      <p className="form-hint" style={{ marginBottom: 20 }}>选择微博数据的保存位置</p>

      <div className="form-field">
        <span className="form-label">保存位置</span>
        <button className="btn btn-outline-accent" style={{ width: "100%", height: 40 }} onClick={handlePick} type="button">
          选择目录
        </button>
        {outputDir && (
          <div className="dir-display">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true" style={{ flexShrink: 0, marginTop: 2 }}>
              <path d="M2 4H5L6.5 6H12V11H2V4Z" stroke="var(--color-text-tertiary)" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="dir-display-path">{outputDir}</span>
          </div>
        )}
      </div>
    </div>
  );
}
