use crate::ledger::{database, estimate_model_cost, frozen_tz, ledger_db_path};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapPeriodInfo {
    /// "day" | "week" | "month" | "year"
    pub kind: String,
    pub key: String,
    pub label: String,
    pub start: String,
    pub end: String,
    pub in_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapDayItem {
    pub day: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub unrecorded: bool,
    pub future: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapMonthItem {
    pub month: String,
    pub label: String,
    pub tokens: u64,
    pub cost_usd: f64,
    pub unrecorded: bool,
    pub future: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapModelItem {
    pub model: String,
    pub tokens: u64,
    pub share: f64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapAgentItem {
    pub source: String,
    pub label: String,
    pub tokens: u64,
    pub share: f64,
    pub active_days: Vec<String>,
    pub active_months: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapProjectItem {
    pub name: String,
    pub tokens: u64,
    pub share: f64,
    pub sessions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapBusiestHours {
    pub start_hour: u32,
    pub end_hour: u32,
    pub share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecapWorkSplit {
    pub weekday_share: f64,
    pub weekend_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapStreakDot {
    pub day: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecapBusiestDay {
    pub day: String,
    pub tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapReport {
    pub period: RecapPeriodInfo,
    pub records_begin: Option<String>,
    pub tokens: u64,
    pub previous_tokens: Option<u64>,
    pub cost_usd: f64,
    pub unpriced_tokens: u64,
    pub partial: bool,
    pub active_days: u64,
    pub observed_days: u64,
    pub total_days: u64,
    pub daily_average_tokens: u64,
    pub sessions: u64,
    pub current_streak: u64,
    pub longest_streak: u64,
    pub streak_chain: Vec<RecapStreakDot>,
    pub busiest_day: Option<RecapBusiestDay>,
    pub days: Vec<RecapDayItem>,
    pub months: Vec<RecapMonthItem>,
    pub hours: Vec<u64>,
    pub peak_hour: Option<u32>,
    pub busiest_hours: Option<RecapBusiestHours>,
    pub first_hour: Option<u32>,
    pub latest_minute: Option<u32>,
    pub late_nights: u64,
    pub persona: String,
    pub work_split: Option<RecapWorkSplit>,
    pub cache_hit_rate: Option<f64>,
    pub cache_savings_usd: f64,
    pub paid_months: f64,
    pub models: Vec<RecapModelItem>,
    pub agents: Vec<RecapAgentItem>,
    pub projects: Vec<RecapProjectItem>,
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let this_month = NaiveDate::from_ymd_opt(year, month, 1);
    match (this_month, next_month) {
        (Some(a), Some(b)) => (b - a).num_days() as u32,
        _ => 30,
    }
}

fn monday_of_week(d: NaiveDate) -> NaiveDate {
    let days_from_mon = d.weekday().num_days_from_monday() as i64;
    d - Duration::days(days_from_mon)
}

fn agent_display_name(src: &str) -> &'static str {
    match src {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "zcode" => "ZCode CLI",
        "gemini" => "Gemini CLI",
        "opencode" => "OpenCode",
        "qwen" => "Qwen CLI",
        "kiro" => "Kiro CLI",
        "cherrystudio" => "Cherry Studio",
        "cline" => "Cline",
        "roocode" => "Roo Code",
        "kilocode" => "Kilo Code",
        "openclaw" => "OpenClaw",
        _ => "AI Agent",
    }
}

fn earliest_recorded_date(db: &rusqlite::Connection, today: NaiveDate) -> NaiveDate {
    let tz = frozen_tz(db);
    let min_archive: Option<String> = db
        .query_row(
            "SELECT MIN(day) FROM daily_archive WHERE (input + output + cache_read + cache_write) > 0",
            [],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let min_ev_ts: Option<i64> = db
        .query_row("SELECT MIN(ts) FROM events", [], |r| r.get(0))
        .ok()
        .flatten();

    let d1 = min_archive
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let d2 = min_ev_ts
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.with_timezone(&tz).date_naive());

    let floor = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let raw = match (d1, d2) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => today - Duration::days(30),
    };
    raw.max(floor).min(today)
}

pub fn default_period_info(kind: &str, now_local_date: NaiveDate, now_local_hour: u32) -> RecapPeriodInfo {
    match kind {
        "day" => {
            let target = if now_local_hour < 5 {
                now_local_date - Duration::days(1)
            } else {
                now_local_date
            };
            make_day_period(target, now_local_date)
        }
        "week" => {
            let cur_mon = monday_of_week(now_local_date);
            let target_mon = if now_local_date.weekday() == Weekday::Mon {
                cur_mon - Duration::days(7)
            } else {
                cur_mon
            };
            make_week_period(target_mon, now_local_date)
        }
        "year" => {
            let target_year = if now_local_date.month() == 1 && now_local_date.day() <= 7 {
                now_local_date.year() - 1
            } else {
                now_local_date.year()
            };
            make_year_period(target_year, now_local_date)
        }
        _ => {
            // "month" default: previous month on days 1..=7, current month from day 8 onward
            let (y, m) = if now_local_date.day() <= 7 {
                if now_local_date.month() == 1 {
                    (now_local_date.year() - 1, 12)
                } else {
                    (now_local_date.year(), now_local_date.month() - 1)
                }
            } else {
                (now_local_date.year(), now_local_date.month())
            };
            make_month_period(y, m, now_local_date)
        }
    }
}

fn make_day_period(d: NaiveDate, today: NaiveDate) -> RecapPeriodInfo {
    let s = d.format("%Y-%m-%d").to_string();
    RecapPeriodInfo {
        kind: "day".into(),
        key: s.clone(),
        label: s.clone(),
        start: s.clone(),
        end: s,
        in_progress: d == today,
    }
}

fn make_week_period(mon: NaiveDate, today: NaiveDate) -> RecapPeriodInfo {
    let sun = mon + Duration::days(6);
    let start = mon.format("%Y-%m-%d").to_string();
    let end = sun.format("%Y-%m-%d").to_string();
    RecapPeriodInfo {
        kind: "week".into(),
        key: format!("week-{start}"),
        label: format!("{} ~ {}", mon.format("%Y-%m-%d"), sun.format("%m-%d")),
        start,
        end,
        in_progress: today >= mon && today <= sun,
    }
}

fn make_month_period(year: i32, month: u32, today: NaiveDate) -> RecapPeriodInfo {
    let dim = days_in_month(year, month);
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let last = NaiveDate::from_ymd_opt(year, month, dim).unwrap();
    let key = format!("{year:04}-{month:02}");
    RecapPeriodInfo {
        kind: "month".into(),
        key: key.clone(),
        label: key,
        start: first.format("%Y-%m-%d").to_string(),
        end: last.format("%Y-%m-%d").to_string(),
        in_progress: today >= first && today <= last,
    }
}

fn make_year_period(year: i32, today: NaiveDate) -> RecapPeriodInfo {
    let first = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let last = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
    let key = format!("{year:04}");
    RecapPeriodInfo {
        kind: "year".into(),
        key: key.clone(),
        label: key,
        start: first.format("%Y-%m-%d").to_string(),
        end: last.format("%Y-%m-%d").to_string(),
        in_progress: today >= first && today <= last,
    }
}

pub fn resolve_period(
    kind: &str,
    key: Option<&str>,
    today: NaiveDate,
    now_hour: u32,
) -> RecapPeriodInfo {
    let Some(k) = key.map(str::trim).filter(|s| !s.is_empty()) else {
        return default_period_info(kind, today, now_hour);
    };
    match kind {
        "day" => {
            if let Ok(d) = NaiveDate::parse_from_str(k, "%Y-%m-%d") {
                return make_day_period(d, today);
            }
        }
        "week" => {
            let stripped = k.strip_prefix("week-").unwrap_or(k);
            if let Ok(d) = NaiveDate::parse_from_str(stripped, "%Y-%m-%d") {
                return make_week_period(monday_of_week(d), today);
            }
        }
        "month" => {
            let mut parts = k.split('-');
            if let (Some(y), Some(m)) = (
                parts.next().and_then(|s| s.parse::<i32>().ok()),
                parts.next().and_then(|s| s.parse::<u32>().ok()),
            ) {
                if (1..=12).contains(&m) {
                    return make_month_period(y, m, today);
                }
            }
        }
        "year" => {
            if let Ok(y) = k.parse::<i32>() {
                if (2020..=2100).contains(&y) {
                    return make_year_period(y, today);
                }
            }
        }
        _ => {}
    }
    default_period_info(kind, today, now_hour)
}

pub fn list_periods_from_db(
    db_path: &Path,
    kind: &str,
    now: DateTime<Utc>,
) -> Result<Vec<RecapPeriodInfo>, String> {
    let db = database(db_path)?;
    let tz = frozen_tz(&db);
    let now_local = now.with_timezone(&tz);
    let today = now_local.date_naive();
    let earliest = earliest_recorded_date(&db, today);

    let mut list = Vec::new();
    match kind {
        "day" => {
            let mut cur = today;
            let stop = earliest.max(today - Duration::days(90));
            while cur >= stop {
                list.push(make_day_period(cur, today));
                cur -= Duration::days(1);
            }
        }
        "week" => {
            let mut cur_mon = monday_of_week(today);
            let stop_mon = monday_of_week(earliest).max(cur_mon - Duration::days(7 * 26));
            while cur_mon >= stop_mon {
                list.push(make_week_period(cur_mon, today));
                cur_mon -= Duration::days(7);
            }
        }
        "year" => {
            for y in (earliest.year()..=today.year()).rev() {
                list.push(make_year_period(y, today));
            }
        }
        _ => {
            let mut y = today.year();
            let mut m = today.month();
            for _ in 0..24 {
                list.push(make_month_period(y, m, today));
                if y < earliest.year() || (y == earliest.year() && m <= earliest.month()) {
                    break;
                }
                if m == 1 {
                    y -= 1;
                    m = 12;
                } else {
                    m -= 1;
                }
            }
        }
    }
    Ok(list)
}

pub fn build_recap_from_db(
    db_path: &Path,
    kind: &str,
    key: Option<&str>,
    now: DateTime<Utc>,
) -> Result<RecapReport, String> {
    let db = database(db_path)?;
    let tz = frozen_tz(&db);
    let now_local = now.with_timezone(&tz);
    let today = now_local.date_naive();
    let period = resolve_period(kind, key, today, now_local.hour());

    let start_date = NaiveDate::parse_from_str(&period.start, "%Y-%m-%d")
        .map_err(|_| "无效的周期起始日期")?;
    let end_date =
        NaiveDate::parse_from_str(&period.end, "%Y-%m-%d").map_err(|_| "无效的周期结束日期")?;

    // Merge all daily_archive + events across all history so we can compute records_begin & previous period
    let mut merged: BTreeMap<(String, String, String), [u64; 4]> = BTreeMap::new();
    {
        let mut s1 = db
            .prepare("SELECT day, source, model, input, output, cache_read, cache_write FROM daily_archive")
            .map_err(|_| "读取归档失败")?;
        let rows = s1
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    [
                        r.get::<_, u64>(3)?,
                        r.get::<_, u64>(4)?,
                        r.get::<_, u64>(5)?,
                        r.get::<_, u64>(6)?,
                    ],
                ))
            })
            .map_err(|_| "读取归档失败")?;
        for (day, src, model, c) in rows.flatten() {
            let e = merged.entry((day, src, model)).or_insert([0; 4]);
            for i in 0..4 {
                e[i] = e[i].max(c[i]);
            }
        }
    }

    let mut ev_by_day: BTreeMap<(String, String, String), [u64; 4]> = BTreeMap::new();
    let mut partial_any = false;
    let mut period_event_ts: Vec<(i64, u64)> = Vec::new();
    let mut project_buckets: BTreeMap<String, (u64, BTreeSet<String>)> = BTreeMap::new();
    let mut period_sessions: BTreeSet<String> = BTreeSet::new();

    // Load session titles for project attribution
    let mut title_map: BTreeMap<String, String> = BTreeMap::new();
    if let Ok(mut ts_stmt) = db.prepare("SELECT path, title FROM session_titles") {
        if let Ok(rows) = ts_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
            for (p, t) in rows.flatten() {
                title_map.insert(p, t);
            }
        }
    }

    {
        let mut s2 = db
            .prepare(
                "SELECT path, source, model, ts, MAX(input), MAX(output), MAX(cache_read), MAX(cache_write), MAX(partial)
                 FROM events
                 GROUP BY source, event_id",
            )
            .map_err(|_| "读取事件失败")?;
        let rows = s2
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    [
                        r.get::<_, u64>(4)?,
                        r.get::<_, u64>(5)?,
                        r.get::<_, u64>(6)?,
                        r.get::<_, u64>(7)?,
                    ],
                    r.get::<_, i64>(8).unwrap_or(0),
                ))
            })
            .map_err(|_| "读取事件失败")?;

        for (path, src, model, ts, c, part) in rows.flatten() {
            let Some(dt) = DateTime::from_timestamp(ts, 0).map(|d| d.with_timezone(&tz)) else {
                continue;
            };
            let day = dt.format("%Y-%m-%d").to_string();
            let e = ev_by_day.entry((day.clone(), src.clone(), model)).or_insert([0; 4]);
            for i in 0..4 {
                e[i] += c[i];
            }
            if day.as_str() >= period.start.as_str() && day.as_str() <= period.end.as_str() {
                if part != 0 {
                    partial_any = true;
                }
                let tok: u64 = c.iter().sum();
                if tok > 0 {
                    period_event_ts.push((ts, tok));
                    period_sessions.insert(format!("{src}:{path}"));
                    let proj_title = title_map
                        .get(&path)
                        .cloned()
                        .unwrap_or_else(|| agent_display_name(&src).to_string());
                    let entry = project_buckets
                        .entry(proj_title)
                        .or_insert_with(|| (0, BTreeSet::new()));
                    entry.0 += tok;
                    entry.1.insert(path);
                }
            }
        }
    }

    for (k, c) in ev_by_day {
        let e = merged.entry(k).or_insert([0; 4]);
        for i in 0..4 {
            e[i] = e[i].max(c[i]);
        }
    }

    // Earliest recorded day across all merged buckets
    let first_record_day = merged
        .iter()
        .filter(|(_, c)| c.iter().sum::<u64>() > 0)
        .map(|((d, _, _), _)| d.as_str())
        .min()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let records_begin_date = first_record_day.filter(|&d| d > start_date && d <= end_date);
    let records_begin = records_begin_date.map(|d| d.format("%Y-%m-%d").to_string());

    // Aggregate per-day totals across entire history for streak & previous period
    let mut history_day_tokens: BTreeMap<String, u64> = BTreeMap::new();
    for ((day, _, _), c) in &merged {
        let tok: u64 = c.iter().sum();
        if tok > 0 {
            *history_day_tokens.entry(day.clone()).or_insert(0) += tok;
        }
    }

    // Previous period comparison (same elapsed days)
    let previous_tokens = if period.kind == "day" && period.in_progress {
        None
    } else {
        let effective_end = end_date.min(today);
        let elapsed_days = (effective_end - start_date).num_days().max(0) + 1;
        let prev_start = match period.kind.as_str() {
            "day" => start_date - Duration::days(1),
            "week" => start_date - Duration::days(7),
            "year" => NaiveDate::from_ymd_opt(start_date.year() - 1, 1, 1).unwrap_or(start_date),
            _ => {
                if start_date.month() == 1 {
                    NaiveDate::from_ymd_opt(start_date.year() - 1, 12, 1).unwrap_or(start_date)
                } else {
                    NaiveDate::from_ymd_opt(start_date.year(), start_date.month() - 1, 1)
                        .unwrap_or(start_date)
                }
            }
        };
        let mut prev_sum = 0u64;
        let mut any_prev_observed = false;
        for i in 0..elapsed_days {
            let d = prev_start + Duration::days(i);
            if first_record_day.is_none_or(|first| d >= first) {
                any_prev_observed = true;
                let k = d.format("%Y-%m-%d").to_string();
                prev_sum += history_day_tokens.get(&k).copied().unwrap_or(0);
            }
        }
        if any_prev_observed && prev_sum > 0 {
            Some(prev_sum)
        } else {
            None
        }
    };

    // Period aggregations
    let mut total_tokens = 0u64;
    let mut total_cost_usd = 0.0;
    let mut unpriced_tokens = 0u64;
    let mut cache_tally = [0u64; 4];
    let mut cache_savings_usd = 0.0;

    let mut per_day_tok_cost: BTreeMap<String, (u64, f64)> = BTreeMap::new();
    let mut per_model: BTreeMap<String, (u64, f64)> = BTreeMap::new();
    let mut per_agent: BTreeMap<String, (u64, BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();

    for ((day, src, model), c) in &merged {
        if day.as_str() < period.start.as_str() || day.as_str() > period.end.as_str() {
            continue;
        }
        let tok: u64 = c.iter().sum();
        if tok == 0 {
            continue;
        }
        total_tokens += tok;
        let cost = if let Some(val) = estimate_model_cost(model, c) {
            total_cost_usd += val;
            let without_cache_counts = [c[0] + c[2] + c[3], c[1], 0, 0];
            if let Some(no_cache_val) = estimate_model_cost(model, &without_cache_counts) {
                if no_cache_val > val {
                    cache_savings_usd += no_cache_val - val;
                }
            }
            val
        } else {
            unpriced_tokens += tok;
            0.0
        };

        if c[2] > 0 || c[3] > 0 {
            for i in 0..4 {
                cache_tally[i] += c[i];
            }
        }

        let d_entry = per_day_tok_cost.entry(day.clone()).or_insert((0, 0.0));
        d_entry.0 += tok;
        d_entry.1 += cost;

        if !model.is_empty() && model != "unknown" {
            let m_entry = per_model.entry(model.clone()).or_insert((0, 0.0));
            m_entry.0 += tok;
            m_entry.1 += cost;
        }

        let a_entry = per_agent
            .entry(src.clone())
            .or_insert_with(|| (0, BTreeSet::new(), BTreeSet::new()));
        a_entry.0 += tok;
        a_entry.1.insert(day.clone());
        if day.len() >= 7 {
            a_entry.2.insert(day[..7].to_string());
        }
    }

    let total_days = ((end_date - start_date).num_days().max(0) + 1) as u64;
    let mut days = Vec::with_capacity(total_days as usize);
    let mut observed_days = 0u64;
    let mut active_days = 0u64;
    let mut paid_months = 0.0;
    let mut busiest_day: Option<RecapBusiestDay> = None;
    let mut weekday_tokens = 0u64;
    let mut weekend_tokens = 0u64;

    let mut cur = start_date;
    while cur <= end_date {
        let key_str = cur.format("%Y-%m-%d").to_string();
        let future = cur > today;
        let unrecorded = !future && first_record_day.is_some_and(|f| cur < f);
        let (tok, cost) = per_day_tok_cost.get(&key_str).copied().unwrap_or((0, 0.0));

        if !future && !unrecorded {
            observed_days += 1;
            let dim = days_in_month(cur.year(), cur.month()) as f64;
            paid_months += 1.0 / dim;
        }
        if tok > 0 {
            active_days += 1;
            if busiest_day.as_ref().is_none_or(|b| tok > b.tokens) {
                busiest_day = Some(RecapBusiestDay {
                    day: key_str.clone(),
                    tokens: tok,
                });
            }
            match cur.weekday() {
                Weekday::Sat | Weekday::Sun => weekend_tokens += tok,
                _ => weekday_tokens += tok,
            }
        }

        days.push(RecapDayItem {
            day: key_str,
            tokens: tok,
            cost_usd: cost,
            unrecorded,
            future,
        });
        cur += Duration::days(1);
    }

    let daily_average_tokens = if observed_days > 0 {
        total_tokens / observed_days
    } else {
        0
    };

    // Streaks within observed days of the period
    let mut longest_streak = 0u64;
    let mut running = 0u64;
    for d in &days {
        if d.future || d.unrecorded {
            continue;
        }
        if d.tokens > 0 {
            running += 1;
            longest_streak = longest_streak.max(running);
        } else {
            running = 0;
        }
    }
    let current_streak = running;

    let observed_slice: Vec<&RecapDayItem> =
        days.iter().filter(|d| !d.future && !d.unrecorded).collect();
    let chain_start = observed_slice.len().saturating_sub(9);
    let streak_chain: Vec<RecapStreakDot> = observed_slice[chain_start..]
        .iter()
        .map(|d| RecapStreakDot {
            day: d.day.clone(),
            active: d.tokens > 0,
        })
        .collect();

    // Months array (for year view)
    let mut months = Vec::new();
    if period.kind == "year" {
        let y = start_date.year();
        for m in 1..=12u32 {
            let prefix = format!("{y:04}-{m:02}");
            let m_first = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
            let m_last = NaiveDate::from_ymd_opt(y, m, days_in_month(y, m)).unwrap();
            let future = m_first > today;
            let unrecorded = !future && first_record_day.is_some_and(|f| m_last < f);
            let mut m_tok = 0u64;
            let mut m_cost = 0.0;
            for d in &days {
                if d.day.starts_with(&prefix) {
                    m_tok += d.tokens;
                    m_cost += d.cost_usd;
                }
            }
            months.push(RecapMonthItem {
                month: prefix,
                label: format!("{m}月"),
                tokens: m_tok,
                cost_usd: m_cost,
                unrecorded,
                future,
            });
        }
    }

    // Hourly distribution, Workday stretches, Late nights & Persona
    let mut hours = vec![0u64; 24];
    period_event_ts.sort_by_key(|(ts, _)| *ts);
    let mut first_hour: Option<u32> = None;
    let mut latest_minute: Option<u32> = None;
    let mut late_nights = 0u64;

    // Group events into workday stretches separated by > 3 hours
    let mut stretches: Vec<(DateTime<chrono::FixedOffset>, DateTime<chrono::FixedOffset>)> = Vec::new();
    for &(ts, tok) in &period_event_ts {
        let Some(dt) = DateTime::from_timestamp(ts, 0).map(|d| d.with_timezone(&tz)) else {
            continue;
        };
        let h = dt.hour() as usize;
        if h < 24 {
            hours[h] += tok;
        }
        if let Some(last) = stretches.last_mut() {
            let gap = (dt - last.1).num_seconds();
            if gap <= 3 * 3600 && (dt - last.0).num_seconds() <= 20 * 3600 {
                last.1 = dt;
                continue;
            }
        }
        stretches.push((dt, dt));
    }

    for (start_dt, end_dt) in stretches {
        let start_h = start_dt.hour();
        if start_h >= 5 {
            first_hour = Some(first_hour.map_or(start_h, |prev| prev.min(start_h)));
        }
        let owner_date = if start_h < 5 {
            start_dt.date_naive() - Duration::days(1)
        } else {
            start_dt.date_naive()
        };
        let end_date_local = end_dt.date_naive();
        let day_diff = (end_date_local - owner_date).num_days().max(0) as u32;
        if day_diff >= 1 {
            late_nights += 1;
        }
        let mins_from_owner_midnight = day_diff * 1440 + end_dt.hour() * 60 + end_dt.minute();
        latest_minute = Some(latest_minute.map_or(mins_from_owner_midnight, |prev| {
            prev.max(mins_from_owner_midnight)
        }));
    }

    let timed_total: u64 = hours.iter().sum();
    let peak_hour = if timed_total > 0 {
        hours
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v)
            .map(|(i, _)| i as u32)
    } else {
        None
    };

    let busiest_hours = if timed_total > 0 {
        let mut best_start = 0u32;
        let mut best_sum = 0u64;
        for s in 0..24u32 {
            let sum: u64 = (0..4).map(|off| hours[((s + off) % 24) as usize]).sum();
            if sum > best_sum {
                best_sum = sum;
                best_start = s;
            }
        }
        Some(RecapBusiestHours {
            start_hour: best_start,
            end_hour: (best_start + 4) % 24,
            share: (best_sum as f64) / (timed_total as f64),
        })
    } else {
        None
    };

    let persona = if timed_total > 0 {
        let night_tok: u64 = (21..24).chain(0..5).map(|h| hours[h]).sum();
        let early_tok: u64 = (5..10).map(|h| hours[h]).sum();
        let day_tok: u64 = (10..18).map(|h| hours[h]).sum();
        if (night_tok as f64) / (timed_total as f64) > 0.5 {
            "night_owl"
        } else if (early_tok as f64) / (timed_total as f64) > 0.5 {
            "early_bird"
        } else if (day_tok as f64) / (timed_total as f64) > 0.5 {
            "day_shift"
        } else {
            "all_day"
        }
    } else {
        "day_shift"
    }
    .to_string();

    let work_split = if weekday_tokens + weekend_tokens > 0 {
        let tot = (weekday_tokens + weekend_tokens) as f64;
        Some(RecapWorkSplit {
            weekday_share: (weekday_tokens as f64) / tot,
            weekend_share: (weekend_tokens as f64) / tot,
        })
    } else {
        None
    };

    let cache_denom = cache_tally[0] + cache_tally[2] + cache_tally[3];
    let cache_hit_rate = if cache_denom > 0 && cache_tally[2] > 0 {
        Some((cache_tally[2] as f64) / (cache_denom as f64))
    } else {
        None
    };

    let mut models: Vec<RecapModelItem> = per_model
        .into_iter()
        .map(|(model, (tok, cost))| RecapModelItem {
            model,
            tokens: tok,
            share: if total_tokens > 0 {
                (tok as f64) / (total_tokens as f64)
            } else {
                0.0
            },
            cost_usd: cost,
        })
        .collect();
    models.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    let mut agents: Vec<RecapAgentItem> = per_agent
        .into_iter()
        .map(|(source, (tok, act_days, act_months))| RecapAgentItem {
            label: agent_display_name(&source).to_string(),
            source,
            tokens: tok,
            share: if total_tokens > 0 {
                (tok as f64) / (total_tokens as f64)
            } else {
                0.0
            },
            active_days: act_days.into_iter().collect(),
            active_months: act_months.into_iter().collect(),
        })
        .collect();
    agents.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    let mut projects: Vec<RecapProjectItem> = project_buckets
        .into_iter()
        .map(|(name, (tok, sess))| RecapProjectItem {
            name,
            tokens: tok,
            share: if total_tokens > 0 {
                (tok as f64) / (total_tokens as f64)
            } else {
                0.0
            },
            sessions: sess.len() as u64,
        })
        .collect();
    projects.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    Ok(RecapReport {
        period,
        records_begin,
        tokens: total_tokens,
        previous_tokens,
        cost_usd: total_cost_usd,
        unpriced_tokens,
        partial: partial_any,
        active_days,
        observed_days,
        total_days,
        daily_average_tokens,
        sessions: period_sessions.len() as u64,
        current_streak,
        longest_streak,
        streak_chain,
        busiest_day,
        days,
        months,
        hours,
        peak_hour,
        busiest_hours,
        first_hour,
        latest_minute,
        late_nights,
        persona,
        work_split,
        cache_hit_rate,
        cache_savings_usd,
        paid_months,
        models,
        agents,
        projects,
    })
}

