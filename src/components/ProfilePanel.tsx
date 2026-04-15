import { useState } from "react";
import type { ProfileResult } from "../types/contracts";

interface ProfilePanelProps {
  profile: ProfileResult;
  authorName: string;
  onExportOpenclaw: () => Promise<string>;
  onClose: () => void;
}

const sectionStyle: React.CSSProperties = {
  padding: "20px",
  borderRadius: "var(--radius-lg)",
  background: "var(--color-bg-inset)",
  border: "1px solid var(--color-border-subtle)",
  marginBottom: 14,
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: 14,
  fontWeight: 700,
  color: "var(--color-text)",
  marginBottom: 12,
  display: "flex",
  alignItems: "center",
  gap: 6,
};

const tagStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "3px 10px",
  borderRadius: "var(--radius-sm)",
  background: "var(--color-accent-subtle)",
  color: "var(--color-accent)",
  fontSize: 12,
  fontWeight: 500,
  lineHeight: 1.5,
};

const chipBase: React.CSSProperties = {
  display: "inline-block",
  padding: "4px 12px",
  borderRadius: "var(--radius-sm)",
  background: "var(--color-bg-hover)",
  color: "var(--color-text-secondary)",
  fontSize: 12,
  lineHeight: 1.5,
};

function sentimentColor(s: string): string {
  if (s.includes("积极") || s.includes("正面") || s.includes("positive")) return "var(--color-success)";
  if (s.includes("消极") || s.includes("负面") || s.includes("negative")) return "var(--color-danger)";
  return "var(--color-warning)";
}

function InfoItem({ emoji, label, value }: { emoji: string; label: string; value: string }) {
  return (
    <div style={{ fontSize: 13, color: "var(--color-text)", padding: "3px 0", lineHeight: 1.6 }}>
      {emoji} {label}: {value}
    </div>
  );
}

function TagList({ items }: { items: [string, number][] }) {
  if (items.length === 0) return null;
  const maxCount = items[0][1];
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
      {items.slice(0, 30).map(([word, count]) => (
        <span
          key={word}
          style={{
            ...chipBase,
            fontSize: Math.max(11, Math.min(16, 11 + (count / maxCount) * 5)),
            fontWeight: count / maxCount > 0.6 ? 600 : 400,
          }}
        >
          {word}
        </span>
      ))}
    </div>
  );
}

