import { useState } from "react";
import { clearAiKey, setAiKey } from "../lib/tauri-bridge";

interface AiKeySetupProps {
  /** A key is stored. */
  configured: boolean;
  /** Why the last AI analysis failed although a key is stored. */
  error?: string | null;
  /** A key was just saved; the caller can run the analysis again. */
  onSaved: () => void;
}

const sectionStyle: React.CSSProperties = {
  padding: "16px 20px",
  borderRadius: "var(--radius-lg)",
  background: "var(--color-bg-inset)",
  border: "1px solid var(--color-border-subtle)",
  marginBottom: 14,
};

/**
 * The AI analysis is opt-in: it sends the blogger's posts to DeepSeek, so the user has to bring a
 * key and is told what it is used for before anything leaves this machine.
 */
export default function AiKeySetup({ configured, error, onSaved }: AiKeySetupProps) {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const describe = (cause: unknown) => (cause instanceof Error ? cause.message : String(cause));

  const save = async () => {
    const trimmed = key.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    setMessage(null);
    try {
      await setAiKey(trimmed);
      onSaved();
    } catch (cause) {
      setMessage(`保存失败：${describe(cause)}`);
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    setMessage(null);
    try {
      await clearAiKey();
      setMessage("已清除密钥。重新打开画像分析后，只剩本地统计。");
    } catch (cause) {
      setMessage(`清除失败：${describe(cause)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section style={sectionStyle}>
      <div style={{ fontSize: 14, fontWeight: 700, color: "var(--color-text)", marginBottom: 12 }}>🤖 AI 深度画像</div>

      {configured && error && (
        <p role="alert" style={{ fontSize: 12, color: "var(--color-danger)", lineHeight: 1.6, marginBottom: 8 }}>
          AI 分析没有成功：{error}
        </p>
      )}

      <p style={{ fontSize: 12, color: "var(--color-text-secondary)", lineHeight: 1.6, marginBottom: 10 }}>
        {configured ? "已保存密钥。" : "尚未保存密钥，以上只有本地统计。"}
        AI 分析会把最多 200 条微博发送给 DeepSeek，请只对你有权处理的内容使用。
      </p>

      <div style={{ display: "flex", gap: 8 }}>
        <input
          type="password"
          className="input"
          style={{ flex: 1, height: 32 }}
          placeholder="DeepSeek API Key"
          aria-label="DeepSeek API Key"
          autoComplete="off"
          value={key}
          onChange={(event) => setKey(event.target.value)}
        />
        <button
          className="btn btn-primary"
          type="button"
          disabled={busy || !key.trim()}
          onClick={() => void save()}
        >
          {busy ? "保存中..." : configured ? "更换密钥" : "保存并分析"}
        </button>
      </div>

      {configured && (
        <button className="btn btn-ghost" type="button" disabled={busy} onClick={() => void remove()} style={{ marginTop: 8 }}>
          清除已保存的密钥
        </button>
      )}

      {message && (
        <p role="alert" style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 8 }}>
          {message}
        </p>
      )}
    </section>
  );
}
