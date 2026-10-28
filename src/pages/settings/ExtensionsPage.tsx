import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, ExtensionEntry } from "../../types";
import { useLang } from "../../lib/i18n";
import { Section, Switch } from "./shared";
import { btnGhost, btnPrimary } from "./constants";

export function ExtensionsPage({
  settings,
  onToggleExtension,
  onOpenAccount,
  toast,
}: {
  settings: AppSettings;
  onToggleExtension: (accountId: string, enabled: boolean) => void;
  onOpenAccount: (accountId: string) => void;
  toast: (type: "success" | "info" | "error", text: string) => void;
}) {
  const { t } = useLang();
  const [entries, setEntries] = useState<ExtensionEntry[]>([]);
  const [scanning, setScanning] = useState(true);

  useEffect(() => {
    let alive = true;
    void invoke<ExtensionEntry[]>("scan_extensions")
      .then((res) => {
        if (alive) setEntries(res);
      })
      .catch(() => {})
      .finally(() => {
        if (alive) setScanning(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  const handleRescan = async () => {
    setScanning(true);
    try {
      const res = await invoke<ExtensionEntry[]>("scan_extensions");
      setEntries(res);
    } catch (e) {
      toast("error", String(e));
    } finally {
      setScanning(false);
    }
  };

  const handleOpenDir = async () => {
    try {
      await invoke<string>("open_extensions_dir");
    } catch (e) {
      toast("error", String(e));
    }
  };

  const handleCreateSample = async () => {
    setScanning(true);
    try {
      const res = await invoke<ExtensionEntry[]>("create_sample_extension");
      setEntries(res);
      toast("success", t("ext.demo_created"));
    } catch (e) {
      toast("error", String(e));
    } finally {
      setScanning(false);
    }
  };

  return (
    <div className="space-y-5 max-w-3xl">
      <Section
        title={t("ext.title")}
        icon="🧩"
        subtitle={t("ext.subtitle")}
        aside={
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => void handleCreateSample()}
              disabled={scanning}
              className={btnGhost}
            >
              {t("ext.create_demo")}
            </button>
            <button
              type="button"
              onClick={() => void handleOpenDir()}
              className={btnGhost}
            >
              {t("ext.open_dir")}
            </button>
            <button
              type="button"
              onClick={() => void handleRescan()}
              disabled={scanning}
              className={btnPrimary}
            >
              {scanning ? t("ext.scanning") : t("ext.rescan")}
            </button>
          </div>
        }
      >
        {entries.length === 0 ? (
          <p className="text-xs text-[var(--text-3)] py-2">{t("ext.empty")}</p>
        ) : (
          <div className="space-y-2.5">
            {entries.map((ext) => {
              const ready = ext.status === "ready";
              const cfg = settings.providers[ext.account_id];
              const enabled = cfg ? cfg.enabled : ext.enabled;
              return (
                <div
                  key={`${ext.folder_name}-${ext.id}`}
                  className="p-3.5 rounded-xl bg-[var(--surface)] border border-[var(--border)] flex items-center justify-between gap-4"
                >
                  <div className="min-w-0 space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-[var(--text-1)] truncate">
                        {ext.name}
                      </span>
                      <span className="text-[11px] font-mono text-[var(--text-3)]">
                        {ext.id}
                      </span>
                      <span
                        className={`text-[10px] px-1.5 py-0.5 rounded font-mono border ${
                          ready
                            ? "bg-[var(--ok-soft)] text-[var(--ok)] border-[var(--ok-border)]"
                            : "bg-[var(--danger-soft)] text-[var(--danger)] border-[var(--danger-border)]"
                        }`}
                      >
                        {t(`ext.status.${ext.status}`)}
                      </span>
                    </div>
                    <div className="text-[11px] font-mono text-[var(--text-3)] truncate">
                      {ext.folder_name}/{ext.executable || "pulse-extension.json"} ·{" "}
                      {ext.timeout_seconds}s
                    </div>
                    {ext.error && (
                      <div className="text-[11px] text-[var(--danger)]">
                        {ext.error}
                      </div>
                    )}
                  </div>

                  {ready && ext.account_id && (
                    <div className="flex items-center gap-3 shrink-0">
                      <button
                        type="button"
                        onClick={() => onOpenAccount(ext.account_id)}
                        className={btnGhost}
                      >
                        {t("ext.configure")}
                      </button>
                      <Switch
                        checked={enabled}
                        onChange={(v) => onToggleExtension(ext.account_id, v)}
                        label={ext.name}
                      />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </Section>
    </div>
  );
}
