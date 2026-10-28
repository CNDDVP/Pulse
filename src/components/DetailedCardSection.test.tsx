// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DetailedCardSection } from "./DetailedCardSection";
import { formatShortTokens } from "../lib/detailedCard";
import { I18nProvider } from "../lib/i18n";
import type { AccountCardDetail, AppSettings } from "../types";

const mockDetail: AccountCardDetail = {
  account_id: "claude-default",
  window_estimates: {
    five_hour: {
      window_id: "five_hour",
      full_usd: 220,
      used_usd: 55,
      unpriced_tokens: 0,
    },
  },
  activity: {
    is_account_wide: false,
    today_tokens: 245_000,
    today_cost_usd: 1.5,
    today_unpriced: 0,
    week_tokens: 3_400_000,
    week_cost_usd: 22.4,
    week_unpriced: 0,
    month_tokens: 18_500_000,
    month_cost_usd: 120.5,
    month_unpriced: 100,
    days_31: Array.from({ length: 31 }, (_, i) => ({
      day: `2026-10-${String(i + 1).padStart(2, "0")}`,
      tokens: i === 30 ? 245_000 : 100_000,
    })),
    top_model: {
      name: "claude-sonnet-4",
      share_percent: 82,
    },
    cache_hit_rate: 0.91,
  },
  prompt_cache: {
    provider_id: "claude",
    lifetime_seconds: 3600,
    is_minimum: false,
    live_chats: [
      {
        id: "chat-1",
        title: "pulse-windows",
        replied_at: "2026-10-25T10:00:00Z",
        expires_at: "2026-10-25T10:38:00Z",
        lifetime_seconds: 3600,
        is_minimum: false,
      },
    ],
    recently_lapsed: false,
  },
};

const mockSettings = {
  display_currency: "USD",
  usd_cny_rate: 7.2,
} as unknown as AppSettings;

describe("DetailedCardSection", () => {
  it("formats short token numbers in zh (万/亿) and en (K/M/B)", () => {
    expect(formatShortTokens(245_000, "zh")).toBe("24.5万");
    expect(formatShortTokens(120_000_000, "zh")).toBe("1.2亿");
    expect(formatShortTokens(245_000, "en")).toBe("245K");
    expect(formatShortTokens(18_500_000, "en")).toBe("18.5M");
  });

  it("renders 1d/7d/31d summaries, 31-day chart, top model, cache hit rate, and prompt cache countdown", () => {
    render(
      <I18nProvider lang="zh">
        <DetailedCardSection
          detail={mockDetail}
          settings={mockSettings}
          nowMs={Date.parse("2026-10-25T10:00:00Z")}
        />
      </I18nProvider>,
    );

    expect(screen.getByText("本机记录")).toBeTruthy();
    expect(screen.getByText("24.5万")).toBeTruthy();
    expect(screen.getByText("340万")).toBeTruthy();
    expect(screen.getByText("1850万")).toBeTruthy();
    expect(screen.getByText("claude-sonnet-4 · 82%")).toBeTruthy();
    expect(screen.getByText("91%")).toBeTruthy();
    expect(screen.getByText("提示缓存（1 小时）")).toBeTruthy();
    expect(screen.getByText("剩余 38 分钟")).toBeTruthy();
  });
});
