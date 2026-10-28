use crate::config::get_config_dir;
use crate::types::{
    Balance, NetworkProxySettings, ProviderConfig, ProviderUsage, UsageMetric, UsageMetricPoint,
    UsageWindow,
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::io::AsyncReadExt;

pub const MAX_OUTPUT_BYTES: usize = 256 * 1024;
pub const METRIC_LIMIT: usize = 4;
pub const HISTORY_LIMIT: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionEntry {
    pub id: String,
    pub name: String,
    pub folder_name: String,
    pub executable: String,
    pub timeout_seconds: u64,
    pub account_id: String,
    pub enabled: bool,
    /// "ready" | "invalid_manifest" | "unsupported_schema" | "duplicate_id" | "executable_outside_folder" | "executable_missing"
    pub status: String,
    pub error: Option<String>,
}

pub fn extensions_root_dir() -> PathBuf {
    get_config_dir().join("extensions")
}

pub fn extension_account_id(ext_id: &str) -> String {
    let sanitized: String = ext_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("ext-{sanitized}")
}

pub fn is_valid_extension_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

fn is_safe_relative_executable(exec: &str) -> bool {
    let trimmed = exec.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains(':')
    {
        return false;
    }
    Path::new(trimmed)
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
}

fn resolve_executable(folder: &Path, exec_rel: &str) -> Result<PathBuf, (&'static str, String)> {
    if !is_safe_relative_executable(exec_rel) {
        return Err((
            "executable_outside_folder",
            "可执行文件必须是扩展子目录内的相对路径".into(),
        ));
    }
    let candidate = folder.join(exec_rel);
    if !candidate.exists() || !candidate.is_file() {
        return Err(("executable_missing", format!("找不到可执行文件: {exec_rel}")));
    }
    let Ok(canon_folder) = std::fs::canonicalize(folder) else {
        return Err(("executable_missing", "无法解析扩展目录".into()));
    };
    let Ok(canon_exec) = std::fs::canonicalize(&candidate) else {
        return Err(("executable_missing", "无法解析可执行文件路径".into()));
    };
    if !canon_exec.starts_with(&canon_folder) {
        return Err((
            "executable_outside_folder",
            "可执行文件越出扩展文件夹边界".into(),
        ));
    }
    Ok(canon_exec)
}

pub fn scan_extensions_in_dir(
    root: &Path,
    providers: &BTreeMap<String, ProviderConfig>,
) -> Vec<ExtensionEntry> {
    if !root.exists() {
        return Vec::new();
    }
    let Ok(read_dir) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = read_dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut entries = Vec::new();

    for dir in dirs {
        let folder_name = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let manifest_path = dir.join("pulse-extension.json");
        if !manifest_path.exists() {
            continue;
        }
        let Ok(bytes) = std::fs::read(&manifest_path) else {
            entries.push(ExtensionEntry {
                id: folder_name.clone(),
                name: folder_name.clone(),
                folder_name,
                executable: String::new(),
                timeout_seconds: 20,
                account_id: String::new(),
                enabled: false,
                status: "invalid_manifest".into(),
                error: Some("无法读取 pulse-extension.json".into()),
            });
            continue;
        };
        let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
            entries.push(ExtensionEntry {
                id: folder_name.clone(),
                name: folder_name.clone(),
                folder_name,
                executable: String::new(),
                timeout_seconds: 20,
                account_id: String::new(),
                enabled: false,
                status: "invalid_manifest".into(),
                error: Some("pulse-extension.json 不是合法 JSON".into()),
            });
            continue;
        };

        let schema_ver = v.get("schemaVersion").and_then(Value::as_i64).unwrap_or(0);
        if schema_ver != 1 {
            entries.push(ExtensionEntry {
                id: folder_name.clone(),
                name: folder_name.clone(),
                folder_name,
                executable: String::new(),
                timeout_seconds: 20,
                account_id: String::new(),
                enabled: false,
                status: "unsupported_schema".into(),
                error: Some(format!("不支持的 schemaVersion: {schema_ver}")),
            });
            continue;
        }

        let id = v
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let name_raw = v
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let exec_raw = v
            .get("executable")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let timeout_secs = v
            .get("timeoutSeconds")
            .and_then(Value::as_f64)
            .filter(|f| f.is_finite())
            .map(|f| (f.round() as i64).clamp(1, 60) as u64)
            .unwrap_or(20);

        if !is_valid_extension_id(id) || name_raw.is_empty() || exec_raw.is_empty() {
            entries.push(ExtensionEntry {
                id: if id.is_empty() {
                    folder_name.clone()
                } else {
                    id.to_string()
                },
                name: if name_raw.is_empty() {
                    folder_name.clone()
                } else {
                    name_raw.chars().take(60).collect()
                },
                folder_name,
                executable: exec_raw.to_string(),
                timeout_seconds: timeout_secs,
                account_id: String::new(),
                enabled: false,
                status: "invalid_manifest".into(),
                error: Some("id、name 或 executable 字段格式无效".into()),
            });
            continue;
        }

        let name: String = name_raw.chars().take(60).collect();
        let account_id = extension_account_id(id);
        let enabled = providers.get(&account_id).is_some_and(|c| c.enabled);

        if !seen_ids.insert(id.to_string()) {
            entries.push(ExtensionEntry {
                id: id.to_string(),
                name,
                folder_name,
                executable: exec_raw.to_string(),
                timeout_seconds: timeout_secs,
                account_id,
                enabled: false,
                status: "duplicate_id".into(),
                error: Some(format!("扩展 ID {id} 与其他文件夹重复")),
            });
            continue;
        }

        match resolve_executable(&dir, exec_raw) {
            Ok(_) => entries.push(ExtensionEntry {
                id: id.to_string(),
                name,
                folder_name,
                executable: exec_raw.to_string(),
                timeout_seconds: timeout_secs,
                account_id,
                enabled,
                status: "ready".into(),
                error: None,
            }),
            Err((code, msg)) => entries.push(ExtensionEntry {
                id: id.to_string(),
                name,
                folder_name,
                executable: exec_raw.to_string(),
                timeout_seconds: timeout_secs,
                account_id,
                enabled,
                status: code.into(),
                error: Some(msg),
            }),
        }
    }

    entries
}

