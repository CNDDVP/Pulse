import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CodexSignalsReport } from "../types";
import { useLang } from "../lib/i18n";
import { Section } from "../pages/settings/shared";
import { btnGhost } from "../pages/settings/constants";

type SpanOption = 30 | 90 | null;

export function CodexSignalsPanel() {
  const { t, lang } = useLang();
  const [span, setSpan] = useState<SpanOption>(30);
  const [report, setReport] = useState<CodexSignalsReport | null>(null);
  const [loading, setLoading] = useState(true);

  const loadSignals = useCallback(async (targetSpan: SpanOption) => {
    try {
      const res = await invoke<CodexSignalsReport>("get_codex_signals", {
        spanDays: targetSpan,
      });
      setReport(res);
    } catch {
      // Keep last report or empty state if scan fails
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadSignals(span);
  }, [span, loadSignals]);

  const handleSelectSpan = (nextSpan: SpanOption) => {
    if (nextSpan !== span) {
      setLoading(true);
      setSpan(nextSpan);
    }
  };

  const handleRescan = () => {
    setLoading(true);
    void loadSignals(span);
  };

  const dateLocale = lang === "en" ? "en-US" : "zh-CN";
  const spans: { value: SpanOption; labelKey: string }[] = [
    { value: 30, labelKey: "codex_signals.span.30" },
    { value: 90, labelKey: "codex_signals.span.90" },
    { value: null, labelKey: "codex_signals.span.all" },
  ];

  const extraChangesCount =
    report && report.total_changes > report.changes.length
      ? report.total_changes - report.changes.length
      : 0;

  return (
    <Section
      title={t("codex_signals.title")}
      icon="🔍"
      subtitle={t("codex_signals.subtitle")}
      aside={
        <div className="flex items-center gap-2">
          <div className="inline-flex rounded-xl bg-[var(--surface)] p-0.5 border border-[var(--border)]">
            {spans.map((opt) => {
              const active = span === opt.value;
              return (
                <button
                  key={String(opt.value)}
                  type="button"
                  onClick={() => handleSelectSpan(opt.value)}
                  className={`px-2.5 py-1 rounded-lg text-[11px] font-medium transition-colors cursor-pointer ${
                    active
                      ? "bg-[var(--surface-3)] text-[var(--text-1)]"
                      : "text-[var(--text-3)] hover:text-[var(--text-1)]"
                  }`}
                >
                  {t(opt.labelKey)}
                </button>
              );
            })}
          </div>
          <button
            type="button"
            disabled={loading}
            onClick={handleRescan}
            className={btnGhost}
          >
            {loading ? t("codex_signals.scanning") : t("codex_signals.scan_btn")}
          </button>
        </div>
      }
    >
      <div className="space-y-4">
        <div className="p-3.5 rounded-xl bg-[var(--surface)] border border-[var(--border)] space-y-2.5">
          <div className="text-xs font-semibold text-[var(--text-1)]">
            {t("codex_signals.lattice_title")}
          </div>
          {!report || report.models.length === 0 ? (
            <p className="text-xs text-[var(--text-3)]">
              {t("codex_signals.no_models")}
            </p>
          ) : (
            <div className="divide-y divide-[var(--border)]">
              {report.models.map((m) => (
                <div
                  key={m.model}
                  className="py-2 first:pt-0 last:pb-0 flex items-center justify-between gap-3"
                >
                  <div className="min-w-0">
                    <div className="text-xs font-medium text-[var(--text-1)] font-mono truncate">
                      {m.model}
                    </div>
                    <div className="text-[11px] text-[var(--text-3)]">
                      {t("codex_signals.lattice_sub", {
                        on_lattice: m.on_lattice,
                        reached: m.reached_lattice,
                        total: m.responses,
                      })}
                    </div>
                  </div>
                  <div className="shrink-0 text-right font-mono text-xs font-semibold text-[var(--text-1)]">
                    {m.insufficient_data || m.share == null ? (
                      <span className="text-[11px] font-normal text-[var(--text-3)]">
                        {t("codex_signals.too_few")}
                      </span>
                    ) : (
                      `${(m.share * 100).toFixed(1)}%`
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="p-3.5 rounded-xl bg-[var(--surface)] border border-[var(--border)] space-y-2.5">
          <div>
            <div className="text-xs font-semibold text-[var(--text-1)]">
              {t("codex_signals.lowered_title")}
            </div>
            {report && (
              <div className="text-[11px] text-[var(--text-3)] mt-0.5">
                {t("codex_signals.lowered_sub", {
                  judged: report.judged_sessions,
                  unjudged: report.unjudged_sessions,
                })}
              </div>
            )}
          </div>
          {!report || report.changes.length === 0 ? (
            <p className="text-xs text-[var(--text-2)]">
              {t("codex_signals.lowered_none")}
            </p>
          ) : (
            <div className="space-y-1.5">
              {report.changes.map((ch, idx) => {
                const dt = new Date(ch.timestamp);
                const timeStr = Number.isNaN(dt.getTime())
                  ? ch.timestamp
                  : dt.toLocaleString(dateLocale, {
                      month: "short",
                      day: "numeric",
                      hour: "2-digit",
                      minute: "2-digit",
                    });
                return (
                  <div
                    key={`${ch.timestamp}-${idx}`}
                    className="flex items-center justify-between gap-2 text-xs py-1 border-b border-[var(--border)] last:border-b-0"
                  >
                    <span className="text-[var(--warn)] font-medium truncate">
                      {ch.kind === "model"
                        ? t("codex_signals.change.model", {
                            asked: ch.asked,
                            ran: ch.ran,
                          })
                        : t("codex_signals.change.effort", {
                            asked: ch.asked,
                            ran: ch.ran,
                          })}
                    </span>
                    <span className="text-[11px] text-[var(--text-3)] shrink-0 font-mono">
                      {timeStr}
                    </span>
                  </div>
                );
              })}
              {extraChangesCount > 0 && (
                <p className="text-[11px] text-[var(--text-3)] pt-1">
                  {t("codex_signals.more_changes", { count: extraChangesCount })}
                </p>
              )}
            </div>
          )}
        </div>

        <p className="text-[11px] text-[var(--text-3)] leading-relaxed">
          {t("codex_signals.footnote")}
        </p>
      </div>
    </Section>
  );
}
