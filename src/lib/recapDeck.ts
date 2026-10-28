import type { RecapProjectItem, RecapReport } from "../types";
import { translate, type Lang } from "./i18n";
import { formatShortTokens, formatUsdEstimateForCard } from "./detailedCard";

export type RecapCardId =
  | "poster"
  | "opener"
  | "calendar"
  | "year_calendar"
  | "months"
  | "timetable"
  | "payback"
  | "scorecard";

export interface RecapCardEntry {
  id: RecapCardId;
  numberLabel: string;
  title: string;
}

export function buildRecapDeck(
  report: RecapReport | null,
  monthlyPriceUsd: number,
  lang: Lang,
): RecapCardEntry[] {
  if (!report || report.tokens <= 0) return [];

  const numberedIds: RecapCardId[] = [];
  if (report.agents.length > 0) {
    numberedIds.push("opener");
  }
  if (report.period.kind === "month" && report.days.some((d) => d.tokens > 0)) {
    numberedIds.push("calendar");
  }
  if (report.period.kind === "year" && report.days.some((d) => d.tokens > 0)) {
    numberedIds.push("year_calendar");
  }
  if (
    report.period.kind === "year" &&
    report.months.length === 12 &&
    report.months.some((m) => m.tokens > 0)
  ) {
    numberedIds.push("months");
  }
  if (
    report.hours.length === 24 &&
    report.peak_hour != null &&
    report.hours.some((h) => h > 0)
  ) {
    numberedIds.push("timetable");
  }
  if (report.cost_usd > 0 && monthlyPriceUsd > 0 && report.paid_months > 0) {
    numberedIds.push("payback");
  }
  numberedIds.push("scorecard");

  const totalNumbered = String(numberedIds.length).padStart(2, "0");
  const deck: RecapCardEntry[] = [
    {
      id: "poster",
      numberLabel: translate(lang, "recap.card.poster"),
      title: translate(lang, "recap.card.poster"),
    },
  ];

  numberedIds.forEach((id, idx) => {
    const num = String(idx + 1).padStart(2, "0");
    deck.push({
      id,
      numberLabel: `${num} / ${totalNumbered}`,
      title: translate(lang, `recap.card.${id}`),
    });
  });

  return deck;
}

export function maskProjects(
  projects: RecapProjectItem[],
  hideProjects: boolean,
  lang: Lang,
): RecapProjectItem[] {
  if (!hideProjects) return projects;
  return projects.map((p, idx) => ({
    ...p,
    name: translate(lang, "recap.project_masked", { n: idx + 1 }),
  }));
}

export function computePaybackMultiple(
  report: RecapReport,
  monthlyPriceUsd: number,
): { paidUsd: number; multiple: number } | null {
  if (
    !Number.isFinite(monthlyPriceUsd) ||
    monthlyPriceUsd <= 0 ||
    report.paid_months <= 0 ||
    report.cost_usd <= 0
  ) {
    return null;
  }
  const paidUsd = monthlyPriceUsd * report.paid_months;
  if (paidUsd <= 0) return null;
  return {
    paidUsd,
    multiple: report.cost_usd / paidUsd,
  };
}

