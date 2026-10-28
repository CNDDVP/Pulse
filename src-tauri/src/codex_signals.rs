use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

const EFFORT_LEVELS: &[&str] = &[
    "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
];
const FEWEST_REACHED: u64 = 20;
const LISTED_CHANGES_LIMIT: usize = 8;
const REPLAY_WINDOW_MS: i64 = 2000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodexModelSignal {
    pub model: String,
    pub responses: u64,
    pub reached_lattice: u64,
    pub on_lattice: u64,
    pub share: Option<f64>,
    pub insufficient_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexSettingChange {
    /// "model" | "effort"
    pub kind: String,
    pub asked: String,
    pub ran: String,
    pub timestamp: String,
    pub session_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexSignalsReport {
    pub span_days: Option<u32>,
    pub scanned_sessions: usize,
    pub judged_sessions: usize,
    pub unjudged_sessions: usize,
    pub models: Vec<CodexModelSignal>,
    pub changes: Vec<CodexSettingChange>,
    pub total_changes: usize,
}

#[derive(Debug, Clone, Default)]
struct ModelCounts {
    responses: u64,
    reached_lattice: u64,
    on_lattice: u64,
}

#[derive(Debug, Clone)]
struct FileFacts {
    size: u64,
    mtime_secs: i64,
    is_judged: bool,
    model_counts: BTreeMap<String, ModelCounts>,
    changes: Vec<CodexSettingChange>,
}

static FILE_CACHE: OnceLock<Mutex<HashMap<PathBuf, FileFacts>>> = OnceLock::new();

pub fn is_on_lattice(reasoning: u64) -> bool {
    reasoning >= 516 && (reasoning + 2) % 518 == 0
}

fn effort_rank(level: &str) -> Option<usize> {
    let lower = level.trim().to_ascii_lowercase();
    EFFORT_LEVELS.iter().position(|&e| e == lower)
}

fn is_auto_review(model: &str) -> bool {
    model.to_ascii_lowercase().contains("auto-review")
}

fn parse_cli_version(v: &str) -> Option<(u32, u32, u32)> {
    let trimmed = v.trim().trim_start_matches('v');
    let mut parts = trimmed.split(['.', '-', '+']);
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch))
}

fn parse_ts_ms(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis())
}

#[derive(Debug, Clone)]
struct RawTurnCheck {
    asked_model: Option<String>,
    asked_effort: Option<String>,
    ran_model: Option<String>,
    ran_effort: Option<String>,
    timestamp: String,
}