pub fn parse_extension_report(bytes: &[u8]) -> Result<ProviderUsage, ProviderUsage> {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else {
        return Err(ProviderUsage::problem(
            "extension",
            "schema",
            "无法解析扩展返回的 JSON",
        ));
    };
    if v.get("schemaVersion").and_then(Value::as_i64) != Some(1) {
        return Err(ProviderUsage::problem(
            "extension",
            "schema",
            "扩展返回了不支持的 schemaVersion",
        ));
    }

    let status = v.get("status").and_then(Value::as_str).unwrap_or("ok");
    match status {
        "ok" => {}
        "signedOut" => {
            return Err(ProviderUsage::problem(
                "extension",
                "auth",
                "扩展报告登录态已失效",
            ))
        }
        "unreachable" => {
            return Err(ProviderUsage::problem(
                "extension",
                "network",
                "扩展报告上游服务无响应",
            ))
        }
        "rateLimited" => {
            return Err(ProviderUsage::problem(
                "extension",
                "rate_limited",
                "扩展报告请求过于频繁，正在退避",
            ))
        }
        "serverError" => {
            return Err(ProviderUsage::problem(
                "extension",
                "server",
                "扩展报告上游服务返回错误",
            ))
        }
        "noLimits" => {
            return Err(ProviderUsage::problem(
                "extension",
                "no_data",
                "扩展未报告任何可用限额",
            ))
        }
        _ => {
            return Err(ProviderUsage::problem(
                "extension",
                "schema",
                "扩展返回了未知的 status 状态",
            ))
        }
    }

    let plan_name = v
        .get("plan")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(60).collect::<String>());

    let mut windows = Vec::new();
    let mut seen_win_ids = HashSet::new();
    if let Some(limits) = v.get("limits").and_then(Value::as_array) {
        for (idx, item) in limits.iter().enumerate() {
            let Some(label) = item
                .get("label")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let raw_id = item
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(64).collect::<String>())
                .unwrap_or_else(|| idx.to_string());
            let win_id = format!("extension.{raw_id}");
            if !seen_win_ids.insert(win_id.clone()) {
                continue;
            }

            let used_fraction = if let Some(pct) = item
                .get("usedPercent")
                .and_then(Value::as_f64)
                .filter(|f| f.is_finite() && *f >= 0.0)
            {
                Some(pct / 100.0)
            } else if let (Some(used), Some(limit)) = (
                item.get("used")
                    .and_then(Value::as_f64)
                    .filter(|f| f.is_finite() && *f >= 0.0),
                item.get("limit")
                    .and_then(Value::as_f64)
                    .filter(|f| f.is_finite() && *f > 0.0),
            ) {
                Some(used / limit)
            } else {
                None
            };
            let Some(frac) = used_fraction else {
                continue;
            };

            let resets_at = item
                .get("resetsAt")
                .and_then(Value::as_str)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.to_rfc3339());
            let window_seconds = item
                .get("windowSeconds")
                .and_then(Value::as_i64)
                .filter(|&s| s > 0);

            windows.push(UsageWindow {
                id: win_id,
                name: label.chars().take(60).collect(),
                used_fraction: frac.clamp(0.0, 1.0),
                used_percent: (frac * 100.0).clamp(0.0, 100.0),
                resets_at,
                window_seconds,
                exhausted: frac >= 1.0,
            });
        }
    }

    let mut balances = Vec::new();
    if let Some(bal) = v.get("balance").and_then(Value::as_object) {
        if let (Some(amount), Some(currency)) = (
            bal.get("amount")
                .and_then(Value::as_f64)
                .filter(|f| f.is_finite()),
            bal.get("currency")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|c| c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase())),
        ) {
            balances.push(Balance::new(currency, amount));
        }
    }

    let mut metrics = Vec::new();
    let mut seen_metric_ids = HashSet::new();
    if let Some(m_arr) = v.get("metrics").and_then(Value::as_array) {
        for (idx, m) in m_arr.iter().enumerate() {
            if metrics.len() >= METRIC_LIMIT {
                break;
            }
            let Some(label) = m
                .get("label")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let Some(value) = m
                .get("value")
                .and_then(Value::as_f64)
                .filter(|f| f.is_finite())
            else {
                continue;
            };
            let id = m
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(64).collect::<String>())
                .unwrap_or_else(|| format!("metric-{idx}"));
            if !seen_metric_ids.insert(id.clone()) {
                continue;
            }
            let unit = m
                .get("unit")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(16).collect::<String>());

            let mut history: Vec<(i64, UsageMetricPoint)> = Vec::new();
            let mut seen_ts = HashSet::new();
            if let Some(h_arr) = m.get("history").and_then(Value::as_array) {
                for pt in h_arr {
                    let Some(t_str) = pt.get("t").and_then(Value::as_str) else {
                        continue;
                    };
                    let Ok(dt) = DateTime::parse_from_rfc3339(t_str) else {
                        continue;
                    };
                    let Some(val) = pt
                        .get("v")
                        .and_then(Value::as_f64)
                        .filter(|f| f.is_finite())
                    else {
                        continue;
                    };
                    let ts_ms = dt.timestamp_millis();
                    if seen_ts.insert(ts_ms) {
                        history.push((
                            ts_ms,
                            UsageMetricPoint {
                                t: dt.to_rfc3339(),
                                v: val,
                            },
                        ));
                    }
                }
            }
            history.sort_by_key(|(ts, _)| *ts);
            let trimmed_history: Vec<UsageMetricPoint> = history
                .into_iter()
                .rev()
                .take(HISTORY_LIMIT)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|(_, pt)| pt)
                .collect();

            metrics.push(UsageMetric {
                id,
                label: label.chars().take(60).collect(),
                value,
                unit,
                history: trimmed_history,
            });
        }
    }

    if windows.is_empty() && balances.is_empty() && metrics.is_empty() {
        return Err(ProviderUsage::problem(
            "extension",
            "no_data",
            "扩展未返回任何限额、余额或指标数据",
        ));
    }

    let primary_percent = windows.iter().map(|w| w.used_percent).reduce(f64::max);
    Ok(ProviderUsage {
        provider_id: "extension".into(),
        display_name: "自定义扩展".into(),
        state: "live".into(),
        primary_percent,
        plan_name,
        windows,
        balances,
        metrics: if metrics.is_empty() {
            None
        } else {
            Some(metrics)
        },
        source: "自定义扩展脚本 → 本地沙箱执行".into(),
        ..Default::default()
    })
}

