import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ProfilePanel from "../components/ProfilePanel";
import type { BasicStats, ProfileResult } from "../types/contracts";

vi.mock("../lib/tauri-bridge", () => ({
  openOutputDir: vi.fn(),
  setAiKey: vi.fn(),
  clearAiKey: vi.fn(),
}));

const stats: BasicStats = {
  totalPosts: 5,
  totalOriginal: 4,
  totalReposts: 1,
  avgTextLength: 3.2,
  maxTextLength: 9,
  minTextLength: 1,
  postsPerDay: 1,
  postsPerWeek: 5,
  postsPerMonth: 30,
  totalActiveDays: 3,
  mostActiveDate: "2024-03-18",
  activeHours: { peakStart: 13, peakEnd: 15 },
  hourlyDistribution: [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0],
  weeklyDistribution: [2, 1, 0, 0, 1, 0, 0],
  monthlyDistribution: [["2024-03", 4]],
};

const localOnly: ProfileResult = {
  authorName: "测试",
  basicStats: stats,
  contentAnalysis: {
    topHashtags: [["春天", 1]],
    topMentions: [],
    avgHashtagsPerPost: 0.2,
    avgMentionsPerPost: 0,
    postsWithImagesRatio: 0.2,
    emojiFrequency: [],
  },
  influenceMetrics: { sourceDistribution: [], regionDistribution: [["北京", 1]], originalRatio: 0.8 },
  deepProfile: null,
  aiConfigured: false,
  aiError: null,
};

const renderPanel = (profile: ProfileResult) =>
  render(
    <ProfilePanel
      profile={profile}
      authorName="测试"
      onExportOpenclaw={vi.fn().mockResolvedValue("/tmp/report.md")}
      onClose={vi.fn()}
      onAnalyze={vi.fn()}
    />,
  );

describe("ProfilePanel", () => {
  it("shows the local statistics to someone who has no AI key", () => {
    renderPanel(localOnly);

    expect(screen.getByText("📊 基础统计")).toBeTruthy();
    expect(screen.getByText(/5 条（原创 80%）/)).toBeTruthy();
    expect(screen.getByText(/最活跃的是 13 点到 15 点/)).toBeTruthy();
    expect(screen.getByTitle("14: 1")).toBeTruthy();
    expect(screen.getByText(/北京/)).toBeTruthy();
    expect(screen.getByText(/以上统计在本地计算/)).toBeTruthy();
    expect(screen.queryByText(/AI推测分析/)).toBeNull();
  });

  it("says why the AI part is missing when a key is set but the call failed", () => {
    renderPanel({ ...localOnly, aiConfigured: true, aiError: "余额不足" });

    expect(screen.getByRole("alert")).toHaveProperty("textContent", "AI 分析没有成功：余额不足");
  });
});