export function formatLatestMinute(latestMinute: number | null): string {
  if (latestMinute == null) return "—";
  const wrapped = latestMinute % 1440;
  const h = Math.floor(wrapped / 60);
  const m = wrapped % 60;
  const prefix = latestMinute >= 1440 ? "+1d " : "";
  return `${prefix}${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

export function renderRecapCardToCanvas(
  canvas: HTMLCanvasElement,
  card: RecapCardEntry,
  report: RecapReport,
  options: {
    monthlyPriceUsd: number;
    hideProjects: boolean;
    displayCurrency?: string;
    usdCnyRate?: number;
    lang: Lang;
  },
): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  canvas.width = 1080;
  canvas.height = 1920;

  const { lang, monthlyPriceUsd, hideProjects, displayCurrency, usdCnyRate } =
    options;
  const projects = maskProjects(report.projects, hideProjects, lang);
  const payback = computePaybackMultiple(report, monthlyPriceUsd);

  // Editorial paper background (#F5F5F1)
  ctx.fillStyle = "#F5F5F1";
  ctx.fillRect(0, 0, 1080, 1920);

  // Outer card frame
  ctx.strokeStyle = "#D8D8D0";
  ctx.lineWidth = 3;
  ctx.strokeRect(48, 48, 984, 1824);

  // Header bar
  ctx.fillStyle = "#1B1B1E";
  ctx.font = "700 32px sans-serif";
  ctx.fillText(`PULSE RECAP · ${report.period.label.toUpperCase()}`, 88, 118);

  ctx.fillStyle = "#6B6B65";
  ctx.font = "600 28px monospace";
  ctx.textAlign = "right";
  ctx.fillText(card.numberLabel, 992, 118);
  ctx.textAlign = "left";

  // Lime accent bar (#C8F03C)
  ctx.fillStyle = "#C8F03C";
  ctx.fillRect(88, 148, 904, 140);
  ctx.strokeStyle = "#1B1B1E";
  ctx.lineWidth = 2;
  ctx.strokeRect(88, 148, 904, 140);

  ctx.fillStyle = "#1B1B1E";
  ctx.font = "800 68px monospace";
  const heroTokens = `${formatShortTokens(report.tokens, lang)} TOKENS`;
  ctx.fillText(heroTokens, 116, 242);

  // Card Title
  ctx.fillStyle = "#1B1B1E";
  ctx.font = "700 44px sans-serif";
  ctx.fillText(card.title, 88, 360);

  // White content box
  ctx.fillStyle = "#FFFFFF";
  ctx.fillRect(88, 396, 904, 1340);
  ctx.strokeStyle = "#E2E2DC";
  ctx.lineWidth = 2;
  ctx.strokeRect(88, 396, 904, 1340);

  const moneyStr = formatUsdEstimateForCard(
    report.cost_usd,
    displayCurrency,
    usdCnyRate,
  );
  const rows: [string, string][] = [
    [
      lang === "en" ? "Estimated API Value" : "API 估算价值",
      `${moneyStr}${report.unpriced_tokens > 0 ? "*" : ""}`,
    ],
    [
      lang === "en" ? "Active Days" : "活跃天数",
      `${report.active_days} / ${report.observed_days}`,
    ],
    [
      lang === "en" ? "Daily Average" : "日均 Token",
      formatShortTokens(report.daily_average_tokens, lang),
    ],
    [
      lang === "en" ? "Streak (Current / Longest)" : "连续打卡（当前 / 最长）",
      `${report.current_streak}d / ${report.longest_streak}d`,
    ],
    [
      lang === "en" ? "Sessions" : "会话总数",
      String(report.sessions),
    ],
    [
      lang === "en" ? "Peak Hour" : "高峰时段",
      report.peak_hour != null ? `${String(report.peak_hour).padStart(2, "0")}:00` : "—",
    ],
    [
      lang === "en" ? "Latest Finish" : "最晚收工",
      formatLatestMinute(report.latest_minute),
    ],
    [
      lang === "en" ? "Cache Hit Rate" : "缓存命中率",
      report.cache_hit_rate != null
        ? `${Math.round(report.cache_hit_rate * 100)}%`
        : "—",
    ],
  ];

  if (payback) {
    rows.push([
      lang === "en" ? "Subscription Payback" : "订阅回本倍数",
      `${payback.multiple.toFixed(1)}×`,
    ]);
  }

  let y = 468;
  for (const [k, v] of rows) {
    ctx.fillStyle = "#6B6B65";
    ctx.font = "500 30px sans-serif";
    ctx.fillText(k, 128, y);

    ctx.fillStyle = "#1B1B1E";
    ctx.font = "700 32px monospace";
    ctx.textAlign = "right";
    ctx.fillText(v, 952, y);
    ctx.textAlign = "left";

    ctx.strokeStyle = "#EFEFE9";
    ctx.beginPath();
    ctx.moveTo(128, y + 20);
    ctx.lineTo(952, y + 20);
    ctx.stroke();
    y += 74;
  }

  // Top Models & Projects section
  y += 24;
  ctx.fillStyle = "#1B1B1E";
  ctx.font = "700 32px sans-serif";
  ctx.fillText(
    lang === "en" ? "TOP MODELS & PROJECTS" : "主力模型与项目",
    128,
    y,
  );
  y += 52;

  for (const m of report.models.slice(0, 3)) {
    ctx.fillStyle = "#1B1B1E";
    ctx.font = "600 28px monospace";
    ctx.fillText(m.model.slice(0, 36), 128, y);
    ctx.textAlign = "right";
    ctx.fillText(`${Math.round(m.share * 100)}%`, 952, y);
    ctx.textAlign = "left";
    y += 46;
  }

  y += 20;
  for (const p of projects.slice(0, 3)) {
    ctx.fillStyle = "#4A4A45";
    ctx.font = "600 28px sans-serif";
    ctx.fillText(p.name.slice(0, 36), 128, y);
    ctx.textAlign = "right";
    ctx.fillText(`${formatShortTokens(p.tokens, lang)}`, 952, y);
    ctx.textAlign = "left";
    y += 46;
  }

  // Footer
  ctx.fillStyle = "#1B1B1E";
  ctx.font = "700 26px sans-serif";
  ctx.fillText(
    translate(lang, `recap.persona.${report.persona}`),
    88,
    1810,
  );
}
