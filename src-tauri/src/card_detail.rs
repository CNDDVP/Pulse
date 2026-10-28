use crate::ledger::{database, estimate_model_cost, frozen_tz, ledger_db_path};
use crate::types::ProviderUsage;
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowValueEstimate {
    pub window_id: String,
    pub full_usd: f64,
    pub used_usd: f64,
    pub unpriced_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DayTokenBar {
    pub day: String,
    pub tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopModelShare {
    pub name: String,
    pub share_percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardActivitySummary {
    pub is_account_wide: bool,
    pub today_tokens: u64,
    pub today_cost_usd: f64,
    pub today_unpriced: u64,
    pub week_tokens: u64,
    pub week_cost_usd: f64,
    pub week_unpriced: u64,
    pub month_tokens: u64,
    pub month_cost_usd: f64,
    pub month_unpriced: u64,
    pub days_31: Vec<DayTokenBar>,
    pub top_model: Option<TopModelShare>,
    pub cache_hit_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptCacheChat {
    pub id: String,
    pub title: String,
    pub replied_at: String,
    pub expires_at: String,
    pub lifetime_seconds: i64,
    pub is_minimum: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptCacheStatus {
    pub provider_id: String,
    pub lifetime_seconds: i64,
    pub is_minimum: bool,
    pub live_chats: Vec<PromptCacheChat>,
    pub recently_lapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountCardDetail {
    pub account_id: String,
    pub window_estimates: BTreeMap<String, WindowValueEstimate>,
    pub activity: Option<CardActivitySummary>,
    pub prompt_cache: Option<PromptCacheStatus>,
}

pub fn ledger_sources_for_provider(provider_id: &str) -> Vec<&'static str> {
    match provider_id {
        "claude" => vec!["claude"],
        "codex" => vec!["codex"],
        "zhipu" | "zai" => vec!["zcode"],
        "opencode" => vec!["opencode"],
        "antigravity" => vec!["gemini"],
        "command-code" => vec!["claude"],
        _ => vec![],
    }
}

pub fn compute_card_detail_from_db(
    db_path: &Path,
    usage: &ProviderUsage,
    token_spend_enabled: bool,
    now: DateTime<Utc>,
) -> AccountCardDetail {
    let prompt_cache = match usage.provider_id.as_str() {
        "claude" => scan_claude_prompt_cache(now),
        "codex" => scan_codex_prompt_cache(now),
        _ => None,
    };

    if !token_spend_enabled || !db_path.exists() {
        return AccountCardDetail {
            account_id: usage.account_id.clone(),
            window_estimates: BTreeMap::new(),
            activity: None,
            prompt_cache,
        };
    }

    let sources = ledger_sources_for_provider(&usage.provider_id);
    if sources.is_empty() {
        return AccountCardDetail {
            account_id: usage.account_id.clone(),
            window_estimates: BTreeMap::new(),
            activity: None,
            prompt_cache,
        };
    }

    let Ok(db) = database(db_path) else {
        return AccountCardDetail {
            account_id: usage.account_id.clone(),
            window_estimates: BTreeMap::new(),
            activity: None,
            prompt_cache,
        };
    };

    let window_estimates = compute_window_estimates(&db, usage, &sources, now);
    let activity = compute_card_activity(&db, &sources, now);

    AccountCardDetail {
        account_id: usage.account_id.clone(),
        window_estimates,
        activity,
        prompt_cache,
    }
}

fn compute_window_estimates(
    db: &rusqlite::Connection,
    usage: &ProviderUsage,
    sources: &[&str],
    now: DateTime<Utc>,
) -> BTreeMap<String, WindowValueEstimate> {
    let mut out = BTreeMap::new();
    if matches!(usage.provider_id.as_str(), "opencode" | "deepseek") {
        return out;
    }

    let primary_src = sources[0];
    let earliest_ts: Option<i64> = db
        .query_row(
            "SELECT MIN(ts) FROM events WHERE source = ?1",
            params![primary_src],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let Some(earliest_ts) = earliest_ts else {
        return out;
    };

    let observed_ts = usage
        .checked_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp())
        .unwrap_or_else(|| now.timestamp())
        .min(now.timestamp());

    let Ok(mut stmt) = db.prepare(
        "SELECT model, SUM(input), SUM(output), SUM(cache_read), SUM(cache_write)
         FROM (
             SELECT event_id, model, MAX(ts) AS ts, MAX(input) AS input, MAX(output) AS output,
                    MAX(cache_read) AS cache_read, MAX(cache_write) AS cache_write
             FROM events
             WHERE source = ?1 AND ts >= ?2 AND ts <= ?3
             GROUP BY event_id
         )
         GROUP BY model",
    ) else {
        return out;
    };

    for w in &usage.windows {
        // Account-wide windows only (skip model-scoped windows that contain ' · ')
        if w.name.contains(" · ") {
            continue;
        }
        if !w.used_fraction.is_finite() || w.used_fraction < 0.05 || w.used_fraction > 1.0 {
            continue;
        }
        let Some(win_secs) = w.window_seconds.filter(|&s| s > 0) else {
            continue;
        };
        let Some(reset_ts) = w
            .resets_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp())
        else {
            continue;
        };
        let opened_ts = reset_ts - win_secs;
        if !(opened_ts < observed_ts && observed_ts < reset_ts) {
            continue;
        }
        // Require local ledger records to have started on or before the window opened
        if earliest_ts > opened_ts {
            continue;
        }

        let mut spent = 0.0;
        let mut unpriced_tokens = 0u64;
        if let Ok(rows) = stmt.query_map(params![primary_src, opened_ts, observed_ts], |r| {
            Ok((
                r.get::<_, String>(0)?,
                [
                    r.get::<_, u64>(1)?,
                    r.get::<_, u64>(2)?,
                    r.get::<_, u64>(3)?,
                    r.get::<_, u64>(4)?,
                ],
            ))
        }) {
            for (model, counts) in rows.flatten() {
                if let Some(cost) = estimate_model_cost(&model, &counts) {
                    spent += cost;
                } else {
                    unpriced_tokens += counts.iter().sum::<u64>();
                }
            }
        }

        // Minimum spend guard ($0.20) aligned with upstream BudgetEstimator
        if spent >= 0.20 {
            let full_usd = spent / w.used_fraction;
            out.insert(
                w.id.clone(),
                WindowValueEstimate {
                    window_id: w.id.clone(),
                    full_usd,
                    used_usd: spent,
                    unpriced_tokens,
                },
            );
        }
    }

    out
}

fn compute_card_activity(
    db: &rusqlite::Connection,
    sources: &[&str],
    now: DateTime<Utc>,
) -> Option<CardActivitySummary> {
    let tz = frozen_tz(db);
    let today = now.with_timezone(&tz).date_naive();
    let start_31 = today - Duration::days(30);
    let start_7 = today - Duration::days(6);
    let today_key = today.format("%Y-%m-%d").to_string();
    let start_7_key = start_7.format("%Y-%m-%d").to_string();
    let start_31_key = start_31.format("%Y-%m-%d").to_string();

    let mut merged: BTreeMap<(String, String), [u64; 4]> = BTreeMap::new();
    for &src in sources {
        if let Ok(mut s1) = db.prepare(
            "SELECT day, model, input, output, cache_read, cache_write
             FROM daily_archive
             WHERE source = ?1 AND day >= ?2 AND day <= ?3",
        ) {
            if let Ok(rows) = s1.query_map(params![src, &start_31_key, &today_key], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    [
                        r.get::<_, u64>(2)?,
                        r.get::<_, u64>(3)?,
                        r.get::<_, u64>(4)?,
                        r.get::<_, u64>(5)?,
                    ],
                ))
            }) {
                for (day, model, c) in rows.flatten() {
                    let e = merged.entry((day, model)).or_insert([0; 4]);
                    for i in 0..4 {
                        e[i] = e[i].max(c[i]);
                    }
                }
            }
        }

        if let Ok(mut s2) = db.prepare(
            "SELECT model, ts, MAX(input), MAX(output), MAX(cache_read), MAX(cache_write)
             FROM events
             WHERE source = ?1
             GROUP BY event_id",
        ) {
            if let Ok(rows) = s2.query_map(params![src], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    [
                        r.get::<_, u64>(2)?,
                        r.get::<_, u64>(3)?,
                        r.get::<_, u64>(4)?,
                        r.get::<_, u64>(5)?,
                    ],
                ))
            }) {
                let mut by_day_model: BTreeMap<(String, String), [u64; 4]> = BTreeMap::new();
                for (model, ts, c) in rows.flatten() {
                    let Some(day) = DateTime::from_timestamp(ts, 0)
                        .map(|d| d.with_timezone(&tz).format("%Y-%m-%d").to_string())
                    else {
                        continue;
                    };
                    if day < start_31_key || day > today_key {
                        continue;
                    }
                    let e = by_day_model.entry((day, model)).or_insert([0; 4]);
                    for i in 0..4 {
                        e[i] += c[i];
                    }
                }
                for (k, c) in by_day_model {
                    let e = merged.entry(k).or_insert([0; 4]);
                    for i in 0..4 {
                        e[i] = e[i].max(c[i]);
                    }
                }
            }
        }
    }

    if merged.is_empty() {
        return None;
    }

    let mut today_tokens = 0u64;
    let mut today_cost_usd = 0.0;
    let mut today_unpriced = 0u64;

    let mut week_tokens = 0u64;
    let mut week_cost_usd = 0.0;
    let mut week_unpriced = 0u64;

    let mut month_tokens = 0u64;
    let mut month_cost_usd = 0.0;
    let mut month_unpriced = 0u64;

    let mut per_day_tokens: BTreeMap<String, u64> = BTreeMap::new();
    let mut per_model_tokens: BTreeMap<String, u64> = BTreeMap::new();
    let mut cache_tally = [0u64; 4];

    for ((day, model), c) in &merged {
        let tok: u64 = c.iter().sum();
        if tok == 0 {
            continue;
        }
        let cost_opt = estimate_model_cost(model, c);
        *per_day_tokens.entry(day.clone()).or_insert(0) += tok;
        if !model.is_empty() && model != "unknown" {
            *per_model_tokens.entry(model.clone()).or_insert(0) += tok;
        }
        if c[2] > 0 || c[3] > 0 {
            for i in 0..4 {
                cache_tally[i] += c[i];
            }
        }

        month_tokens += tok;
        if let Some(cost) = cost_opt {
            month_cost_usd += cost;
        } else {
            month_unpriced += tok;
        }

        if day.as_str() >= start_7_key.as_str() {
            week_tokens += tok;
            if let Some(cost) = cost_opt {
                week_cost_usd += cost;
            } else {
                week_unpriced += tok;
            }
        }

        if day == &today_key {
            today_tokens += tok;
            if let Some(cost) = cost_opt {
                today_cost_usd += cost;
            } else {
                today_unpriced += tok;
            }
        }
    }

    let mut days_31 = Vec::with_capacity(31);
    for offset in 0..31 {
        let d = (start_31 + Duration::days(offset))
            .format("%Y-%m-%d")
            .to_string();
        let tokens = per_day_tokens.get(&d).copied().unwrap_or(0);
        days_31.push(DayTokenBar { day: d, tokens });
    }

    let top_model = per_model_tokens
        .into_iter()
        .max_by_key(|(_, t)| *t)
        .filter(|(_, t)| *t > 0 && month_tokens > 0)
        .map(|(name, t)| TopModelShare {
            name,
            share_percent: (((t as f64) / (month_tokens as f64)) * 100.0).round() as u32,
        });

    let cache_denom = cache_tally[0] + cache_tally[2] + cache_tally[3];
    let cache_hit_rate = if cache_denom > 0 && cache_tally[2] > 0 {
        Some((cache_tally[2] as f64) / (cache_denom as f64))
    } else {
        None
    };

    Some(CardActivitySummary {
        is_account_wide: false,
        today_tokens,
        today_cost_usd,
        today_unpriced,
        week_tokens,
        week_cost_usd,
        week_unpriced,
        month_tokens,
        month_cost_usd,
        month_unpriced,
        days_31,
        top_model,
        cache_hit_rate,
    })
}

