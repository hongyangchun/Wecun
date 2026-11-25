export interface Bar {
  label: string;
  value: number;
}

interface BarChartProps {
  values: Bar[];
  /** Index of the bar to draw in the accent color. */
  highlight?: number;
  /** Show every n-th label, so that narrow bars do not overlap. */
  labelEvery?: number;
}

/** A bar chart made of plain divs: no chart library for a handful of numbers. */
export default function BarChart({ values, highlight, labelEvery = 1 }: BarChartProps) {
  const max = Math.max(1, ...values.map((bar) => bar.value));

  return (
    <div role="img" aria-label="柱状图" style={{ display: "flex", alignItems: "stretch", gap: 2, height: 86 }}>
      {values.map((bar, index) => (
        <div
          key={`${bar.label}-${index}`}
          title={`${bar.label}: ${bar.value}`}
          style={{ flex: "1 1 0", minWidth: 0, display: "flex", flexDirection: "column", justifyContent: "flex-end" }}
        >
          <div
            style={{
              height: `${Math.max(2, (bar.value / max) * 100)}%`,
              minHeight: 2,
              borderRadius: 3,
              background: index === highlight ? "var(--color-accent)" : "var(--color-accent-muted)",
            }}
          />
          <div
            style={{
              height: 14,
              marginTop: 2,
              fontSize: 9,
              lineHeight: "14px",
              textAlign: "center",
              color: index === highlight ? "var(--color-accent)" : "var(--color-text-tertiary)",
              overflow: "hidden",
              whiteSpace: "nowrap",
            }}
          >
            {index % labelEvery === 0 ? bar.label : ""}
          </div>
        </div>
      ))}
    </div>
  );
}
