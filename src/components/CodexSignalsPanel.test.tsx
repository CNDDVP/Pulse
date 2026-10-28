// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CodexSignalsPanel } from "./CodexSignalsPanel";
import { I18nProvider } from "../lib/i18n";
import type { CodexSignalsReport } from "../types";

const mockReport: CodexSignalsReport = {
  span_days: 30,
  scanned_sessions: 42,
  judged_sessions: 35,
  unjudged_sessions: 7,
  models: [
    {
      model: "gpt-5.6-sol",
      responses: 120,
      reached_lattice: 40,
      on_lattice: 6,
      share: 0.15,
      insufficient_data: false,
    },
    {
      model: "gpt-5.5-mini",
      responses: 18,
      reached_lattice: 8,
      on_lattice: 1,
      share: null,
      insufficient_data: true,
    },
  ],
  changes: [
    {
      kind: "effort",
      asked: "xhigh",
      ran: "medium",
      timestamp: "2026-10-25T10:15:00Z",
      session_title: "rollout-1",
    },
  ],
  total_changes: 10,
};

const invokeMock = vi.fn().mockResolvedValue(mockReport);
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("CodexSignalsPanel", () => {
  it("loads and renders lattice stop shares, insufficient data label, and silently lowered settings", async () => {
    render(
      <I18nProvider lang="zh">
        <CodexSignalsPanel />
      </I18nProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText("gpt-5.6-sol")).toBeTruthy();
    });

    expect(screen.getByText("15.0%")).toBeTruthy();
    expect(screen.getByText("样本不足（<20）")).toBeTruthy();
    expect(screen.getByText(/推理强度调低：xhigh → medium/)).toBeTruthy();
    expect(screen.getByText("另有 9 条历史记录")).toBeTruthy();

    fireEvent.click(screen.getByText("最近 90 天"));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("get_codex_signals", {
        spanDays: 90,
      });
    });
  });
});
