// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RecapModal } from "./RecapModal";
import {
  buildRecapDeck,
  computePaybackMultiple,
  maskProjects,
} from "../../lib/recapDeck";
import { I18nProvider } from "../../lib/i18n";
import type { AppSettings, RecapReport } from "../../types";

const sampleReport: RecapReport = {
  period: {
    kind: "month",
    key: "2026-10",
    label: "2026-10",
    start: "2026-10-01",
    end: "2026-10-31",
    in_progress: true,
  },
  records_begin: "2026-10-01",
  tokens: 12_500_000,
  previous_tokens: 10_000_000,
  cost_usd: 85.0,
  unpriced_tokens: 0,
  partial: false,
  active_days: 18,
  observed_days: 25,
  total_days: 31,
  daily_average_tokens: 500_000,
  sessions: 64,
  current_streak: 5,
  longest_streak: 9,
  streak_chain: [{ day: "2026-10-25", active: true }],
  busiest_day: { day: "2026-10-15", tokens: 1_800_000 },
  days: [
    {
      day: "2026-10-15",
      tokens: 1_800_000,
      cost_usd: 12.0,
      unrecorded: false,
      future: false,
    },
  ],
  months: [],
  hours: Array.from({ length: 24 }, (_, h) => (h === 15 ? 2_000_000 : 100_000)),
  peak_hour: 15,
  busiest_hours: { start_hour: 14, end_hour: 18, share: 0.48 },
  first_hour: 9,
  latest_minute: 1500, // 01:00 next day
  late_nights: 3,
  persona: "day_shift",
  work_split: { weekday_share: 0.8, weekend_share: 0.2 },
  cache_hit_rate: 0.88,
  cache_savings_usd: 42.5,
  paid_months: 0.8,
  models: [
    {
      model: "claude-sonnet-4",
      tokens: 8_000_000,
      share: 0.64,
      cost_usd: 60.0,
    },
  ],
  agents: [
    {
      source: "claude",
      label: "Claude Code",
      tokens: 8_000_000,
      share: 0.64,
      active_days: ["2026-10-15"],
      active_months: ["2026-10"],
    },
  ],
  projects: [
    {
      name: "secret-internal-repo",
      tokens: 12_500_000,
      share: 1.0,
      sessions: 64,
    },
  ],
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "list_recap_periods") {
      return [sampleReport.period];
    }
    if (cmd === "build_recap") {
      return sampleReport;
    }
    if (cmd === "save_recap_png") {
      return "D:\\exports\\pulse-recap-2026-10-01-poster.png";
    }
    return null;
  }),
}));

describe("RecapDeck & RecapModal", () => {
  it("builds deck with payback card only when monthlyPrice > 0", () => {
    const deckWithoutPrice = buildRecapDeck(sampleReport, 0, "zh");
    expect(deckWithoutPrice.map((c) => c.id)).toEqual([
      "poster",
      "opener",
      "calendar",
      "timetable",
      "scorecard",
    ]);

    const deckWithPrice = buildRecapDeck(sampleReport, 20, "zh");
    expect(deckWithPrice.map((c) => c.id)).toEqual([
      "poster",
      "opener",
      "calendar",
      "timetable",
      "payback",
      "scorecard",
    ]);
  });

  it("masks project names when privacy switch is enabled", () => {
    const unmasked = maskProjects(sampleReport.projects, false, "zh");
    expect(unmasked[0].name).toBe("secret-internal-repo");

    const maskedZh = maskProjects(sampleReport.projects, true, "zh");
    expect(maskedZh[0].name).toBe("项目 1");

    const maskedEn = maskProjects(sampleReport.projects, true, "en");
    expect(maskedEn[0].name).toBe("Project 1");
  });

  it("computes prorated payback multiple", () => {
    const res = computePaybackMultiple(sampleReport, 20);
    expect(res).not.toBeNull();
    // paidUsd = 20 * 0.8 = 16; multiple = 85 / 16 = 5.3125
    expect(res!.paidUsd).toBeCloseTo(16.0);
    expect(res!.multiple).toBeCloseTo(5.3125);
  });

  it("renders RecapModal, navigates cards, and masks projects on toggle", async () => {
    const settings = {
      display_currency: "USD",
      usd_cny_rate: 7.2,
      recap_monthly_price: 20,
      recap_hide_projects: false,
    } as unknown as AppSettings;

    render(
      <I18nProvider lang="zh">
        <RecapModal open={true} onClose={() => {}} settings={settings} />
      </I18nProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("recap-card-poster")).toBeTruthy();
    });

    expect(screen.getByText("secret-internal-repo")).toBeTruthy();

    // Toggle hide project names
    const toggle = screen.getByRole("switch", { name: "隐藏项目名称" });
    fireEvent.click(toggle);
    expect(screen.getByText("项目 1")).toBeTruthy();

    // Navigate to next card (opener)
    fireEvent.click(screen.getByLabelText("Next card"));
    expect(screen.getByTestId("recap-card-opener")).toBeTruthy();
  });
});