static BASELINE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn apply_balance_ring_basis(
    reading: &mut ProviderUsage,
    account_id: &str,
    cfg: &ProviderConfig,
) {
    if reading.state != "live" || !reading.windows.is_empty() || reading.balances.is_empty() {
        return;
    }
    let bal = &reading.balances[0];
    if !bal.amount.is_finite() {
        return;
    }
    let basis = cfg
        .balance_basis
        .as_deref()
        .unwrap_or(if cfg.primary_window.as_deref() == Some("__balance__") {
            "balance_only"
        } else {
            "since_top_up"
        });

    match basis {
        "balance_only" => {
            reading.primary_percent = None;
        }
        "budget" => {
            if let Some(budget) = cfg.balance_budget.filter(|b| b.is_finite() && *b > 0.0) {
                let used_frac = ((budget - bal.amount) / budget).clamp(0.0, 1.0);
                reading.primary_percent = Some(used_frac * 100.0);
            } else {
                reading.primary_percent = None;
            }
        }
        _ => {
            // "since_top_up": track peak balance per account_id|currency
            let lock = BASELINE_LOCK.get_or_init(|| Mutex::new(()));
            let _guard = lock.lock().ok();
            let path = get_config_dir().join("balance-baselines.json");
            let mut map: BTreeMap<String, f64> = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let key = format!("{account_id}|{}", bal.currency);
            let prev_peak = map.get(&key).copied().unwrap_or(0.0);
            let peak = if bal.amount > prev_peak {
                let new_peak = bal.amount.max(0.0);
                map.insert(key, new_peak);
                if let Ok(bytes) = serde_json::to_vec_pretty(&map) {
                    let _ = crate::config::atomic_write(&path, &bytes);
                }
                new_peak
            } else {
                prev_peak
            };
            if peak > 0.0 {
                let used_frac = ((peak - bal.amount) / peak).clamp(0.0, 1.0);
                reading.primary_percent = Some(used_frac * 100.0);
            } else {
                reading.primary_percent = None;
            }
        }
    }
}

