import { useState } from "react";
import type { UsageMetricPoint } from "../types";
import { computeTrendPoints, formatMetricValue } from "../lib/metricsFormat";

export function MetricTrendLine({
  history,
  unit,
  height = 30,
  interactive = false,
}: {
  history?: UsageMetricPoint[];
  unit?: string | null;
  height?: number;
  interactive?: boolean;
}) {
  const [hoverIdx, setHoverIdx] = useState<number | null>(null);
  const width = 260;
  const pts = computeTrendPoints(history, width, height, 4);
  if (pts.length < 2) return null;

  const polyline = pts.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ");
  const last = pts[pts.length - 1];
  const hovered = hoverIdx != null && pts[hoverIdx] ? pts[hoverIdx] : null;

  return (
    <div className="relative select-none" data-testid="metric-trend-line">
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="w-full overflow-visible"
        style={{ height: `${height}px` }}
        onMouseLeave={() => setHoverIdx(null)}
      >
        <polyline
          fill="none"
          stroke={interactive ? "var(--accent)" : "currentColor"}
          strokeOpacity={interactive ? 0.9 : 0.75}
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
          points={polyline}
        />
        <circle
          cx={last.x}
          cy={last.y}
          r="2.8"
          fill={interactive ? "var(--accent)" : "currentColor"}
        />
        {interactive &&
          pts.map((p, idx) => (
            <circle
              key={p.t}
              cx={p.x}
              cy={p.y}
              r="6"
              fill="transparent"
              className="cursor-crosshair"
              onMouseEnter={() => setHoverIdx(idx)}
            />
          ))}
        {interactive && hovered && (
          <circle
            cx={hovered.x}
            cy={hovered.y}
            r="3.5"
            fill="var(--text-1)"
          />
        )}
      </svg>
      {interactive && hovered && (
        <div className="text-[10.5px] font-mono text-[var(--text-2)] flex justify-between pt-0.5">
          <span>{new Date(hovered.t).toLocaleTimeString()}</span>
          <span className="font-semibold text-[var(--text-1)]">
            {formatMetricValue(hovered.v, unit)}
          </span>
        </div>
      )}
    </div>
  );
}
