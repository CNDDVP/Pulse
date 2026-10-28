// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CodexResetSection } from "./CodexResetSection";
import {
  buildResetTooltip,
  formatCompactTiming,
  pickPostContent,
} from "../lib/codexReset";
import { I18nProvider } from "../lib/i18n";
import type { CodexResetEvent, CodexResetState } from "../types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const sampleEvent: CodexResetEvent = {
  id: "ev-1",
  kind: "direct_reset",
  status: "announced",
  title: "Codex quota reset",
  scope: "all",
  created_at: "2026-10-25T08:00:00Z",
  updated_at: "2026-10-25T08:05:00Z",
  confirmed_at: null,
  schedule: {
    precision: "window",
    from: "2026-10-25T10:00:00Z",
    through: "2026-10-25T12:00:00Z",
    label: "10:00-12:00 UTC",
  },
  posts: [
    {
      published_at: "2026-10-25T07:55:00Z",
      text: "OpenAI 宣布今日重置 Codex 额度",
      original_text: "OpenAI announced Codex limits will reset today",
      full_original_text: null,
      url: "https://x.com/openai/status/123",
    },
  ],
  url: "https://aihot.news/events/ev-1",
};

describe("CodexResetSection helpers & component", () => {
  it("pickPostContent prefers Chinese translation in zh and original text in en", () => {
    const post = sampleEvent.posts[0];
    expect(pickPostContent(post, "zh")).toEqual({
      text: "OpenAI 宣布今日重置 Codex 额度",
      isTranslation: true,
    });
    expect(pickPostContent(post, "en")).toEqual({
      text: "OpenAI announced Codex limits will reset today",
      isTranslation: false,
    });
  });

  it("pickPostContent falls back to Chinese translation with honest flag when original is missing", () => {
    const post = {
      ...sampleEvent.posts[0],
      original_text: null,
      full_original_text: null,
    };
    expect(pickPostContent(post, "en")).toEqual({
      text: "OpenAI 宣布今日重置 Codex 额度",
      isTranslation: true,
    });
  });

  it("formatCompactTiming formats announced window and confirmed timestamps", () => {
    const nowMs = Date.parse("2026-10-25T09:00:00Z");
    const zhTiming = formatCompactTiming(sampleEvent, nowMs, "zh");
    expect(zhTiming).toContain("预计");
    expect(zhTiming).toContain("UTC");

    const confirmedEv: CodexResetEvent = {
      ...sampleEvent,
      status: "confirmed",
      confirmed_at: "2026-10-25T11:30:00Z",
    };
    const enTiming = formatCompactTiming(confirmedEv, nowMs, "en");
    expect(enTiming).toContain("Confirmation reported");
  });

  it("buildResetTooltip includes confirmation state, check time, and local quota increase note", () => {
    const tip = buildResetTooltip(
      sampleEvent,
      {
        last_succeeded_at: "2026-10-25T09:00:00Z",
        last_attempt_failed: false,
        is_stale: false,
      },
      {
        event_id: "ev-1",
        detected_at: "2026-10-25T09:05:00Z",
      },
      "zh",
    );
    expect(tip).toContain("AI Hot News 尚未确认这条公告");
    expect(tip).toContain("检测到本账号剩余额度回升");
  });

  it("renders announcement section, local quota increase note, and language-ordered links", () => {
    const state: CodexResetState = {
      event: sampleEvent,
      health: {
        last_succeeded_at: "2026-10-25T09:00:00Z",
        last_attempt_failed: false,
        is_stale: true,
      },
      increases: {
        "codex-default": {
          event_id: "ev-1",
          detected_at: "2026-10-25T09:05:00Z",
        },
      },
    };

    const { rerender } = render(
      <I18nProvider lang="zh">
        <CodexResetSection
          resetState={state}
          accountId="codex-default"
          nowMs={Date.parse("2026-10-25T09:10:00Z")}
        />
      </I18nProvider>,
    );

    expect(screen.getByText("额度重置")).toBeTruthy();
    expect(screen.getByText("已预告")).toBeTruthy();
    expect(screen.getByText(/译文：OpenAI 宣布今日重置 Codex 额度/)).toBeTruthy();
    expect(screen.getByText("本账号额度已回升 · 全局重置待确认")).toBeTruthy();
    expect(screen.getByText("公告数据可能已过期")).toBeTruthy();

    const zhButtons = screen.getAllByRole("button");
    expect(zhButtons[0].textContent).toBe("AI Hot News");
    expect(zhButtons[1].textContent).toBe("打开原帖 (X)");

    rerender(
      <I18nProvider lang="en">
        <CodexResetSection
          resetState={state}
          accountId="codex-default"
          nowMs={Date.parse("2026-10-25T09:10:00Z")}
        />
      </I18nProvider>,
    );

    expect(screen.getByText("Limit reset")).toBeTruthy();
    expect(screen.getByText("Announced")).toBeTruthy();
    expect(screen.getByText(/Original: OpenAI announced Codex limits will reset today/)).toBeTruthy();
    const enButtons = screen.getAllByRole("button");
    expect(enButtons[0].textContent).toBe("Open original post (X)");
    expect(enButtons[1].textContent).toBe("AI Hot News");
  });
});