pub async fn fetch_extension(
    account_id: &str,
    cfg: &ProviderConfig,
    proxy: &NetworkProxySettings,
) -> ProviderUsage {
    let root = extensions_root_dir();
    let mut dummy_map = BTreeMap::new();
    dummy_map.insert(account_id.to_string(), cfg.clone());
    let entries = scan_extensions_in_dir(&root, &dummy_map);

    let matched = entries.into_iter().find(|e| {
        e.account_id == account_id
            || cfg.extension_id.as_deref() == Some(e.id.as_str())
    });

    let Some(entry) = matched else {
        return ProviderUsage::problem(
            "extension",
            "extension_missing",
            "未找到对应的扩展文件夹或 pulse-extension.json",
        );
    };
    if entry.status != "ready" {
        return ProviderUsage::problem(
            "extension",
            &entry.status,
            entry.error.as_deref().unwrap_or("扩展状态异常，无法运行"),
        );
    }

    let folder = root.join(&entry.folder_name);
    let Ok(exec_path) = resolve_executable(&folder, &entry.executable) else {
        return ProviderUsage::problem("extension", "executable_missing", "可执行文件校验失败");
    };

    let ext = exec_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut cmd = if ext == "ps1" {
        let mut c = tokio::process::Command::new("powershell.exe");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&exec_path);
        c
    } else if matches!(ext.as_str(), "js" | "mjs" | "cjs") {
        let mut c = tokio::process::Command::new("node");
        c.arg(&exec_path);
        c
    } else if ext == "py" {
        let mut c = tokio::process::Command::new("python");
        c.arg(&exec_path);
        c
    } else if cfg!(windows) && matches!(ext.as_str(), "bat" | "cmd") {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.args(["/D", "/S", "/C"]).arg(&exec_path);
        c
    } else {
        tokio::process::Command::new(&exec_path)
    };

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    // Sanitized environment: never pass other providers' credentials
    cmd.env_clear();
    for key in [
        "SystemRoot",
        "WINDIR",
        "COMSPEC",
        "PATHEXT",
        "PATH",
        "USERPROFILE",
        "HOME",
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "LANG",
    ] {
        if let Some(val) = std::env::var_os(key) {
            cmd.env(key, val);
        }
    }
    cmd.env("PULSE_EXTENSION_ID", &entry.id)
        .env("PULSE_EXTENSION_SCHEMA", "1")
        .current_dir(&folder)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    match proxy.mode.as_str() {
        "manual_http" if !proxy.host.is_empty() && proxy.port > 0 => {
            let url = format!("http://{}:{}", proxy.host, proxy.port);
            cmd.env("HTTP_PROXY", &url)
                .env("HTTPS_PROXY", &url)
                .env("NO_PROXY", "localhost,127.0.0.1,::1");
        }
        "manual_socks5" if !proxy.host.is_empty() && proxy.port > 0 => {
            let url = format!("socks5h://{}:{}", proxy.host, proxy.port);
            cmd.env("ALL_PROXY", &url)
                .env("NO_PROXY", "localhost,127.0.0.1,::1");
        }
        _ => {}
    }

    let Ok(mut child) = cmd.spawn() else {
        return ProviderUsage::problem(
            "extension",
            "extension_failed",
            "无法启动扩展脚本进程",
        );
    };

    let timeout_dur = Duration::from_secs(entry.timeout_seconds.clamp(1, 60));
    let read_task = async {
        let mut stdout = child.stdout.take()?;
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match stdout.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    let take = n.min(MAX_OUTPUT_BYTES.saturating_sub(buf.len()));
                    buf.extend_from_slice(&chunk[..take]);
                    if buf.len() >= MAX_OUTPUT_BYTES {
                        break;
                    }
                }
                Err(_) => return None,
            }
        }
        let status = child.wait().await.ok()?;
        Some((status.success(), buf))
    };

    match tokio::time::timeout(timeout_dur, read_task).await {
        Ok(Some((true, bytes))) => {
            let mut r = match parse_extension_report(&bytes) {
                Ok(r) => r,
                Err(err_r) => err_r,
            };
            r.scope = format!("ext:{}", entry.id);
            apply_balance_ring_basis(&mut r, account_id, cfg);
            r
        }
        Ok(Some((false, _))) => {
            let _ = child.kill().await;
            ProviderUsage::problem("extension", "extension_failed", "扩展执行返回非零退出码")
        }
        Ok(None) => {
            let _ = child.kill().await;
            ProviderUsage::problem("extension", "extension_failed", "扩展输出读取失败")
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            ProviderUsage::problem("extension", "extension_timed_out", "扩展执行超时")
        }
    }
}

