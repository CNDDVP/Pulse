import { invoke } from "@tauri-apps/api/core";
import type { CodexResetState } from "../types";
import { useLang } from "../lib/i18n";
import {
  buildResetTooltip,
  formatCompactTiming,
  pickPostContent,
} from "../lib/codexReset";

export function CodexResetSection({
  resetState,
  accountId,
  nowMs,
}: {
  resetState: CodexResetState;
  accountId: string;
  nowMs: number;
}) {
  const { t, lang } = useLang();
  const event = resetState.event;
  if (!event) return null;

  const isConfirmed = event.status === "confirmed";
  const accentColor = isConfirmed ? "text-[var(--ok)]" : "text-[#f97316]";
  const kindTitle =
    event.kind === "reset_credit"
      ? t("codex_reset.kind.reset_credit")
      : t("codex_reset.kind.direct_reset");
  const statusLabel = isConfirmed
    ? t("codex_reset.status.confirmed")
    : t("codex_reset.status.announced");

  const timingLine = formatCompactTiming(event, nowMs, lang);
  const firstPost = event.posts[0];
  const picked = pickPostContent(firstPost, lang);
  const excerptLine = picked
    ? picked.isTranslation
      ? t("codex_reset.excerpt.translation", { text: picked.text })
      : t("codex_reset.excerpt.original", { text: picked.text })
    : null;

  const baseAccountId = accountId.split("::")[0];
  const increase = resetState.increases[baseAccountId];
  const hasLocalIncrease =
    !isConfirmed &&
    event.kind === "direct_reset" &&
    increase &&
    increase.event_id === event.id;

  const tooltip = buildResetTooltip(
    event,
    resetState.health,
    hasLocalIncrease ? increase : undefined,
    lang,
  );

  const openUrl = (url: string) => {
    if (!url || (!url.startsWith("https://") && !url.startsWith("http://"))) {
      return;
    }
    void invoke("open_external_url", { url }).catch(() => {});
  };

  const aihotLink = {
    label: t("codex_reset.link.aihot"),
    url: event.url,
  };
  const origLink = firstPost?.url
    ? {
        label: `${t("codex_reset.link.original")} (X)`,
        url: firstPost.url,
      }
    : null;

  // Chinese puts AI Hot News first; English puts original X post first.
  const links =
    lang === "zh"
      ? [aihotLink, ...(origLink ? [origLink] : [])]
      : [...(origLink ? [origLink] : []), aihotLink];

  return (
    <div
      className="mt-3 pt-2.5 border-t border-[var(--border)] space-y-1.5"
      title={tooltip}
      aria-label={t("codex_reset.section_title")}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <span className={`${accentColor} font-semibold text-xs shrink-0`} aria-hidden>
            {event.kind === "reset_credit" ? "🎫" : "↻"}
          </span>
          <span className="font-semibold text-xs text-[var(--text-1)] truncate">
            {kindTitle}
          </span>
        </div>
        <span className={`text-[10.5px] font-medium shrink-0 ${accentColor}`}>
          {statusLabel}
        </span>
      </div>

      <p className={`text-[11.5px] font-medium font-mono leading-snug ${accentColor}`}>
        {timingLine}
      </p>

      {excerptLine && (
        <p className="text-[11px] text-[var(--text-2)] leading-relaxed line-clamp-3 break-words">
          {excerptLine}
        </p>
      )}

      {hasLocalIncrease && (
        <p className="text-[10.5px] text-[var(--ok)] font-medium">
          {t("codex_reset.local_increase")}
        </p>
      )}

      {resetState.health.is_stale && (
        <p className="text-[10.5px] text-[var(--text-3)]">
          {t("codex_reset.stale_warn")}
        </p>
      )}

      <div className="flex items-center gap-2.5 pt-0.5 text-[10.5px] text-[var(--text-3)]">
        <span aria-hidden>🔗</span>
        {links.map((lnk) => (
          <button
            key={lnk.url}
            type="button"
            onClick={() => openUrl(lnk.url)}
            className="text-[var(--text-2)] hover:text-[var(--text-1)] hover:underline cursor-pointer transition-colors"
          >
            {lnk.label}
          </button>
        ))}
      </div>
    </div>
  );
}