fn read_tail_lines(path: &Path, max_bytes: u64) -> Vec<String> {
    let Ok(mut f) = File::open(path) else {
        return Vec::new();
    };
    let Ok(meta) = f.metadata() else {
        return Vec::new();
    };
    let len = meta.len();
    if len > max_bytes {
        let _ = f.seek(SeekFrom::Start(len - max_bytes));
    }
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    String::from_utf8_lossy(&buf)
        .lines()
        .map(str::to_string)
        .collect()
}

pub fn scan_claude_prompt_cache(now: DateTime<Utc>) -> Option<PromptCacheStatus> {
    let root = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(|d| PathBuf::from(d).join("projects"))
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude").join("projects")))?;
    if !root.exists() {
        return None;
    }

    let now_secs = now.timestamp();
    let cutoff_24h = now_secs - 86400;
    let mut live_chats = Vec::new();
    let mut recently_lapsed = false;
    let mut last_lifetime = 300i64;

    let Ok(proj_dirs) = std::fs::read_dir(&root) else {
        return None;
    };
    for proj in proj_dirs.flatten() {
        let proj_path = proj.path();
        if !proj_path.is_dir() {
            continue;
        }
        let proj_name = proj.file_name().to_string_lossy().to_string();
        let Ok(files) = std::fs::read_dir(&proj_path) else {
            continue;
        };
        for entry in files.flatten() {
            let p = entry.path();
            if !p.is_file() || p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if mtime < cutoff_24h {
                continue;
            }

            let lines = read_tail_lines(&p, 256 * 1024);
            let mut latest_reply_ts: Option<DateTime<Utc>> = None;
            let mut lifetime_secs = 300i64;

            for line in lines.iter().rev() {
                if !line.contains("cache_read_input_tokens")
                    && !line.contains("cache_creation_input_tokens")
                {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                let Some(usage) = v.pointer("/message/usage").or_else(|| v.get("usage")) else {
                    continue;
                };
                let cr = usage
                    .get("cache_read_input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let cw = usage
                    .get("cache_creation_input_tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if cr == 0 && cw == 0 {
                    continue;
                }
                if let Some(ts_str) = v.get("timestamp").and_then(Value::as_str) {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
                        latest_reply_ts = Some(dt.with_timezone(&Utc));
                        if usage
                            .pointer("/cache_creation/ephemeral_1h_input_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            > 0
                        {
                            lifetime_secs = 3600;
                        }
                        break;
                    }
                }
            }

            if let Some(replied_at) = latest_reply_ts {
                last_lifetime = lifetime_secs;
                let expires_at = replied_at + Duration::seconds(lifetime_secs);
                if expires_at > now {
                    let id = p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("chat")
                        .to_string();
                    live_chats.push(PromptCacheChat {
                        id: id.clone(),
                        title: proj_name.clone(),
                        replied_at: replied_at.to_rfc3339(),
                        expires_at: expires_at.to_rfc3339(),
                        lifetime_seconds: lifetime_secs,
                        is_minimum: false,
                    });
                } else if (now - expires_at).num_seconds() <= 86400 {
                    recently_lapsed = true;
                }
            }
        }
    }

    if live_chats.is_empty() && !recently_lapsed {
        return None;
    }
    live_chats.sort_by(|a, b| a.expires_at.cmp(&b.expires_at));
    let top_lifetime = live_chats
        .first()
        .map(|c| c.lifetime_seconds)
        .unwrap_or(last_lifetime);

    Some(PromptCacheStatus {
        provider_id: "claude".into(),
        lifetime_seconds: top_lifetime,
        is_minimum: false,
        live_chats,
        recently_lapsed,
    })
}

fn is_gpt_5_6_or_newer(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("gpt-") else {
        return false;
    };
    let num_part: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = num_part.split('.');
    let Some(major) = parts.next().and_then(|s| s.parse::<u32>().ok()) else {
        return false;
    };
    let minor = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    major > 5 || (major == 5 && minor >= 6)
}

pub fn scan_codex_prompt_cache(now: DateTime<Utc>) -> Option<PromptCacheStatus> {
    let root = std::env::var_os("CODEX_HOME")
        .map(|d| PathBuf::from(d).join("sessions"))
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex").join("sessions")))?;
    if !root.exists() {
        return None;
    }

    let now_secs = now.timestamp();
    let cutoff_24h = now_secs - 86400;
    let mut live_chats = Vec::new();
    let mut recently_lapsed = false;
    let lifetime_secs = 1800i64; // 30 minutes minimum guarantee on GPT-5.6+

    let mut stack = vec![(root, 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 6 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                stack.push((p, depth + 1));
            } else if meta.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                if mtime < cutoff_24h {
                    continue;
                }

                let lines = read_tail_lines(&p, 256 * 1024);
                let mut current_model = String::new();
                let mut latest_cached_ts: Option<DateTime<Utc>> = None;

                for line in &lines {
                    if !line.contains("turn_context") && !line.contains("cached_input_tokens") {
                        continue;
                    }
                    let Ok(v) = serde_json::from_str::<Value>(line) else {
                        continue;
                    };
                    let t = v.get("type").and_then(Value::as_str).unwrap_or("");
                    if t == "turn_context" {
                        if let Some(m) = v.pointer("/payload/model").and_then(Value::as_str) {
                            current_model = m.to_string();
                        }
                    } else if t == "event_msg"
                        && v.pointer("/payload/type").and_then(Value::as_str) == Some("token_count")
                    {
                        let cached = v
                            .pointer("/payload/info/last_token_usage/cached_input_tokens")
                            .or_else(|| {
                                v.pointer("/payload/info/total_token_usage/cached_input_tokens")
                            })
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        if cached > 0 && is_gpt_5_6_or_newer(&current_model) {
                            if let Some(ts_str) = v.get("timestamp").and_then(Value::as_str) {
                                if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
                                    latest_cached_ts = Some(dt.with_timezone(&Utc));
                                }
                            }
                        }
                    }
                }

                if let Some(replied_at) = latest_cached_ts {
                    let expires_at = replied_at + Duration::seconds(lifetime_secs);
                    if expires_at > now {
                        let id = p
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("codex-session")
                            .to_string();
                        live_chats.push(PromptCacheChat {
                            id: id.clone(),
                            title: id,
                            replied_at: replied_at.to_rfc3339(),
                            expires_at: expires_at.to_rfc3339(),
                            lifetime_seconds: lifetime_secs,
                            is_minimum: true,
                        });
                    } else if (now - expires_at).num_seconds() <= 86400 {
                        recently_lapsed = true;
                    }
                }
            }
        }
    }

    if live_chats.is_empty() && !recently_lapsed {
        return None;
    }
    live_chats.sort_by(|a, b| a.expires_at.cmp(&b.expires_at));

    Some(PromptCacheStatus {
        provider_id: "codex".into(),
        lifetime_seconds: lifetime_secs,
        is_minimum: true,
        live_chats,
        recently_lapsed,
    })
}

