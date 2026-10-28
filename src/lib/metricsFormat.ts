import type { UsageMetricPoint } from "../types";

export function formatMetricValue(
  value: number,
  unit?: string | null,
): string {
  if (!Number.isFinite(value)) return "—";
  let numStr: string;
  if (value !== 0 && Math.abs(value) < 0.01) {
    numStr = Number(value.toPrecision(2)).toString();
  } else {
    numStr = Number(value.toFixed(2)).toString();
  }
  const cleanUnit = unit?.trim();
  if (!cleanUnit) return numStr;
  return cleanUnit === "%" ? `${numStr}%` : `${numStr} ${cleanUnit}`;
}

export interface PlottedPoint {
  x: number;
  y: number;
  t: string;
  v: number;
}

export function computeTrendPoints(
  history: UsageMetricPoint[] | undefined,
  width: number,
  height: number,
  inset = 4,
): PlottedPoint[] {
  if (!history || history.length < 2) return [];
  const parsed = history
    .map((pt) => ({
      ms: Date.parse(pt.t),
      t: pt.t,
      v: pt.v,
    }))
    .filter((pt) => !Number.isNaN(pt.ms) && Number.isFinite(pt.v));
  if (parsed.length < 2) return [];

  const firstMs = parsed[0].ms;
  const lastMs = parsed[parsed.length - 1].ms;
  const spanMs = lastMs - firstMs;
  if (spanMs <= 0) return [];

  let low = parsed[0].v;
  let high = parsed[0].v;
  for (const p of parsed) {
    if (p.v < low) low = p.v;
    if (p.v > high) high = p.v;
  }
  const range = high - low;
  const innerW = Math.max(1, width - inset * 2);
  const innerH = Math.max(1, height - inset * 2);

  return parsed.map((p) => {
    const x = inset + innerW * ((p.ms - firstMs) / spanMs);
    const y =
      range <= 1e-12
        ? height / 2
        : inset + innerH * (1 - (p.v - low) / range);
    return { x, y, t: p.t, v: p.v };
  });
}
