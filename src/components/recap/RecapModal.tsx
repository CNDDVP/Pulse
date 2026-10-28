import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  RecapPeriodInfo,
  RecapPeriodKind,
  RecapReport,
} from "../../types";
import { useLang } from "../../lib/i18n";
import {
  buildRecapDeck,
  computePaybackMultiple,
  formatLatestMinute,
  maskProjects,
  renderRecapCardToCanvas,
  type RecapCardEntry,
} from "../../lib/recapDeck";
import {
  formatShortTokens,
  formatUsdEstimateForCard,
} from "../../lib/detailedCard";
import { Switch } from "../../pages/settings/shared";
import { btnGhost, btnPrimary, inputCls, selectCls } from "../../pages/settings/constants";

function EditorialCardView({
  card,
  report,
  monthlyPriceUsd,
  hideProjects,
  settings,
}: {
  card: RecapCardEntry;
  report: RecapReport;
  monthlyPriceUsd: number;
  hideProjects: boolean;
  settings: AppSettings;
}) {
  const { t, lang } = useLang();
  const projects = useMemo(
    () => maskProjects(report.projects, hideProjects, lang),
    [report.projects, hideProjects, lang],
  );
  const payback = useMemo(
    () => computePaybackMultiple(report, monthlyPriceUsd),
    [report, monthlyPriceUsd],
  );
  const moneyStr = formatUsdEstimateForCard(
    report.cost_usd,
    settings.display_currency,
    settings.usd_cny_rate,
  );

  const deltaPercent =
    report.previous_tokens && report.previous_tokens > 0
      ? Math.round(
          ((report.tokens - report.previous_tokens) / report.previous_tokens) *
            100,
        )
      : null;

  const peakDayTokens = Math.max(1, ...report.days.map((d) => d.tokens));
  const peakHourTokens = Math.max(1, ...report.hours);

  return (
    <div
      data-testid={`recap-card-${card.id}`}
      className="w-[405px] h-[720px] rounded-xl p-5 flex flex-col justify-between select-none border-2"
      style={{
        backgroundColor: "#F5F5F1",
        color: "#1B1B1E",
        borderColor: "#D8D8D0",
      }}
    >
      <div className="space-y-3">
        <div className="flex items-center justify-between border-b border-[#D8D8D0] pb-2">
          <span className="text-[11px] font-bold tracking-wider uppercase">
            PULSE RECAP · {report.period.label}
          </span>
          <span className="text-[11px] font-mono font-semibold text-[#6B6B65]">
            {card.numberLabel}
          </span>
        </div>

        <div
          className="p-3.5 border-2 border-[#1B1B1E] flex items-baseline justify-between"
          style={{ backgroundColor: "#C8F03C" }}
        >
          <div>
            <div className="text-[10px] font-bold uppercase tracking-wider text-[#1B1B1E]/75">
              TOTAL TOKENS · {card.title}
            </div>
            <div className="text-2xl font-black font-mono tracking-tight text-[#1B1B1E]">
              {formatShortTokens(report.tokens, lang)}
            </div>
          </div>
          {deltaPercent != null && (
            <span className="px-2 py-0.5 text-[11px] font-mono font-bold bg-[#1B1B1E] text-[#F5F5F1] rounded">
              {deltaPercent >= 0 ? `+${deltaPercent}%` : `${deltaPercent}%`}
            </span>
          )}
        </div>

        <div className="bg-white border border-[#E2E2DC] p-3.5 space-y-3">
          {card.id === "opener" && (
            <div className="space-y-2.5">
              <div className="text-xs font-bold uppercase tracking-wide text-[#6B6B65]">
                {t("recap.card.opener")} ({report.agents.length})
              </div>
              {report.agents.slice(0, 4).map((ag, idx) => (
                <div
                  key={ag.source}
                  className={`p-2.5 rounded border ${
                    idx === 0
                      ? "bg-[#1B1B1E] text-[#F5F5F1] border-[#1B1B1E]"
                      : "bg-[#F5F5F1] text-[#1B1B1E] border-[#E2E2DC]"
                  } flex items-center justify-between`}
                >
                  <div>
                    <div className="text-xs font-bold">{ag.label}</div>
                    <div className="text-[10px] opacity-75">
                      {ag.active_days.length} active days
                    </div>
                  </div>
                  <div className="text-right font-mono">
                    <div className="text-xs font-bold">
                      {formatShortTokens(ag.tokens, lang)}
                    </div>
                    <div className="text-[10px] opacity-75">
                      {Math.round(ag.share * 100)}%
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {(card.id === "calendar" || card.id === "year_calendar") && (
            <div className="space-y-2.5">
              <div className="flex justify-between text-xs font-bold">
                <span>
                  {report.active_days} / {report.observed_days} Days Active
                </span>
                <span className="font-mono">
                  Streak {report.current_streak}d (Max {report.longest_streak}d)
                </span>
              </div>
              <div className="grid grid-cols-7 gap-1">
                {report.days.slice(0, 35).map((d) => {
                  const isBusy =
                    report.busiest_day?.day === d.day && d.tokens > 0;
                  return (
                    <div
                      key={d.day}
                      title={`${d.day}: ${d.tokens.toLocaleString()}`}
                      className={`h-6 rounded text-[9px] font-mono flex items-center justify-center border ${
                        d.future || d.unrecorded
                          ? "border-dashed border-[#D8D8D0] text-[#9E9E95]"
                          : isBusy
                          ? "bg-[#C8F03C] border-[#1B1B1E] font-bold text-[#1B1B1E]"
                          : d.tokens > 0
                          ? "bg-[#1B1B1E] border-[#1B1B1E] text-[#F5F5F1]"
                          : "bg-[#F5F5F1] border-[#E2E2DC] text-[#6B6B65]"
                      }`}
                    >
                      {d.day.slice(-2)}
                    </div>
                  );
                })}
              </div>
              {report.work_split && (
                <div className="flex justify-between text-[11px] font-mono pt-1 text-[#6B6B65]">
                  <span>
                    Weekday {Math.round(report.work_split.weekday_share * 100)}%
                  </span>
                  <span>
                    Weekend {Math.round(report.work_split.weekend_share * 100)}%
                  </span>
                </div>
              )}
            </div>
          )}

          {card.id === "months" && (
            <div className="space-y-1.5">
              {report.months.map((m) => {
                const maxM = Math.max(1, ...report.months.map((x) => x.tokens));
                const pct = Math.round((m.tokens / maxM) * 100);
                return (
                  <div
                    key={m.month}
                    className="flex items-center gap-2 text-[11px] font-mono"
                  >
                    <span className="w-10 text-[#6B6B65]">{m.label}</span>
                    <div className="flex-1 h-3 bg-[#F5F5F1] border border-[#E2E2DC] overflow-hidden">
                      <div
                        className="h-full bg-[#1B1B1E]"
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                    <span className="w-14 text-right font-bold">
                      {formatShortTokens(m.tokens, lang)}
                    </span>
                  </div>
                );
              })}
            </div>
          )}

          {card.id === "timetable" && (
            <div className="space-y-2.5">
              <div className="grid grid-cols-3 gap-2 text-center">
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">Peak Hour</div>
                  <div className="text-xs font-bold font-mono">
                    {report.peak_hour != null
                      ? `${String(report.peak_hour).padStart(2, "0")}:00`
                      : "—"}
                  </div>
                </div>
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">First Start</div>
                  <div className="text-xs font-bold font-mono">
                    {report.first_hour != null
                      ? `${String(report.first_hour).padStart(2, "0")}:00`
                      : "—"}
                  </div>
                </div>
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">Latest End</div>
                  <div className="text-xs font-bold font-mono">
                    {formatLatestMinute(report.latest_minute)}
                  </div>
                </div>
              </div>
              <div className="h-24 flex items-end gap-[2px] pt-2">
                {report.hours.map((hTok, hr) => {
                  const hPx =
                    hTok > 0
                      ? Math.max(4, Math.round((hTok / peakHourTokens) * 80))
                      : 2;
                  const isPeak = report.peak_hour === hr && hTok > 0;
                  return (
                    <div
                      key={hr}
                      className="flex-1 flex flex-col items-center"
                      title={`${hr}:00 - ${hTok.toLocaleString()}`}
                    >
                      <div
                        className="w-full"
                        style={{
                          height: `${hPx}px`,
                          backgroundColor: isPeak ? "#C8F03C" : "#1B1B1E",
                          border: isPeak ? "1px solid #1B1B1E" : "none",
                        }}
                      />
                    </div>
                  );
                })}
              </div>
              {report.busiest_hours && (
                <div className="text-[11px] font-mono text-[#6B6B65]">
                  Busiest 4h window:{" "}
                  {String(report.busiest_hours.start_hour).padStart(2, "0")}:00–
                  {String(report.busiest_hours.end_hour).padStart(2, "0")}:00 (
                  {Math.round(report.busiest_hours.share * 100)}%)
                </div>
              )}
            </div>
          )}

          {card.id === "payback" && payback && (
            <div className="space-y-3">
              <div className="p-3 bg-[#1B1B1E] text-[#F5F5F1] flex items-baseline justify-between">
                <div>
                  <div className="text-[10px] uppercase opacity-75">
                    Subscription Payback Multiple
                  </div>
                  <div
                    className="text-2xl font-black font-mono"
                    style={{ color: "#C8F03C" }}
                  >
                    {payback.multiple.toFixed(1)}×
                  </div>
                </div>
                <div className="text-right font-mono text-xs">
                  <div>API Value: {moneyStr}</div>
                  <div className="opacity-75">
                    Paid: ${payback.paidUsd.toFixed(2)}
                  </div>
                </div>
              </div>
              {report.cache_savings_usd >= 0.5 && (
                <div className="p-2.5 bg-[#F5F5F1] border border-[#E2E2DC] flex justify-between text-xs font-mono">
                  <span>Prompt Cache Savings</span>
                  <span className="font-bold">
                    {formatUsdEstimateForCard(
                      report.cache_savings_usd,
                      settings.display_currency,
                      settings.usd_cny_rate,
                    )}
                  </span>
                </div>
              )}
              <div className="space-y-1">
                {report.models.slice(0, 4).map((m) => (
                  <div
                    key={m.model}
                    className="flex justify-between text-[11px] font-mono"
                  >
                    <span className="truncate max-w-[200px]">{m.model}</span>
                    <span className="font-bold">
                      {formatUsdEstimateForCard(
                        m.cost_usd,
                        settings.display_currency,
                        settings.usd_cny_rate,
                      )}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {(card.id === "poster" || card.id === "scorecard") && (
            <div className="space-y-2.5">
              <div className="grid grid-cols-2 gap-2">
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">
                    API Value Estimate
                  </div>
                  <div className="text-sm font-bold font-mono">
                    {moneyStr}
                    {report.unpriced_tokens > 0 ? "*" : ""}
                  </div>
                </div>
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">Active Days</div>
                  <div className="text-sm font-bold font-mono">
                    {report.active_days} / {report.observed_days}
                  </div>
                </div>
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">
                    Daily Average
                  </div>
                  <div className="text-sm font-bold font-mono">
                    {formatShortTokens(report.daily_average_tokens, lang)}
                  </div>
                </div>
                <div className="p-2 bg-[#F5F5F1] border border-[#E2E2DC]">
                  <div className="text-[10px] text-[#6B6B65]">
                    Cache Hit Rate
                  </div>
                  <div className="text-sm font-bold font-mono">
                    {report.cache_hit_rate != null
                      ? `${Math.round(report.cache_hit_rate * 100)}%`
                      : "—"}
                  </div>
                </div>
              </div>

              {report.period.kind !== "day" && (
                <div className="h-12 flex items-end gap-[2px] pt-1">
                  {report.days.slice(-31).map((d) => {
                    const hPx =
                      d.tokens > 0
                        ? Math.max(3, Math.round((d.tokens / peakDayTokens) * 44))
                        : 2;
                    return (
                      <div
                        key={d.day}
                        className="flex-1"
                        style={{
                          height: `${hPx}px`,
                          backgroundColor: d.tokens > 0 ? "#1B1B1E" : "#D8D8D0",
                        }}
                      />
                    );
                  })}
                </div>
              )}

              <div className="space-y-1 pt-1 border-t border-[#E2E2DC]">
                <div className="text-[10px] font-bold uppercase text-[#6B6B65]">
                  Top Models & Projects
                </div>
                {report.models.slice(0, 2).map((m) => (
                  <div
                    key={m.model}
                    className="flex justify-between text-[11px] font-mono"
                  >
                    <span className="truncate max-w-[230px]">{m.model}</span>
                    <span className="font-bold">
                      {Math.round(m.share * 100)}%
                    </span>
                  </div>
                ))}
                {projects.slice(0, 2).map((p) => (
                  <div
                    key={p.name}
                    className="flex justify-between text-[11px] font-mono text-[#4A4A45]"
                  >
                    <span className="truncate max-w-[230px]">{p.name}</span>
                    <span>{formatShortTokens(p.tokens, lang)}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center justify-between pt-2 border-t border-[#D8D8D0] text-[11px]">
        <span className="px-2 py-0.5 rounded bg-[#1B1B1E] text-[#F5F5F1] font-semibold">
          {t(`recap.persona.${report.persona}`)}
        </span>
        <span className="font-mono text-[#6B6B65]">
          {report.sessions} sessions
        </span>
      </div>
    </div>
  );
}

export function RecapModal({
  open,
  onClose,
  settings,
  onUpdateSettings,
}: {
  open: boolean;
  onClose: () => void;
  settings: AppSettings;
  onUpdateSettings?: (patch: Partial<AppSettings>) => void;
}) {
  const { t, lang } = useLang();
  const [kind, setKind] = useState<RecapPeriodKind>("month");
  const [periods, setPeriods] = useState<RecapPeriodInfo[]>([]);
  const [selectedKey, setSelectedKey] = useState<string>("");
  const [report, setReport] = useState<RecapReport | null>(null);
  const [cardIndex, setCardIndex] = useState(0);
  const [monthlyPriceInput, setMonthlyPriceInput] = useState<string>(() =>
    settings.recap_monthly_price && settings.recap_monthly_price > 0
      ? String(settings.recap_monthly_price)
      : "",
  );
  const [hideProjects, setHideProjects] = useState<boolean>(
    !!settings.recap_hide_projects,
  );
  const [statusNote, setStatusNote] = useState<string | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const monthlyPriceUsd = useMemo(() => {
    const parsed = Number(monthlyPriceInput.replace(/,/g, "").trim());
    return Number.isFinite(parsed) && parsed > 0 && parsed <= 10000 ? parsed : 0;
  }, [monthlyPriceInput]);

  const deck = useMemo(
    () => buildRecapDeck(report, monthlyPriceUsd, lang),
    [report, monthlyPriceUsd, lang],
  );

  const safeIndex = deck.length > 0 ? Math.min(cardIndex, deck.length - 1) : 0;
  const activeCard = deck[safeIndex] ?? null;

  const loadPeriodKind = useCallback(
    async (targetKind: RecapPeriodKind, explicitKey?: string) => {
      try {
        const list = await invoke<RecapPeriodInfo[]>("list_recap_periods", {
          kind: targetKind,
        });
        setPeriods(list);
        const rep = await invoke<RecapReport>("build_recap", {
          kind: targetKind,
          key: explicitKey ?? null,
        });
        setReport(rep);
        setSelectedKey(rep.period.key);
        setCardIndex(0);
      } catch (e) {
        setStatusNote(String(e));
      }
    },
    [],
  );

  useEffect(() => {
    if (!open) return;
    let alive = true;
    void invoke<RecapPeriodInfo[]>("list_recap_periods", { kind })
      .then(async (list) => {
        if (!alive) return;
        setPeriods(list);
        const rep = await invoke<RecapReport>("build_recap", {
          kind,
          key: null,
        });
        if (!alive) return;
        setReport(rep);
        setSelectedKey(rep.period.key);
        setCardIndex(0);
      })
      .catch((e) => {
        if (alive) setStatusNote(String(e));
      });
    return () => {
      alive = false;
    };
  }, [open, kind]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) {
        return;
      }
      if (e.key === "ArrowLeft") {
        setCardIndex((i) => Math.max(0, i - 1));
      } else if (e.key === "ArrowRight") {
        setCardIndex((i) => Math.min(Math.max(0, deck.length - 1), i + 1));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, deck.length]);

  if (!open) return null;

  const handleToggleHideProjects = (val: boolean) => {
    setHideProjects(val);
    onUpdateSettings?.({ recap_hide_projects: val });
  };

  const handleCommitPrice = () => {
    onUpdateSettings?.({ recap_monthly_price: monthlyPriceUsd });
  };

  const renderCardPngBase64 = (card: RecapCardEntry): string | null => {
    if (!report) return null;
    const canvas = canvasRef.current ?? document.createElement("canvas");
    renderRecapCardToCanvas(canvas, card, report, {
      monthlyPriceUsd,
      hideProjects,
      displayCurrency: settings.display_currency,
      usdCnyRate: settings.usd_cny_rate,
      lang,
    });
    return canvas.toDataURL("image/png");
  };

  const handleSaveCurrent = async () => {
    if (!activeCard || !report) return;
    const dataUrl = renderCardPngBase64(activeCard);
    if (!dataUrl) return;
    try {
      const idxStr = String(safeIndex + 1).padStart(2, "0");
      const fname = `pulse-recap-${report.period.key}-${idxStr}-${activeCard.id}.png`;
      const savedPath = await invoke<string>("save_recap_png", {
        filename: fname,
        pngBase64: dataUrl,
      });
      setStatusNote(t("recap.saved_to", { path: savedPath }));
    } catch (e) {
      setStatusNote(String(e));
    }
  };

  const handleSaveAll = async () => {
    if (!report || deck.length === 0) return;
    try {
      let lastPath = "";
      for (let i = 0; i < deck.length; i++) {
        const c = deck[i];
        const dataUrl = renderCardPngBase64(c);
        if (!dataUrl) continue;
        const idxStr = String(i + 1).padStart(2, "0");
        const fname = `pulse-recap-${report.period.key}-${idxStr}-${c.id}.png`;
        lastPath = await invoke<string>("save_recap_png", {
          filename: fname,
          pngBase64: dataUrl,
        });
      }
      if (lastPath) {
        setStatusNote(t("recap.saved_to", { path: lastPath }));
      }
    } catch (e) {
      setStatusNote(String(e));
    }
  };

  const handleCopyImage = async () => {
    if (!activeCard || !report) return;
    const canvas = canvasRef.current ?? document.createElement("canvas");
    renderRecapCardToCanvas(canvas, activeCard, report, {
      monthlyPriceUsd,
      hideProjects,
      displayCurrency: settings.display_currency,
      usdCnyRate: settings.usd_cny_rate,
      lang,
    });
    canvas.toBlob(async (blob) => {
      if (!blob) return;
      try {
        if (typeof ClipboardItem !== "undefined" && navigator.clipboard?.write) {
          await navigator.clipboard.write([
            new ClipboardItem({ "image/png": blob }),
          ]);
          setStatusNote(t("recap.copied"));
        }
      } catch (e) {
        setStatusNote(String(e));
      }
    }, "image/png");
  };

  const kinds: { id: RecapPeriodKind; label: string }[] = [
    { id: "day", label: t("recap.kind.day") },
    { id: "week", label: t("recap.kind.week") },
    { id: "month", label: t("recap.kind.month") },
    { id: "year", label: t("recap.kind.year") },
  ];

  return (
    <div
      className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <canvas ref={canvasRef} className="hidden" />
      <div
        className="bg-[var(--surface-2)] border border-[var(--border-strong)] rounded-2xl shadow-2xl w-full max-w-4xl h-[820px] max-h-[94vh] flex flex-col overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-5 py-3.5 border-b border-[var(--border)] flex items-center justify-between">
          <h3 className="text-sm font-bold text-[var(--text-1)]">
            {t("recap.modal_title")}
          </h3>
          <button
            type="button"
            onClick={onClose}
            className="text-xs text-[var(--text-2)] hover:text-[var(--text-1)] cursor-pointer px-2 py-1"
          >
            ✕
          </button>
        </div>

        <div className="flex-1 min-h-0 flex flex-col md:flex-row">
          {/* Left Stage: 1080x1920 Live Preview */}
          <div className="flex-1 bg-[var(--surface)] flex flex-col items-center justify-center p-4 overflow-y-auto">
            {!report || deck.length === 0 || !activeCard ? (
              <p className="text-xs text-[var(--text-3)] text-center max-w-sm">
                {t("recap.empty")}
              </p>
            ) : (
              <>
                <EditorialCardView
                  card={activeCard}
                  report={report}
                  monthlyPriceUsd={monthlyPriceUsd}
                  hideProjects={hideProjects}
                  settings={settings}
                />
                <div className="flex items-center gap-3 mt-3">
                  <button
                    type="button"
                    disabled={safeIndex <= 0}
                    onClick={() => setCardIndex((i) => Math.max(0, i - 1))}
                    className={`${btnGhost} px-2.5 py-1`}
                    aria-label="Previous card"
                  >
                    ◀
                  </button>
                  <div className="flex items-center gap-1.5">
                    {deck.map((c, idx) => (
                      <button
                        key={c.id}
                        type="button"
                        onClick={() => setCardIndex(idx)}
                        title={c.title}
                        className={`w-2 h-2 rounded-full transition-all cursor-pointer ${
                          idx === safeIndex
                            ? "bg-[var(--ok)] w-4"
                            : "bg-[var(--border-strong)]"
                        }`}
                      />
                    ))}
                  </div>
                  <button
                    type="button"
                    disabled={safeIndex >= deck.length - 1}
                    onClick={() =>
                      setCardIndex((i) => Math.min(deck.length - 1, i + 1))
                    }
                    className={`${btnGhost} px-2.5 py-1`}
                    aria-label="Next card"
                  >
                    ▶
                  </button>
                </div>
              </>
            )}
          </div>

          {/* Right Sidebar: Controls & Export (280px) */}
          <div className="w-full md:w-72 shrink-0 border-t md:border-t-0 md:border-l border-[var(--border)] p-4 flex flex-col justify-between space-y-4 overflow-y-auto">
            <div className="space-y-4">
              <div className="grid grid-cols-4 gap-1 p-1 rounded-xl bg-[var(--surface)] border border-[var(--border)]">
                {kinds.map((k) => (
                  <button
                    key={k.id}
                    type="button"
                    onClick={() => setKind(k.id)}
                    className={`py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-colors ${
                      kind === k.id
                        ? "bg-[var(--surface-3)] text-[var(--text-1)] font-semibold"
                        : "text-[var(--text-3)] hover:text-[var(--text-1)]"
                    }`}
                  >
                    {k.label}
                  </button>
                ))}
              </div>

              {periods.length > 0 && (
                <select
                  className={selectCls}
                  value={selectedKey}
                  onChange={(e) => {
                    const nextKey = e.target.value;
                    setSelectedKey(nextKey);
                    void loadPeriodKind(kind, nextKey);
                  }}
                >
                  {periods.map((p) => (
                    <option key={p.key} value={p.key}>
                      {p.label}
                    </option>
                  ))}
                </select>
              )}

              <div className="space-y-1">
                <label className="block text-xs font-medium text-[var(--text-2)]">
                  {t("recap.monthly_price")}
                </label>
                <input
                  type="number"
                  min={0}
                  max={10000}
                  step="any"
                  className={inputCls}
                  placeholder={t("recap.monthly_price_ph")}
                  value={monthlyPriceInput}
                  onChange={(e) => setMonthlyPriceInput(e.target.value)}
                  onBlur={handleCommitPrice}
                />
              </div>

              <div className="p-3 rounded-xl bg-[var(--surface)] border border-[var(--border)] flex items-center justify-between gap-2">
                <div>
                  <div className="text-xs font-medium text-[var(--text-1)]">
                    {t("recap.hide_projects")}
                  </div>
                  <div className="text-[11px] text-[var(--text-3)]">
                    {t("recap.hide_projects_sub")}
                  </div>
                </div>
                <Switch
                  checked={hideProjects}
                  onChange={handleToggleHideProjects}
                  label={t("recap.hide_projects")}
                />
              </div>
            </div>

            <div className="space-y-2 pt-3 border-t border-[var(--border)]">
              {statusNote && (
                <p className="text-[11px] text-[var(--ok)] break-all">
                  {statusNote}
                </p>
              )}
              <div className="grid grid-cols-1 gap-2">
                <button
                  type="button"
                  disabled={!activeCard}
                  onClick={() => void handleSaveCurrent()}
                  className={btnPrimary}
                >
                  {t("recap.save_image")}
                </button>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    disabled={deck.length === 0}
                    onClick={() => void handleSaveAll()}
                    className={btnGhost}
                  >
                    {t("recap.save_all")}
                  </button>
                  <button
                    type="button"
                    disabled={!activeCard}
                    onClick={() => void handleCopyImage()}
                    className={btnGhost}
                  >
                    {t("recap.copy_image")}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