fn parse_rollout_reader<R: BufRead>(reader: R, session_title: &str) -> (bool, BTreeMap<String, ModelCounts>, Vec<CodexSettingChange>) {
    let mut cli_version: Option<(u32, u32, u32)> = None;
    let mut not_helper = true;
    let mut applied_any = false;
    let mut replay_until_ms: Option<i64> = None;

    let mut current_model = String::new();
    let mut last_totals: Option<[u64; 4]> = None;
    let mut model_counts: BTreeMap<String, ModelCounts> = BTreeMap::new();

    let mut applied_model: Option<String> = None;
    let mut applied_effort: Option<String> = None;
    let mut at_start_model: Option<String> = None;
    let mut at_start_effort: Option<String> = None;
    let mut prev_model: Option<String> = None;
    let mut prev_effort: Option<String> = None;
    let mut sub_turns: HashSet<String> = HashSet::new();
    let mut raw_turns: Vec<RawTurnCheck> = Vec::new();

    for line_res in reader.lines() {
        let Ok(line) = line_res else { break };
        if line.is_empty() {
            continue;
        }
        // Fast string pre-filter before full JSON deserialization
        if !line.contains("session_meta")
            && !line.contains("turn_context")
            && !line.contains("thread_settings_applied")
            && !line.contains("task_started")
            && !line.contains("token_count")
        {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let top_type = v.get("type").and_then(Value::as_str).unwrap_or("");
        let ts_str = v.get("timestamp").and_then(Value::as_str).unwrap_or("");
        let ts_ms = parse_ts_ms(ts_str);

        if top_type == "session_meta" {
            if let Some(payload) = v.get("payload") {
                if let Some(ver) = payload.get("cli_version").and_then(Value::as_str) {
                    cli_version = parse_cli_version(ver);
                }
                if payload.get("source").is_some_and(Value::is_object)
                    || payload.get("parent_thread_id").is_some_and(Value::is_string)
                {
                    not_helper = false;
                }
                if payload
                    .get("forked_from_id")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty())
                {
                    let base_ms = ts_ms.or_else(|| {
                        payload
                            .get("timestamp")
                            .and_then(Value::as_str)
                            .and_then(parse_ts_ms)
                    });
                    if let Some(b) = base_ms {
                        replay_until_ms = Some(b + REPLAY_WINDOW_MS);
                    }
                }
            }
            continue;
        }

        // Skip forked parent history copied within 2s of the fork header
        if let (Some(limit), Some(cur)) = (replay_until_ms, ts_ms) {
            if cur <= limit {
                continue;
            }
        }

        if top_type == "turn_context" {
            if let Some(payload) = v.get("payload") {
                if let Some(m) = payload.get("model").and_then(Value::as_str) {
                    if !m.trim().is_empty() {
                        current_model = m.trim().to_string();
                    }
                }
                let turn_id = payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !turn_id.is_empty() && sub_turns.contains(turn_id) {
                    continue;
                }
                let ran_m = payload
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let ran_e = payload
                    .get("effort")
                    .or_else(|| payload.pointer("/collaboration_mode/settings/reasoning_effort"))
                    .or_else(|| payload.get("reasoning_effort"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);

                let asked_m = at_start_model.clone().or_else(|| prev_model.clone());
                let asked_e = at_start_effort.clone().or_else(|| prev_effort.clone());

                raw_turns.push(RawTurnCheck {
                    asked_model: asked_m,
                    asked_effort: asked_e,
                    ran_model: ran_m.clone(),
                    ran_effort: ran_e.clone(),
                    timestamp: ts_str.to_string(),
                });

                if ran_m.is_some() {
                    prev_model = ran_m;
                }
                if ran_e.is_some() {
                    prev_effort = ran_e;
                }
            }
            continue;
        }

        if top_type == "event_msg" {
            let Some(payload) = v.get("payload") else {
                continue;
            };
            let ev_type = payload.get("type").and_then(Value::as_str).unwrap_or("");
            match ev_type {
                "thread_settings_applied" => {
                    applied_any = true;
                    let settings = payload.get("thread_settings").unwrap_or(payload);
                    if let Some(m) = settings
                        .get("model")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        applied_model = Some(m.to_string());
                    }
                    if let Some(e) = settings
                        .get("reasoning_effort")
                        .or_else(|| settings.get("effort"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        applied_effort = Some(e.to_string());
                    }
                }
                "task_started" => {
                    let turn_id = payload
                        .get("turn_id")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let root_id = payload
                        .get("root_turn_id")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    if !root_id.is_empty() && !turn_id.is_empty() && root_id != turn_id {
                        sub_turns.insert(turn_id.to_string());
                    } else {
                        if applied_model.is_some() {
                            at_start_model = applied_model.clone();
                        }
                        if applied_effort.is_some() {
                            at_start_effort = applied_effort.clone();
                        }
                    }
                }
                "token_count" => {
                    if current_model.is_empty() || is_auto_review(&current_model) {
                        continue;
                    }
                    let Some(info) = payload.get("info") else {
                        continue;
                    };
                    if let Some(tot) = info.get("total_token_usage") {
                        let totals = [
                            tot.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
                            tot.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
                            tot.get("reasoning_output_tokens")
                                .and_then(Value::as_u64)
                                .unwrap_or(0),
                            tot.get("total_tokens").and_then(Value::as_u64).unwrap_or(0),
                        ];
                        if last_totals == Some(totals) {
                            continue;
                        }
                        last_totals = Some(totals);
                    }
                    let reasoning = info
                        .pointer("/last_token_usage/reasoning_output_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    if reasoning > 0 {
                        let entry = model_counts.entry(current_model.clone()).or_default();
                        entry.responses += 1;
                        if reasoning >= 516 {
                            entry.reached_lattice += 1;
                            if is_on_lattice(reasoning) {
                                entry.on_lattice += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let is_judged = cli_version.is_some_and(|v| v >= (0, 144, 0)) && applied_any && not_helper;
    let mut changes = Vec::new();
    if is_judged {
        for turn in raw_turns {
            if let (Some(asked), Some(ran)) = (&turn.asked_model, &turn.ran_model) {
                if asked != ran && !is_auto_review(asked) && !is_auto_review(ran) {
                    changes.push(CodexSettingChange {
                        kind: "model".into(),
                        asked: asked.clone(),
                        ran: ran.clone(),
                        timestamp: turn.timestamp.clone(),
                        session_title: session_title.to_string(),
                    });
                }
            }
            if let (Some(asked_e), Some(ran_e)) = (&turn.asked_effort, &turn.ran_effort) {
                if let (Some(asked_rank), Some(ran_rank)) = (effort_rank(asked_e), effort_rank(ran_e))
                {
                    if ran_rank < asked_rank {
                        changes.push(CodexSettingChange {
                            kind: "effort".into(),
                            asked: asked_e.clone(),
                            ran: ran_e.clone(),
                            timestamp: turn.timestamp.clone(),
                            session_title: session_title.to_string(),
                        });
                    }
                }
            }
        }
    }

    (is_judged, model_counts, changes)
}

fn collect_rollout_files(root: &Path, since_secs: Option<i64>, seen_names: &mut HashSet<String>, out: &mut Vec<(PathBuf, u64, i64)>) {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 12 || out.len() >= 5000 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                stack.push((path, depth + 1));
            } else if meta.is_file() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if !fname.ends_with(".jsonl") {
                    continue;
                }
                let mtime_secs = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                if let Some(since) = since_secs {
                    if mtime_secs < since {
                        continue;
                    }
                }
                // Deduplicate same rollout file appearing in both `sessions` and `archived_sessions` (#95)
                if !seen_names.insert(fname) {
                    continue;
                }
                out.push((path, meta.len(), mtime_secs));
            }
        }
    }
}

pub fn scan_codex_signals(
    span_days: Option<u32>,
    extra_paths: &[String],
) -> CodexSignalsReport {
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")));

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(home) = codex_home {
        roots.push(home.join("sessions"));
        roots.push(home.join("archived_sessions"));
    }
    for p in extra_paths {
        let pb = PathBuf::from(p);
        if !roots.contains(&pb) {
            roots.push(pb);
        }
    }

    scan_codex_signals_in_roots(&roots, span_days, Utc::now().timestamp())
}

pub fn scan_codex_signals_in_roots(
    roots: &[PathBuf],
    span_days: Option<u32>,
    now_secs: i64,
) -> CodexSignalsReport {
    let since_secs = span_days.map(|d| now_secs - (d as i64) * 86400);
    let mut seen_names = HashSet::new();
    let mut files = Vec::new();
    for root in roots {
        if root.exists() {
            collect_rollout_files(root, since_secs, &mut seen_names, &mut files);
        }
    }

    let cache_mutex = FILE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut total_models: BTreeMap<String, ModelCounts> = BTreeMap::new();
    let mut all_changes: Vec<CodexSettingChange> = Vec::new();
    let mut judged_sessions = 0usize;
    let mut unjudged_sessions = 0usize;

    for (path, size, mtime_secs) in &files {
        let cached = cache_mutex
            .lock()
            .ok()
            .and_then(|g| g.get(path).cloned())
            .filter(|f| f.size == *size && f.mtime_secs == *mtime_secs);

        let facts = if let Some(hit) = cached {
            hit
        } else {
            let Ok(file) = File::open(path) else {
                continue;
            };
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("session")
                .to_string();
            let (is_judged, model_counts, changes) =
                parse_rollout_reader(BufReader::new(file), &title);
            let f = FileFacts {
                size: *size,
                mtime_secs: *mtime_secs,
                is_judged,
                model_counts,
                changes,
            };
            if let Ok(mut g) = cache_mutex.lock() {
                g.insert(path.clone(), f.clone());
            }
            f
        };

        if facts.is_judged {
            judged_sessions += 1;
        } else {
            unjudged_sessions += 1;
        }

        for (model, counts) in facts.model_counts {
            let agg = total_models.entry(model).or_default();
            agg.responses += counts.responses;
            agg.reached_lattice += counts.reached_lattice;
            agg.on_lattice += counts.on_lattice;
        }
        all_changes.extend(facts.changes);
    }

    let mut models: Vec<CodexModelSignal> = total_models
        .into_iter()
        .map(|(model, c)| {
            let insufficient = c.reached_lattice < FEWEST_REACHED;
            let share = if !insufficient && c.reached_lattice > 0 {
                Some((c.on_lattice as f64) / (c.reached_lattice as f64))
            } else {
                None
            };
            CodexModelSignal {
                model,
                responses: c.responses,
                reached_lattice: c.reached_lattice,
                on_lattice: c.on_lattice,
                share,
                insufficient_data: insufficient,
            }
        })
        .collect();
    models.sort_by(|a, b| b.responses.cmp(&a.responses).then_with(|| a.model.cmp(&b.model)));

    all_changes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total_changes = all_changes.len();
    all_changes.truncate(LISTED_CHANGES_LIMIT);

    CodexSignalsReport {
        span_days,
        scanned_sessions: judged_sessions + unjudged_sessions,
        judged_sessions,
        unjudged_sessions,
        models,
        changes: all_changes,
        total_changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_formula_matches_518n_minus_2() {
        assert!(!is_on_lattice(514));
        assert!(is_on_lattice(516)); // 518 * 1 - 2
        assert!(!is_on_lattice(518));
        assert!(is_on_lattice(1034)); // 518 * 2 - 2
        assert!(is_on_lattice(1552)); // 518 * 3 - 2
    }

    #[test]
    fn parses_lattice_stops_skips_fork_replay_and_detects_lowered_settings() {
        let mut lines = Vec::new();
        // Forked session header at 10:00:00.000Z
        lines.push(r#"{"timestamp":"2026-10-25T10:00:00.000Z","type":"session_meta","payload":{"cli_version":"0.144.0","forked_from_id":"parent-123"}}"#.to_string());
        // Copied parent event within 2s (10:00:01.000Z) -> must be skipped!
        lines.push(r#"{"timestamp":"2026-10-25T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"t0","model":"gpt-5.5"}}"#.to_string());
        lines.push(r#"{"timestamp":"2026-10-25T10:00:01.500Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":600,"reasoning_output_tokens":516,"total_tokens":700},"last_token_usage":{"reasoning_output_tokens":516}}}}"#.to_string());

        // Real user settings applied at 10:05:00Z
        lines.push(r#"{"timestamp":"2026-10-25T10:05:00.000Z","type":"event_msg","payload":{"type":"thread_settings_applied","thread_settings":{"model":"gpt-5.6-sol","reasoning_effort":"xhigh"}}}"#.to_string());
        lines.push(r#"{"timestamp":"2026-10-25T10:05:01.000Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1","root_turn_id":"t1"}}"#.to_string());
        // Turn runs with lowered effort ("medium" < "xhigh")
        lines.push(r#"{"timestamp":"2026-10-25T10:05:02.000Z","type":"turn_context","payload":{"turn_id":"t1","model":"gpt-5.6-sol","effort":"medium"}}"#.to_string());

        // Add 20 responses reaching >= 516, with 5 stopping on 516
        for i in 1..=20u64 {
            let r = if i <= 5 { 516 } else { 600 };
            lines.push(format!(
                r#"{{"timestamp":"2026-10-25T10:06:{:02}.000Z","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{},"output_tokens":{},"reasoning_output_tokens":{},"total_tokens":{}}},"last_token_usage":{{"reasoning_output_tokens":{}}}}}}}}}"#,
                i,
                i * 100,
                i * 600,
                i * r,
                i * 700,
                r
            ));
        }

        let input = lines.join("\n");
        let (is_judged, models, changes) =
            parse_rollout_reader(std::io::Cursor::new(input.as_bytes()), "rollout-test");

        assert!(is_judged);
        assert!(!models.contains_key("gpt-5.5"), "fork replay within 2s must be skipped");
        let m = models.get("gpt-5.6-sol").expect("gpt-5.6-sol recorded");
        assert_eq!(m.responses, 20);
        assert_eq!(m.reached_lattice, 20);
        assert_eq!(m.on_lattice, 5);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, "effort");
        assert_eq!(changes[0].asked, "xhigh");
        assert_eq!(changes[0].ran, "medium");
    }
}