export default function ProfilePanel({ profile, authorName, onExportOpenclaw, onClose }: ProfilePanelProps) {
  const { basicStats, contentAnalysis, deepProfile } = profile;
  const [isExporting, setIsExporting] = useState(false);
  const [exportedPath, setExportedPath] = useState<string | null>(null);

  const handleExport = async () => {
    setIsExporting(true);
    setExportedPath(null);
    try {
      const path = await onExportOpenclaw();
      setExportedPath(path);
    } catch (err) {
      console.error("Export failed:", err);
    } finally {
      setIsExporting(false);
    }
  };

  const handleOpenDirectory = async () => {
    if (exportedPath) {
      try {
        const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
        await revealItemInDir(exportedPath);
      } catch (err) {
        console.error("Failed to open directory:", err);
      }
    }
  };

  return (
    <div
      style={{
        maxHeight: "70vh",
        overflowY: "auto",
        paddingRight: 4,
      }}
    >
      <div style={{ marginBottom: 20 }}>
        <h2 style={{ fontSize: 20, fontWeight: 700, color: "var(--color-text)", letterSpacing: "-0.01em" }}>
          博主画像分析报告
        </h2>
        <p style={{ fontSize: 13, color: "var(--color-text-tertiary)", marginTop: 6 }}>
          @{authorName} · 共 {basicStats.totalPosts} 条微博
        </p>
        <div style={{ marginTop: 12, height: 1, background: "var(--color-border-subtle)" }} />
      </div>

      {deepProfile && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>🎭 个人画像</div>
          <div style={{
            textAlign: "center",
            padding: "12px 0",
            marginBottom: 12,
            borderBottom: "1px solid var(--color-border-subtle)",
          }}>
            <span style={{
              fontSize: 28,
              fontWeight: 800,
              color: "var(--color-accent)",
              letterSpacing: "0.04em",
            }}>
              {deepProfile.personalInfo.mbtiType || "—"}
            </span>
          </div>
          <InfoItem emoji="🎂" label="星座" value={deepProfile.personalInfo.zodiacSign || "未知"} />
          <InfoItem emoji="🏙️" label="城市" value={deepProfile.personalInfo.possibleCities.join("、") || "未知"} />
          <InfoItem emoji="💼" label="职业" value={deepProfile.personalInfo.possibleOccupation || "未知"} />
          {deepProfile.personality.traits.length > 0 && (
            <InfoItem emoji="🎭" label="性格" value={deepProfile.personality.traits.join("、")} />
          )}
        </div>
      )}

      <div style={sectionStyle}>
        <div style={sectionTitleStyle}>📝 内容分析</div>
        {deepProfile?.keywords?.topKeywords && deepProfile.keywords.topKeywords.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 6 }}>关键词</div>
            <TagList items={deepProfile.keywords.topKeywords.map((k, i) => [k, i + 1] as [string, number])} />
          </div>
        )}
        {contentAnalysis.topHashtags.length > 0 && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 6 }}>话题</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {contentAnalysis.topHashtags.slice(0, 15).map(([tag]) => (
                <span key={tag} style={tagStyle}>#{tag}</span>
              ))}
            </div>
          </div>
        )}
        {contentAnalysis.topMentions.length > 0 && (
          <div>
            <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 6 }}>常互动用户</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {contentAnalysis.topMentions.slice(0, 10).map(([mention]) => (
                <span key={mention} style={{ ...chipBase, background: "var(--color-accent-subtle)", color: "var(--color-accent)" }}>
                  @{mention}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      {deepProfile && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>🎯 兴趣偏好</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {deepProfile.interests.music.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)", minWidth: 50 }}>🎵 音乐</span>
                <span style={{ fontSize: 13, color: "var(--color-text)" }}>{deepProfile.interests.music.join("、")}</span>
              </div>
            )}
            {deepProfile.interests.books.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)", minWidth: 50 }}>📚 书籍</span>
                <span style={{ fontSize: 13, color: "var(--color-text)" }}>{deepProfile.interests.books.join("、")}</span>
              </div>
            )}
            {deepProfile.interests.movies.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)", minWidth: 50 }}>🎬 影视</span>
                <span style={{ fontSize: 13, color: "var(--color-text)" }}>{deepProfile.interests.movies.join("、")}</span>
              </div>
            )}
            {deepProfile.interests.hobbies.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)", minWidth: 50 }}>🎮 爱好</span>
                <span style={{ fontSize: 13, color: "var(--color-text)" }}>{deepProfile.interests.hobbies.join("、")}</span>
              </div>
            )}
            {deepProfile.interests.sports.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)", minWidth: 50 }}>⚽ 运动</span>
                <span style={{ fontSize: 13, color: "var(--color-text)" }}>{deepProfile.interests.sports.join("、")}</span>
              </div>
            )}
            {deepProfile.interests.food.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)", minWidth: 50 }}>🍜 美食</span>
                <span style={{ fontSize: 13, color: "var(--color-text)" }}>{deepProfile.interests.food.join("、")}</span>
              </div>
            )}
          </div>
        </div>
      )}

      {deepProfile && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>💭 价值观</div>
          <InfoItem emoji="🏛️" label="政治倾向" value={deepProfile.values.politicalStance} />
          <InfoItem emoji="🧭" label="人生哲学" value={deepProfile.values.philosophy} />
          <InfoItem emoji="🌍" label="世界观" value={deepProfile.values.worldview} />
          <InfoItem emoji="🌱" label="生活态度" value={deepProfile.values.attitudeTowardLife} />
          {deepProfile.values.coreValues.length > 0 && (
            <div style={{ marginTop: 8, display: "flex", flexWrap: "wrap", gap: 6 }}>
              {deepProfile.values.coreValues.map((v) => (
                <span key={v} style={tagStyle}>{v}</span>
              ))}
            </div>
          )}
        </div>
      )}

      {deepProfile?.interestingInsights && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>🔮 有趣的发现</div>
          {deepProfile.interestingInsights.contradictions.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 6 }}>矛盾点</div>
              {deepProfile.interestingInsights.contradictions.map((c, i) => (
                <div key={i} style={{
                  padding: "8px 12px",
                  marginBottom: 8,
                  borderRadius: "var(--radius-sm)",
                  background: "var(--color-bg-hover)",
                  fontSize: 13,
                }}>
                  <div style={{ fontWeight: 500, color: "var(--color-text)", marginBottom: 4 }}>
                    {c.what}
                  </div>
                  {c.evidence.length > 0 && (
                    <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", lineHeight: 1.5 }}>
                      {c.evidence.map((e, j) => (
                        <div key={j} style={{ fontStyle: "italic" }}>↳ {e}</div>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
          {deepProfile.interestingInsights.hiddenPatterns.length > 0 && (
            <div>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 6 }}>隐藏模式</div>
              {deepProfile.interestingInsights.hiddenPatterns.map((p, i) => (
                <div key={i} style={{
                  padding: "6px 12px",
                  marginBottom: 4,
                  borderRadius: "var(--radius-sm)",
                  background: "var(--color-accent-subtle)",
                  color: "var(--color-accent)",
                  fontSize: 13,
                }}>
                  {p}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {deepProfile?.interestingInsights?.growthArc && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>📈 成长轨迹</div>
          <div style={{
            padding: "12px",
            borderRadius: "var(--radius-sm)",
            background: "var(--color-bg-hover)",
            marginBottom: 12,
            textAlign: "center",
            fontStyle: "italic",
            color: "var(--color-text-secondary)",
            fontSize: 14,
          }}>
            {deepProfile.interestingInsights.growthArc.narrative}
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 4 }}>
                {deepProfile.interestingInsights.growthArc.then.period}
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                {deepProfile.interestingInsights.growthArc.then.keywords.map((k) => (
                  <span key={k} style={{
                    padding: "3px 8px",
                    borderRadius: "var(--radius-sm)",
                    background: "var(--color-border-subtle)",
                    fontSize: 12,
                  }}>{k}</span>
                ))}
              </div>
            </div>
            <div>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginBottom: 4 }}>
                {deepProfile.interestingInsights.growthArc.now.period}
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                {deepProfile.interestingInsights.growthArc.now.keywords.map((k) => (
                  <span key={k} style={{
                    padding: "3px 8px",
                    borderRadius: "var(--radius-sm)",
                    background: "var(--color-accent-subtle)",
                    color: "var(--color-accent)",
                    fontSize: 12,
                  }}>{k}</span>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {deepProfile?.interestingInsights?.socialRoles && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>🎭 社交角色</div>
          <InfoItem emoji="👥" label="在朋友圈" value={deepProfile.interestingInsights.socialRoles.inFriendCircle} />
          <InfoItem emoji="💬" label="在评论区" value={deepProfile.interestingInsights.socialRoles.inComments} />
          <InfoItem emoji="🚨" label="出事时" value={deepProfile.interestingInsights.socialRoles.inCrisis} />
        </div>
      )}

      {deepProfile && (
        <div style={sectionStyle}>
          <div style={sectionTitleStyle}>❤️ 情感分析</div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
            <span style={{
              display: "inline-block",
              padding: "3px 12px",
              borderRadius: 100,
              background: `color-mix(in srgb, ${sentimentColor(deepProfile.sentiment.overallSentiment)} 15%, transparent)`,
              color: sentimentColor(deepProfile.sentiment.overallSentiment),
              fontSize: 12,
              fontWeight: 600,
            }}>
              {deepProfile.sentiment.overallSentiment}
            </span>
            <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
              情绪稳定性: {deepProfile.sentiment.emotionalStability}
            </span>
          </div>
          <div style={{ marginBottom: 4 }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>幸福感指数</span>
              <span style={{ fontSize: 12, color: "var(--color-text)", fontWeight: 600, fontVariantNumeric: "tabular-nums" }}>
                {deepProfile.sentiment.happinessIndex}%
              </span>
            </div>
            <div style={{
              width: "100%",
              height: 8,
              borderRadius: 4,
              background: "var(--color-border)",
              overflow: "hidden",
            }}>
              <div style={{
                width: `${Math.max(0, Math.min(100, deepProfile.sentiment.happinessIndex))}%`,
                height: "100%",
                borderRadius: 4,
                background: "var(--color-accent)",
                transition: "width 400ms cubic-bezier(0.16, 1, 0.3, 1)",
              }} />
            </div>
          </div>
        </div>
      )}

      <div style={{
        display: "flex",
        gap: 8,
        marginTop: 16,
        marginBottom: 8,
      }}>
        <button
          className="btn btn-outline-accent"
          style={{ flex: exportedPath ? 1 : 2 }}
          onClick={handleExport}
          disabled={isExporting}
          type="button"
        >
          {isExporting ? "导出中..." : "📄 导出画像分析"}
        </button>
        {exportedPath && (
          <button
            className="btn btn-primary"
            style={{ flex: 1 }}
            onClick={handleOpenDirectory}
            type="button"
          >
            📂 打开目录
          </button>
        )}
        <button className="btn btn-secondary" style={{ flex: exportedPath ? 1 : 2 }} onClick={onClose} type="button">
          关闭
        </button>
      </div>

      <div style={{
        fontSize: 11,
        color: "var(--color-text-tertiary)",
        textAlign: "center",
        paddingBottom: 8,
        lineHeight: 1.5,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 4,
      }}>
        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M8 1L15 14H1L8 1Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"/>
          <path d="M8 6V9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
          <circle cx="8" cy="11.5" r="0.75" fill="currentColor"/>
        </svg>
        基于微博内容的AI推测分析，仅供参考
      </div>
    </div>
  );
}
