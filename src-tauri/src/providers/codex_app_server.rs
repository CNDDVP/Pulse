use crate::types::{CodexResetCredits, NetworkProxySettings};
use chrono::DateTime;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const RPC_TIMEOUT: Duration = Duration::from_secs(15);

pub fn parse_reset_credits_response(v: &Value) -> CodexResetCredits {
    let block = v
        .pointer("/result/rateLimitResetCredits")
        .or_else(|| v.get("rateLimitResetCredits"));
    let Some(block) = block.filter(|b| b.is_object()) else {
        return CodexResetCredits {
            status: "unreported".into(),
            count: None,
            next_expiry: None,
        };
    };

    let credits_arr = block.get("credits").and_then(Value::as_array);
    let available_items: Vec<&Value> = credits_arr
        .map(|arr| {
            arr.iter()
                .filter(|item| item.get("status").and_then(Value::as_str) == Some("available"))
                .collect()
        })
        .unwrap_or_default();

    let explicit_count = block
        .get("availableCount")
        .and_then(Value::as_i64)
        .filter(|&c| (0..1_000_000).contains(&c))
        .map(|c| c as u32);

    let count = explicit_count.or_else(|| {
        if credits_arr.is_some() {
            Some(available_items.len() as u32)
        } else {
            None
        }
    });

    let Some(count) = count else {
        return CodexResetCredits {
            status: "unreported".into(),
            count: None,
            next_expiry: None,
        };
    };

    let min_expiry_ts = available_items
        .iter()
        .filter_map(|item| {
            item.get("expiresAt").and_then(|val| {
                val.as_i64()
                    .or_else(|| val.as_f64().map(|f| f.round() as i64))
                    .or_else(|| {
                        val.as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp()))
                    })
            })
        })
        .filter(|&ts| ts > 0)
        .min();

    let next_expiry = min_expiry_ts
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.to_rfc3339());

    CodexResetCredits {
        status: if count > 0 || next_expiry.is_some() || explicit_count.is_some() || credits_arr.is_some() {
            "available".into()
        } else {
            "unreported".into()
        },
        count: Some(count),
        next_expiry,
    }
}

