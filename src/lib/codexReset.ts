import type {
  CodexResetEvent,
  CodexResetHealth,
  CodexResetIncrease,
  CodexResetPost,
} from "../types";
import { translate, type Lang } from "./i18n";

export function shortUtcOffset(date: Date): string {
  const offsetMin = -date.getTimezoneOffset();
  if (offsetMin === 0) return "UTC";
  const sign = offsetMin > 0 ? "+" : "-";
  const abs = Math.abs(offsetMin);
  const hours = Math.floor(abs / 60);
  const mins = abs % 60;
  return mins === 0
    ? `UTC${sign}${hours}`
    : `UTC${sign}${hours}:${String(mins).padStart(2, "0")}`;
}

function isSameLocalDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function formatDatePart(d: Date, lang: Lang): string {
  const locale = lang === "en" ? "en-US" : "zh-CN";
  return d.toLocaleDateString(locale, { month: "short", day: "numeric" });
}

function formatTimePart(d: Date, lang: Lang): string {
  const locale = lang === "en" ? "en-US" : "zh-CN";
  return d.toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function formatDateTime(d: Date, now: Date, lang: Lang): string {
  if (isSameLocalDay(d, now)) {
    return formatTimePart(d, lang);
  }
  return `${formatDatePart(d, lang)} ${formatTimePart(d, lang)}`;
}

export function formatCompactTiming(
  event: CodexResetEvent,
  nowMs: number,
  lang: Lang,
): string {
  const now = new Date(nowMs);
  if (event.status === "confirmed") {
    const raw = event.confirmed_at || event.updated_at;
    const dt = new Date(raw);
    if (Number.isNaN(dt.getTime())) {
      return translate(lang, "codex_reset.status.confirmed");
    }
    const stamp = `${formatDateTime(dt, now, lang)} ${shortUtcOffset(dt)}`;
    return translate(lang, "codex_reset.confirmed_at", { time: stamp });
  }

  const sched = event.schedule;
  if (!sched) {
    return translate(lang, "codex_reset.time_unspecified");
  }
  const from = new Date(sched.from);
  const through = new Date(sched.through);
  if (Number.isNaN(from.getTime()) || Number.isNaN(through.getTime())) {
    return sched.label || translate(lang, "codex_reset.time_unspecified");
  }

  const tz = shortUtcOffset(from);
  switch (sched.precision) {
    case "date":
      return translate(lang, "codex_reset.expected_date", {
        date: formatDatePart(from, lang),
      });
    case "deadline":
      return translate(lang, "codex_reset.expected_by", {
        time: `${formatDateTime(through, now, lang)} ${shortUtcOffset(through)}`,
      });
    case "exact":
      return translate(lang, "codex_reset.expected_at", {
        time: `${formatDateTime(from, now, lang)} ${tz}`,
      });
    case "approximate":
      return translate(lang, "codex_reset.expected_around", {
        time: `${formatDateTime(from, now, lang)} ${tz}`,
      });
    case "window":
    default: {
      const startStr = formatDateTime(from, now, lang);
      const endStr = isSameLocalDay(from, through)
        ? formatTimePart(through, lang)
        : formatDateTime(through, now, lang);
      return translate(lang, "codex_reset.expected_window", {
        from: startStr,
        through: `${endStr} ${shortUtcOffset(through)}`,
      });
    }
  }
}

export function pickPostContent(
  post: CodexResetPost | undefined,
  lang: Lang,
): { text: string; isTranslation: boolean } | null {
  if (!post) return null;
  const zhText = post.text?.trim() || "";
  const origText =
    post.original_text?.trim() || post.full_original_text?.trim() || "";

  if (lang === "zh") {
    if (zhText) return { text: zhText, isTranslation: true };
    if (origText) return { text: origText, isTranslation: false };
    return null;
  }

  if (origText) return { text: origText, isTranslation: false };
  if (zhText) return { text: zhText, isTranslation: true };
  return null;
}

export function buildResetTooltip(
  event: CodexResetEvent,
  health: CodexResetHealth,
  increase: CodexResetIncrease | undefined,
  lang: Lang,
): string {
  const locale = lang === "en" ? "en-US" : "zh-CN";
  const lines: string[] = [];

  lines.push(
    event.status === "confirmed"
      ? translate(lang, "codex_reset.tooltip.confirmed")
      : translate(lang, "codex_reset.tooltip.unconfirmed"),
  );

  if (health.last_succeeded_at) {
    const dt = new Date(health.last_succeeded_at);
    if (!Number.isNaN(dt.getTime())) {
      lines.push(
        translate(lang, "codex_reset.tooltip.last_check", {
          time: dt.toLocaleString(locale),
        }),
      );
    }
  } else {
    lines.push(translate(lang, "codex_reset.tooltip.no_check"));
  }

  if (event.schedule) {
    lines.push(translate(lang, "codex_reset.tooltip.estimate_note"));
  }

  const firstPost = event.posts[0];
  if (firstPost?.published_at) {
    const pdt = new Date(firstPost.published_at);
    if (!Number.isNaN(pdt.getTime())) {
      lines.push(
        translate(lang, "codex_reset.tooltip.posted_at", {
          time: pdt.toLocaleString(locale),
        }),
      );
    }
  }

  const picked = pickPostContent(firstPost, lang);
  if (picked?.isTranslation) {
    lines.push(translate(lang, "codex_reset.tooltip.translation_note"));
  }

  if (increase && increase.event_id === event.id) {
    const idt = new Date(increase.detected_at);
    if (!Number.isNaN(idt.getTime())) {
      lines.push(
        translate(lang, "codex_reset.tooltip.increase_detail", {
          time: idt.toLocaleString(locale),
        }),
      );
    }
  }

  return lines.join("\n");
}
