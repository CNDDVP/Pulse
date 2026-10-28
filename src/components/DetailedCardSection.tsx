import type { AccountCardDetail, AppSettings } from "../types";
import { useLang } from "../lib/i18n";
import {
  formatCacheTierLabel,
  formatShortTokens,
  formatUsdEstimateForCard,
} from "../lib/detailedCard";

export function DetailedCardSection({
  detail,
  settings,
  nowMs,
}: {
  detail: AccountCardDetail;
  settings: AppSettings;
  nowMs: number;
}) {
  const { t, lang } = useLang();
  const act = detail.activity;
  const pc = detail.prompt_cache;

  const liveChats = (pc?.live_chats ?? []).filter((c) => {
    const exp = Date.parse(c.expires_at);
    return !Number.isNaN(exp) && exp > nowMs;
  });
  const soonest = liveChats[0];
  const soonestMin = soonest
    ? Math.max(1, Math.ceil((Date.parse(soonest.expires_at) - nowMs) / 60000))
    : 0;

  if (!act && !pc) return null;

  const peakDayTokens = act
    ? Math.max(1, ...act.days_31.map((d) => d.tokens))
    : 1;

  const renderMoney = (usd: number, unpriced: number) => {
    if (usd <= 0) return null;
    const formatted = formatUsdEstimateForCard(
      usd,
      settings.display_currency,
      settings.usd_cny_rate,
    );
    return `${formatted}${unpriced > 0 ? "*" : ""}`;
  };

  return (
    <div
      className="mt-3 pt-2.5 border-t border-[var(--border)] space-y-2.5"
      data-testid="detailed-card-section"
    >
      {act && (
        <>
          <div className="text-[10.5px] font-semibold uppercase tracking-wide text-[var(--text-3)]">
            {act.is_account_wide
              ? t("detail_card.whole_account")
              : t("detail_card.on_this_pc")}
          </div>

          <div className="grid grid-cols-3 gap-2">
            {[
              {
                label: t("detail_card.today"),
                tokens: act.today_tokens,
                cost: renderMoney(act.today_cost_usd, act.today_unpriced),
              },
              {
                label: t("detail_card.days_7"),
                tokens: act.week_tokens,
                cost: renderMoney(act.week_cost_usd, act.week_unpriced),
              },
              {
                label: t("detail_card.days_31"),
                tokens: act.month_tokens,
                cost: renderMoney(act.month_cost_usd, act.month_unpriced),
              },
            ].map((col) => (
              <div key={col.label} className="space-y-0.5">
                <div className="text-[10.5px] text-[var(--text-3)]">
                  {col.label}
                </div>
                <div className="text-sm font-semibold font-mono text-[var(--text-1)] leading-tight">
                  {formatShortTokens(col.tokens, lang)}
                </div>
                {col.cost && (
                  <div className="text-[10.5px] font-mono text-[var(--text-3)]">
                    {col.cost}
                  </div>
                )}
              </div>
            ))}
          </div>

          <div
            className="h-[30px] flex items-end gap-[2.5px] pt-1"
            aria-label="31-day usage chart"
          >
            {act.days_31.map((d, idx) => {
              const isToday = idx === act.days_31.length - 1;
              const ratio = d.tokens > 0 ? d.tokens / peakDayTokens : 0;
              const hPx =
                d.tokens > 0 ? Math.max(4, Math.round(ratio * 28)) : 2;
              const opacityCls =
                d.tokens === 0
                  ? "opacity-15"
                  : isToday
                  ? "opacity-90"
                  : "opacity-35";
              return (
                <div
                  key={d.day}
                  title={`${d.day}: ${d.tokens.toLocaleString()}`}
                  className={`flex-1 rounded-full bg-[var(--text-1)] ${opacityCls}`}
                  style={{ height: `${hPx}px` }}
                />
              );
            })}
          </div>

          <div className="space-y-1 pt-0.5">
            {act.top_model && (
              <div className="flex items-center justify-between gap-2 text-[11px]">
                <span className="text-[var(--text-3)] shrink-0">
                  {t("detail_card.top_model")}
                </span>
                <span className="font-mono text-[var(--text-1)] truncate">
                  {act.top_model.name} · {act.top_model.share_percent}%
                </span>
              </div>
            )}

            {act.cache_hit_rate != null && (
              <div className="flex items-center justify-between gap-2 text-[11px]">
                <span className="text-[var(--text-3)] shrink-0">
                  {t("detail_card.cache_hit_rate")}
                </span>
                <span className="font-mono text-[var(--text-1)]">
                  {Math.round(act.cache_hit_rate * 100)}%
                </span>
              </div>
            )}
          </div>
        </>
      )}

      {pc && (liveChats.length > 0 || pc.recently_lapsed) && (
        <div className="space-y-0.5 pt-0.5">
          <div className="flex items-center justify-between gap-2 text-[11px]">
            <span className="text-[var(--text-3)]">
              {liveChats.length > 1
                ? t("detail_card.prompt_cache_multi", {
                    count: liveChats.length,
                  })
                : t("detail_card.prompt_cache_single", {
                    tier: formatCacheTierLabel(
                      soonest?.lifetime_seconds ?? pc.lifetime_seconds,
                      lang,
                    ),
                  })}
            </span>
            <span
              className={`font-mono ${
                liveChats.length > 0
                  ? "text-[var(--text-1)]"
                  : "text-[var(--text-3)]"
              }`}
            >
              {liveChats.length === 0
                ? pc.is_minimum
                  ? t("detail_card.prompt_cache_lapsed")
                  : t("detail_card.prompt_cache_expired")
                : liveChats.length > 1
                ? pc.is_minimum
                  ? t("detail_card.prompt_cache_at_least", { min: soonestMin })
                  : t("detail_card.prompt_cache_soonest", { min: soonestMin })
                : pc.is_minimum
                ? t("detail_card.prompt_cache_at_least", { min: soonestMin })
                : t("detail_card.prompt_cache_left", { min: soonestMin })}
            </span>
          </div>
          {liveChats.length > 1 && soonest && (
            <div className="text-[10.5px] text-[var(--text-3)] truncate">
              {soonest.title}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
