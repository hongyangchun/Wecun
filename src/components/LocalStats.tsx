import type { BasicStats } from "../types/contracts";
import BarChart from "./ui/BarChart";

const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"];

const sectionStyle: React.CSSProperties = {
  padding: "16px 20px",
  borderRadius: "var(--radius-lg)",
  background: "var(--color-bg-inset)",
  border: "1px solid var(--color-border-subtle)",
  marginBottom: 14,
};

const titleStyle: React.CSSProperties = {
  fontSize: 14,
  fontWeight: 700,
  color: "var(--color-text)",
  marginBottom: 12,
};

const noteStyle: React.CSSProperties = {
  fontSize: 12,
  color: "var(--color-text-secondary)",
  lineHeight: 1.6,
};

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: "flex", gap: 8, fontSize: 13, padding: "3px 0", lineHeight: 1.6 }}>
      <span style={{ color: "var(--color-text-tertiary)", flexShrink: 0, minWidth: 112 }}>{label}</span>
      <span style={{ color: "var(--color-text)" }}>{value}</span>
    </div>
  );
}

const hour = (value: number) => String(value).padStart(2, "0");

/** What the saved posts say about when and how much someone posts. Computed on this machine. */
export default function LocalStats({ stats, regions }: { stats: BasicStats; regions: [string, number][] }) {
  const share = (count: number) => (stats.totalPosts > 0 ? Math.round((count / stats.totalPosts) * 100) : 0);
  const monthsShown = Math.min(24, stats.monthlyDistribution.length);

  return (
    <>
      <section style={sectionStyle}>
        <div style={titleStyle}>📊 基础统计</div>
        <Fact label="微博总数" value={`${stats.totalPosts} 条（原创 ${share(stats.totalOriginal)}%）`} />
        <Fact
          label="发帖频率"
          value={`平均每天 ${stats.postsPerDay.toFixed(1)} 条，共活跃 ${stats.totalActiveDays} 天`}
        />
        {stats.mostActiveDate && <Fact label="发帖最多的一天" value={stats.mostActiveDate} />}
        <Fact label="正文长度" value={`平均 ${Math.round(stats.avgTextLength)} 字，最长 ${stats.maxTextLength} 字`} />
        {regions.length > 0 && (
          <Fact label="常发地域" value={regions.slice(0, 5).map(([region]) => region).join("、")} />
        )}
      </section>

      <section style={sectionStyle}>
        <div style={titleStyle}>🕐 发帖时段</div>
        <BarChart
          values={stats.hourlyDistribution.map((count, at) => ({ label: hour(at), value: count }))}
          highlight={stats.activeHours.peakStart}
          labelEvery={4}
        />
        <p style={{ ...noteStyle, marginTop: 8 }}>
          最活跃的是 {hour(stats.activeHours.peakStart)} 点到 {hour(stats.activeHours.peakEnd)} 点
        </p>
      </section>

      <section style={sectionStyle}>
        <div style={titleStyle}>📅 星期与月份</div>
        <BarChart
          values={stats.weeklyDistribution.map((count, day) => ({ label: WEEKDAYS[day] ?? "", value: count }))}
        />
        {monthsShown > 0 && (
          <>
            <p style={{ ...noteStyle, margin: "12px 0 6px" }}>按月发帖（最近 {monthsShown} 个月）</p>
            <BarChart
              values={stats.monthlyDistribution
                .slice(-monthsShown)
                .map(([month, count]) => ({ label: month.slice(2), value: count }))}
              labelEvery={Math.max(1, Math.ceil(monthsShown / 8))}
            />
          </>
        )}
      </section>
    </>
  );
}