pub fn list_recap_periods(kind: &str) -> Result<Vec<RecapPeriodInfo>, String> {
    list_periods_from_db(&ledger_db_path(), kind, Utc::now())
}

pub fn build_recap(kind: &str, key: Option<&str>) -> Result<RecapReport, String> {
    build_recap_from_db(&ledger_db_path(), kind, key, Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn default_period_follows_upstream_cutoff_rules() {
        // Day 5 of October -> defaults to September
        let oct5 = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let m1 = default_period_info("month", oct5, 14);
        assert_eq!(m1.key, "2026-09");
        assert!(!m1.in_progress);

        // Day 10 of October -> defaults to October (in progress)
        let oct10 = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let m2 = default_period_info("month", oct10, 14);
        assert_eq!(m2.key, "2026-10");
        assert!(m2.in_progress);

        // Before 05:00 local time -> day recap defaults to yesterday
        let d_early = default_period_info("day", oct10, 3);
        assert_eq!(d_early.key, "2026-10-09");
    }

    #[test]
    fn builds_recap_report_with_persona_cache_savings_and_payback_proration() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("recap.sqlite");
        let db = database(&db_path).unwrap();

        let now = DateTime::parse_from_rfc3339("2026-10-20T15:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let ts1 = DateTime::parse_from_rfc3339("2026-10-15T14:00:00Z")
            .unwrap()
            .timestamp();
        let ts2 = DateTime::parse_from_rfc3339("2026-10-16T15:00:00Z")
            .unwrap()
            .timestamp();

        db.execute(
            "INSERT INTO session_titles(path, title) VALUES ('p1', 'pulse-windows')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO events(path,source,event_id,ts,model,input,output,cache_read,cache_write,partial)
             VALUES ('p1','claude','e1',?1,'claude-sonnet-4',500000,100000,2000000,0,0)",
            params![ts1],
        )
        .unwrap();
        db.execute(
            "INSERT INTO events(path,source,event_id,ts,model,input,output,cache_read,cache_write,partial)
             VALUES ('p1','codex','e2',?1,'gpt-5.6-sol',400000,100000,1000000,0,0)",
            params![ts2],
        )
        .unwrap();

        let report = build_recap_from_db(&db_path, "month", Some("2026-10"), now).unwrap();
        assert_eq!(report.period.key, "2026-10");
        assert_eq!(report.active_days, 2);
        assert_eq!(report.tokens, 4_100_000);
        assert!(report.cost_usd > 0.0);
        assert!(report.cache_savings_usd > 1.0);
        assert_eq!(report.models.len(), 2);
        assert_eq!(report.agents.len(), 2);
        assert_eq!(report.projects[0].name, "pulse-windows");
    }
}
