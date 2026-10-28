import type { Lang } from "./i18n";
import { translate } from "./i18n";
import { formatMoney, normalizeDisplayCurrency, normalizeRate } from "./currency";

export function formatShortTokens(tokens: number, lang: Lang): string {
  if (!Number.isFinite(tokens) || tokens <= 0) return "0";
  if (lang === "zh") {
    if (tokens >= 100_000_000) {
      const val = tokens / 100_000_000;
      return `${val >= 10 ? val.toFixed(1) : val.toFixed(2).replace(/\.?0+$/, "")}亿`;
    }
    if (tokens >= 10_000) {
      const val = tokens / 10_000;
      return `${val >= 100 ? Math.round(val) : val.toFixed(1).replace(/\.0$/, "")}万`;
    }
    return Math.round(tokens).toLocaleString("zh-CN");
  }
  if (tokens >= 1_000_000_000) {
    const val = tokens / 1_000_000_000;
    return `${val >= 10 ? val.toFixed(1) : val.toFixed(2).replace(/\.?0+$/, "")}B`;
  }
  if (tokens >= 1_000_000) {
    const val = tokens / 1_000_000;
    return `${val >= 100 ? Math.round(val) : val.toFixed(1).replace(/\.0$/, "")}M`;
  }
  if (tokens >= 1_000) {
    const val = tokens / 1_000;
    return `${val >= 100 ? Math.round(val) : val.toFixed(1).replace(/\.0$/, "")}K`;
  }
  return Math.round(tokens).toLocaleString("en-US");
}

export function formatUsdEstimateForCard(
  usd: number,
  displayCurrency: string | undefined,
  usdCnyRate: number | undefined,
): string {
  const dc = normalizeDisplayCurrency(displayCurrency);
  const rate = normalizeRate(usdCnyRate);
  const amount = dc === "CNY" ? usd * rate : usd;
  return formatMoney(amount, dc);
}

export function formatRelativeUpdate(
  checkedAt: string | null,
  nowMs: number,
  lang: Lang,
): string | null {
  if (!checkedAt) return null;
  const ts = Date.parse(checkedAt);
  if (Number.isNaN(ts)) return null;
  const diffSec = Math.max(0, Math.floor((nowMs - ts) / 1000));
  if (diffSec < 60) {
    return translate(lang, "detail_card.updated_just_now");
  }
  const mins = Math.floor(diffSec / 60);
  if (mins < 60) {
    const ago = translate(lang, "rail.warn.time.minutes", { n: mins });
    return translate(lang, "detail_card.updated_ago", { ago });
  }
  const hours = Math.floor(mins / 60);
  const ago = translate(lang, "rail.warn.time.hours", { n: hours });
  return translate(lang, "detail_card.updated_ago", { ago });
}

export function formatCacheTierLabel(lifetimeSeconds: number, lang: Lang): string {
  if (lifetimeSeconds >= 3600) return translate(lang, "detail_card.tier_1h");
  if (lifetimeSeconds >= 1800) return translate(lang, "detail_card.tier_30m");
  return translate(lang, "detail_card.tier_5m");
}