pub fn locate_codex() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let names = if cfg!(windows) {
        vec!["codex.exe", "codex.cmd", "codex"]
    } else {
        vec!["codex"]
    };

    // 1. PATH entries
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for name in &names {
                candidates.push(dir.join(name));
            }
        }
    }

    // 2. Standard user & global locations on Windows / Unix
    if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
        for name in &names {
            candidates.push(appdata.join("npm").join(name));
        }
        // nvm-windows versioned folders
        if let Ok(entries) = std::fs::read_dir(appdata.join("nvm")) {
            let mut vers: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|x| x.path()))
                .filter(|p| p.is_dir())
                .collect();
            vers.sort_by(|a, b| b.cmp(a));
            for vdir in vers {
                for name in &names {
                    candidates.push(vdir.join(name));
                }
            }
        }
    }

    if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        for name in &names {
            candidates.push(local_appdata.join("pnpm").join(name));
            candidates.push(
                local_appdata
                    .join("Programs")
                    .join("com.openai.codex")
                    .join("resources")
                    .join("codex-cli")
                    .join("bin")
                    .join(name),
            );
            candidates.push(
                local_appdata
                    .join("Programs")
                    .join("ChatGPT")
                    .join("resources")
                    .join("codex-cli")
                    .join("bin")
                    .join(name),
            );
        }
        // Windows Store OpenAI.Codex package npm cache
        if let Ok(pkgs) = std::fs::read_dir(local_appdata.join("Packages")) {
            for pkg in pkgs.flatten() {
                let fname = pkg.file_name();
                if fname.to_string_lossy().starts_with("OpenAI.Codex_") {
                    let npm_dir = pkg
                        .path()
                        .join("LocalCache")
                        .join("Roaming")
                        .join("npm");
                    for name in &names {
                        candidates.push(npm_dir.join(name));
                    }
                }
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        for sub in [
            ".local/bin",
            ".volta/bin",
            ".bun/bin",
            ".npm-global/bin",
        ] {
            let dir = home.join(sub);
            for name in &names {
                candidates.push(dir.join(name));
            }
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

fn build_child_command(codex_path: &Path, proxy: &NetworkProxySettings) -> tokio::process::Command {
    let ext = codex_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut cmd = if cfg!(windows) && (ext == "cmd" || ext == "bat") {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.args(["/D", "/S", "/C"])
            .arg(format!("\"{}\" app-server", codex_path.display()));
        c
    } else {
        let mut c = tokio::process::Command::new(codex_path);
        c.arg("app-server");
        c
    };

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    // Prepend executable's directory to PATH so `node` is always resolvable for npm scripts.
    if let Some(parent) = codex_path.parent() {
        let mut new_path = parent.as_os_str().to_os_string();
        if let Some(existing) = std::env::var_os("PATH") {
            new_path.push(if cfg!(windows) { ";" } else { ":" });
            new_path.push(existing);
        }
        cmd.env("PATH", new_path);
    }

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

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    cmd
}

pub async fn fetch_reset_credits(proxy: &NetworkProxySettings) -> CodexResetCredits {
    let Some(codex_path) = locate_codex() else {
        return CodexResetCredits {
            status: "codex_missing".into(),
            count: None,
            next_expiry: None,
        };
    };

    let mut cmd = build_child_command(&codex_path, proxy);
    let Ok(mut child) = cmd.spawn() else {
        return CodexResetCredits {
            status: "unreported".into(),
            count: None,
            next_expiry: None,
        };
    };

    let probe = async {
        let mut stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        let req_lines = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"Pulse","title":"Pulse","version":"0.8.0"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"account/rateLimits/read","params":{}}"#,
            "\n"
        );
        stdin.write_all(req_lines.as_bytes()).await.ok()?;
        stdin.flush().await.ok()?;

        let mut lines = BufReader::new(stdout).lines();
        let mut read_count = 0usize;
        while let Ok(Some(line)) = lines.next_line().await {
            read_count += 1;
            if read_count > 200 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                if v.get("id").and_then(Value::as_i64) == Some(2) {
                    return Some(parse_reset_credits_response(&v));
                }
            }
        }
        None
    };

    let result = tokio::time::timeout(RPC_TIMEOUT, probe)
        .await
        .ok()
        .flatten();
    let _ = child.kill().await;
    let _ = child.wait().await;

    result.unwrap_or(CodexResetCredits {
        status: "unreported".into(),
        count: None,
        next_expiry: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_available_reset_credits_and_soonest_expiry() {
        let raw: Value = serde_json::from_str(
            r#"{
              "jsonrpc": "2.0",
              "id": 2,
              "result": {
                "rateLimitResetCredits": {
                  "availableCount": 2,
                  "credits": [
                    { "title": "Reset credit", "status": "used", "expiresAt": 1761000000 },
                    { "title": "Reset credit", "status": "available", "expiresAt": 1762000000 },
                    { "title": "Reset credit", "status": "available", "expiresAt": 1761868800 }
                  ]
                }
              }
            }"#,
        )
        .unwrap();

        let parsed = parse_reset_credits_response(&raw);
        assert_eq!(parsed.status, "available");
        assert_eq!(parsed.count, Some(2));
        assert_eq!(
            parsed.next_expiry.as_deref(),
            Some("2025-10-31T00:00:00+00:00")
        );
    }

    #[test]
    fn falls_back_to_counting_available_entries_when_available_count_missing() {
        let raw: Value = serde_json::from_str(
            r#"{
              "rateLimitResetCredits": {
                "credits": [
                  { "status": "available", "expiresAt": 1761868800 },
                  { "status": "expired", "expiresAt": 1760000000 }
                ]
              }
            }"#,
        )
        .unwrap();
        let parsed = parse_reset_credits_response(&raw);
        assert_eq!(parsed.status, "available");
        assert_eq!(parsed.count, Some(1));
    }

    #[test]
    fn missing_reset_credits_block_returns_unreported() {
        let raw: Value = serde_json::from_str(r#"{"jsonrpc":"2.0","id":2,"result":{}}"#).unwrap();
        let parsed = parse_reset_credits_response(&raw);
        assert_eq!(parsed.status, "unreported");
        assert_eq!(parsed.count, None);
    }
}