pub fn create_sample_extension_in(root: &Path) -> Result<String, String> {
    let sample_dir = root.join("sample-relay");
    std::fs::create_dir_all(&sample_dir).map_err(|e| format!("无法创建示例扩展目录: {e}"))?;
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "sample-relay",
        "name": "Sample Relay Gateway",
        "executable": "run.js",
        "timeoutSeconds": 10
    });
    std::fs::write(
        sample_dir.join("pulse-extension.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .map_err(|e| format!("写入清单失败: {e}"))?;

    let script = r#"const now = Date.now();
const resetsAt = new Date(now + 4 * 3600 * 1000).toISOString();
const points = [380, 355, 340, 310, 295, 285].map((v, i) => ({
  t: new Date(now - (5 - i) * 600 * 1000).toISOString(),
  v
}));
console.log(JSON.stringify({
  schemaVersion: 1,
  status: "ok",
  plan: "Relay Pro",
  limits: [
    {
      id: "rolling_5h",
      label: "5h Requests",
      usedPercent: 36.5,
      resetsAt,
      windowSeconds: 18000
    }
  ],
  balance: {
    amount: 42.80,
    currency: "USD"
  },
  metrics: [
    {
      id: "ttft",
      label: "TTFT 首字延迟",
      value: 285,
      unit: "ms",
      history: points
    },
    {
      id: "tps",
      label: "输出速率",
      value: 78.4,
      unit: "tok/s"
    }
  ]
}));
"#;
    std::fs::write(sample_dir.join("run.js"), script)
        .map_err(|e| format!("写入示例脚本失败: {e}"))?;
    Ok(sample_dir.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extension_report_with_limits_balance_and_metrics() {
        let raw = br#"{
          "schemaVersion": 1,
          "status": "ok",
          "plan": "Team",
          "limits": [
            { "id": "monthly", "label": "Monthly requests", "usedPercent": 42.5, "windowSeconds": 2592000 },
            { "id": "tokens", "label": "Tokens", "used": 30000, "limit": 120000 }
          ],
          "balance": { "amount": 16.33, "currency": "USD" },
          "metrics": [
            {
              "id": "ttft",
              "label": "Time to first token",
              "value": 320,
              "unit": "ms",
              "history": [
                { "t": "2026-10-12T14:00:00Z", "v": 410 },
                { "t": "2026-10-12T15:00:00Z", "v": 320 }
              ]
            }
          ]
        }"#;

        let r = parse_extension_report(raw).expect("valid report");
        assert_eq!(r.state, "live");
        assert_eq!(r.plan_name.as_deref(), Some("Team"));
        assert_eq!(r.windows.len(), 2);
        assert!((r.windows[0].used_percent - 42.5).abs() < 1e-6);
        assert!((r.windows[1].used_percent - 25.0).abs() < 1e-6);
        assert_eq!(r.balances.len(), 1);
        assert_eq!(r.balances[0].currency, "USD");
        let m = r.metrics.expect("metrics present");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].history.len(), 2);
    }

    #[test]
    fn rejects_executable_path_traversal_outside_folder() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("evil-ext");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(
            ext_dir.join("pulse-extension.json"),
            r#"{"schemaVersion":1,"id":"evil","name":"Evil","executable":"../outside.exe"}"#,
        )
        .unwrap();

        let list = scan_extensions_in_dir(dir.path(), &BTreeMap::new());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, "executable_outside_folder");
    }
}
