// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ExtensionsPage } from "./ExtensionsPage";
import { MetricTrendLine } from "../../components/MetricTrendLine";
import { computeTrendPoints, formatMetricValue } from "../../lib/metricsFormat";
import { I18nProvider } from "../../lib/i18n";
import type { AppSettings, ExtensionEntry } from "../../types";

const mockExtensions: ExtensionEntry[] = [
  {
    id: "sample-relay",
    name: "Sample Relay Gateway",
    folder_name: "sample-relay",
    executable: "run.js",
    timeout_seconds: 10,
    account_id: "ext-sample-relay",
    enabled: false,
    status: "ready",
    error: null,
  },
  {
    id: "bad-ext",
    name: "Broken Extension",
    folder_name: "bad-ext",
    executable: "../outside.exe",
    timeout_seconds: 20,
    account_id: "",
    enabled: false,
    status: "executable_outside_folder",
    error: "可执行文件越出扩展文件夹边界",
  },
];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "scan_extensions" || cmd === "create_sample_extension") {
      return mockExtensions;
    }
    return "D:\\extensions";
  }),
}));

describe("ExtensionsPage & MetricTrendLine", () => {
  it("formats metric values with units and significant digits", () => {
    expect(formatMetricValue(320, "ms")).toBe("320 ms");
    expect(formatMetricValue(99.2, "%")).toBe("99.2%");
    expect(formatMetricValue(0.00456, "s")).toBe("0.0046 s");
  });

  it("computes normalized trend line coordinates", () => {
    const pts = computeTrendPoints(
      [
        { t: "2026-10-25T10:00:00Z", v: 100 },
        { t: "2026-10-25T11:00:00Z", v: 200 },
      ],
      200,
      30,
      4,
    );
    expect(pts.length).toBe(2);
    expect(pts[0].x).toBeCloseTo(4);
    expect(pts[1].x).toBeCloseTo(196);
    expect(pts[0].y).toBeCloseTo(26);
    expect(pts[1].y).toBeCloseTo(4);
  });

  it("renders MetricTrendLine and ExtensionsPage with status badges", async () => {
    const onToggle = vi.fn();
    const onOpen = vi.fn();
    const settings = {
      providers: {
        "ext-sample-relay": {
          provider_id: "extension",
          label: "Sample Relay Gateway",
          enabled: false,
        },
      },
    } as unknown as AppSettings;

    render(
      <I18nProvider lang="zh">
        <div>
          <MetricTrendLine
            history={[
              { t: "2026-10-25T10:00:00Z", v: 320 },
              { t: "2026-10-25T10:10:00Z", v: 280 },
            ]}
            unit="ms"
            height={64}
            interactive={true}
          />
          <ExtensionsPage
            settings={settings}
            onToggleExtension={onToggle}
            onOpenAccount={onOpen}
            toast={() => {}}
          />
        </div>
      </I18nProvider>,
    );

    expect(screen.getByTestId("metric-trend-line")).toBeTruthy();

    await waitFor(() => {
      expect(screen.getByText("Sample Relay Gateway")).toBeTruthy();
    });
    expect(screen.getByText("就绪")).toBeTruthy();
    expect(screen.getByText("可执行文件越界")).toBeTruthy();

    fireEvent.click(screen.getByText("配置圆环与指标"));
    expect(onOpen).toHaveBeenCalledWith("ext-sample-relay");
  });
});