pub fn get_account_card_detail(
    usage: &ProviderUsage,
    token_spend_enabled: bool,
) -> AccountCardDetail {
    compute_card_detail_from_db(&ledger_db_path(), usage, token_spend_enabled, Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UsageWindow;

    #[test]
    fn gpt_version_filter_for_codex_prompt_cache() {
        assert!(!is_gpt_5_6_or_newer("gpt-4o"));
        assert!(!is_gpt_5_6_or_newer("gpt-5.4"));
        assert!(!is_gpt_5_6_or_newer("gpt-5.5"));
        assert!(is_gpt_5_6_or_newer("gpt-5.6-sol"));
        assert!(is_gpt_5_6_or_newer("gpt-6.1-sol"));
    }

    #[test]
    fn computes_window_value_estimate_and_31d_activity_from_ledger() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.sqlite");
        let db = database(&db_path).unwrap();

        let now = DateTime::parse_from_rfc3339("2026-10-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let earliest = (now - Duration::hours(10)).timestamp();
        let recent = (now - Duration::hours(1)).timestamp();

        // Insert an earliest event before the 5h window opened, plus a recent event inside the window
        db.execute(
            "INSERT INTO events(path,source,event_id,ts,model,input,output,cache_read,cache_write,partial)
             VALUES ('p1','claude','e0',?1,'claude-sonnet-4',1000,1000,0,0,0)",
            params![earliest],
        )
        .unwrap();
        // 1M input ($3.00) + 100k output ($1.50) + 1M cache_read ($0.30) = $4.80 spent inside the window
        db.execute(
            "INSERT INTO events(path,source,event_id,ts,model,input,output,cache_read,cache_write,partial)
             VALUES ('p1','claude','e1',?1,'claude-sonnet-4',1000000,100000,1000000,0,0)",
            params![recent],
        )
        .unwrap();

        let usage = ProviderUsage {
            account_id: "claude-default".into(),
            provider_id: "claude".into(),
            state: "live".into(),
            checked_at: Some(now.to_rfc3339()),
            windows: vec![UsageWindow {
                id: "five_hour".into(),
                name: "5小时限额".into(),
                used_fraction: 0.24, // 24% used -> $4.80 / 0.24 = $20.00 full window value
                used_percent: 24.0,
                resets_at: Some((now + Duration::hours(2)).to_rfc3339()),
                window_seconds: Some(18000),
                exhausted: false,
            }],
            ..Default::default()
        };

        let detail = compute_card_detail_from_db(&db_path, &usage, true, now);
        let est = detail.window_estimates.get("five_hour").expect("estimate computed");
        assert!((est.used_usd - 4.80).abs() < 1e-6);
        assert!((est.full_usd - 20.0).abs() < 1e-6);

        let act = detail.activity.expect("activity summary computed");
        assert!(act.month_tokens >= 2_100_000);
        assert_eq!(act.days_31.len(), 31);
        assert_eq!(
            act.top_model.as_ref().map(|m| m.name.as_str()),
            Some("claude-sonnet-4")
        );
        assert!(act.cache_hit_rate.is_some());
    }
}
