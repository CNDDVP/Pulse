use crate::types::{AppSettings, ProviderUsage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

pub const SOURCE_URL: &str = "https://aihot.news/api/v1/codex-resets";
pub const POLL_INTERVAL_SECS: i64 = 15 * 60;
pub const STALE_THRESHOLD_SECS: i64 = 30 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodexResetKind {
    #[serde(rename = "direct_reset")]
    DirectReset,
    #[serde(rename = "reset_credit")]
    ResetCredit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodexResetStatus {
    #[serde(rename = "announced")]
    Announced,
    #[serde(rename = "confirmed")]
    Confirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodexResetPrecision {
    #[serde(rename = "exact")]
    Exact,
    #[serde(rename = "approximate")]
    Approximate,
    #[serde(rename = "deadline")]
    Deadline,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "window")]
    Window,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexResetSchedule {
    pub precision: CodexResetPrecision,
    pub from: String,
    pub through: String,
    pub label: String,
}

impl CodexResetSchedule {
    pub fn from_dt(&self) -> Option<DateTime<Utc>> {
        parse_iso8601(&self.from)
    }
    pub fn through_dt(&self) -> Option<DateTime<Utc>> {
        parse_iso8601(&self.through)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexResetPost {
    #[serde(alias = "publishedAt")]
    pub published_at: String,
    pub text: String,
    #[serde(default, alias = "originalText")]
    pub original_text: Option<String>,
    #[serde(default, alias = "fullOriginalText")]
    pub full_original_text: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexResetEvent {
    pub id: String,
    #[serde(alias = "type")]
    pub kind: CodexResetKind,
    pub status: CodexResetStatus,
    pub title: String,
    pub scope: String,
    #[serde(alias = "createdAt")]
    pub created_at: String,
    #[serde(alias = "updatedAt")]
    pub updated_at: String,
    #[serde(default, alias = "confirmedAt")]
    pub confirmed_at: Option<String>,
    #[serde(default)]
    pub schedule: Option<CodexResetSchedule>,
    #[serde(default)]
    pub posts: Vec<CodexResetPost>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexResetSnapshot {
    pub events: Vec<CodexResetEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CodexResetHealth {
    pub last_succeeded_at: Option<String>,
    pub last_attempt_failed: bool,
    pub is_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexResetIncrease {
    pub event_id: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CodexResetState {
    pub event: Option<CodexResetEvent>,
    pub health: CodexResetHealth,
    pub increases: BTreeMap<String, CodexResetIncrease>,
}

pub fn parse_iso8601(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

impl CodexResetEvent {
    pub fn created_at_dt(&self) -> Option<DateTime<Utc>> {
        parse_iso8601(&self.created_at)
    }

    pub fn updated_at_dt(&self) -> Option<DateTime<Utc>> {
        parse_iso8601(&self.updated_at)
    }

    pub fn confirmed_at_dt(&self) -> Option<DateTime<Utc>> {
        self.confirmed_at.as_deref().and_then(parse_iso8601)
    }

    fn validate_dates(&self) -> Result<(), String> {
        if self.created_at_dt().is_none() {
            return Err(format!("invalid createdAt: {}", self.created_at));
        }
        if self.updated_at_dt().is_none() {
            return Err(format!("invalid updatedAt: {}", self.updated_at));
        }
        if let Some(ref c) = self.confirmed_at {
            if parse_iso8601(c).is_none() {
                return Err(format!("invalid confirmedAt: {c}"));
            }
        }
        if let Some(ref s) = self.schedule {
            if s.from_dt().is_none() || s.through_dt().is_none() {
                return Err("invalid schedule dates".into());
            }
        }
        for p in &self.posts {
            if parse_iso8601(&p.published_at).is_none() {
                return Err(format!("invalid post publishedAt: {}", p.published_at));
            }
        }
        Ok(())
    }

    /// Old unconfirmed announcements must not remain on the ring forever.
    /// Scheduled ones remain useful only through the announced window;
    /// schedule-less news gets 48 hours. Confirmed events remain visible for 24 hours.
    pub fn is_displayable(&self, now: DateTime<Utc>) -> bool {
        let Some(updated) = self.updated_at_dt() else {
            return false;
        };
        match self.status {
            CodexResetStatus::Confirmed => {
                let anchor = self.confirmed_at_dt().unwrap_or(updated);
                (now - anchor).num_milliseconds() <= 24 * 60 * 60 * 1000
            }
            CodexResetStatus::Announced => {
                if let Some(ref sched) = self.schedule {
                    let Some(through) = sched.through_dt() else {
                        return false;
                    };
                    now <= through
                } else {
                    (now - updated).num_milliseconds() <= 48 * 60 * 60 * 1000
                }
            }
        }
    }

    fn selection_rank(&self, now: DateTime<Utc>) -> (i32, i64) {
        let updated_ms = self
            .updated_at_dt()
            .map(|d| d.timestamp_millis())
            .unwrap_or(0);
        if self.status == CodexResetStatus::Confirmed {
            let confirmed_ms = self
                .confirmed_at_dt()
                .map(|d| d.timestamp_millis())
                .unwrap_or(updated_ms);
            return (6, confirmed_ms);
        }
        let Some(ref sched) = self.schedule else {
            return (3, updated_ms);
        };
        let (Some(from), Some(through)) = (sched.from_dt(), sched.through_dt()) else {
            return (3, updated_ms);
        };
        if now < from {
            (4, -from.timestamp_millis())
        } else if now <= through {
            (5, through.timestamp_millis())
        } else {
            (3, through.timestamp_millis())
        }
    }

    pub fn current(events: &[Self], now: DateTime<Utc>) -> Option<Self> {
        let mut candidates: Vec<&Self> =
            events.iter().filter(|e| e.is_displayable(now)).collect();
        candidates.sort_by(|lhs, rhs| {
            let left = lhs.selection_rank(now);
            let right = rhs.selection_rank(now);
            right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1))
        });
        candidates.first().copied().cloned()
    }
}

pub fn decode_snapshot(bytes: &[u8]) -> Result<CodexResetSnapshot, String> {
    let snap: CodexResetSnapshot =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid snapshot json: {e}"))?;
    for ev in &snap.events {
        ev.validate_dates()?;
    }
    Ok(snap)
}

pub fn parse_retry_after(header_val: Option<&str>, now: DateTime<Utc>) -> DateTime<Utc> {
    let default_next = now + chrono::Duration::seconds(POLL_INTERVAL_SECS);
    let Some(raw) = header_val.map(str::trim).filter(|s| !s.is_empty()) else {
        return default_next;
    };
    if let Ok(secs) = raw.parse::<f64>() {
        if secs.is_finite() && secs >= 0.0 {
            let clamped = (secs.round() as i64).clamp(1, 86400);
            return now + chrono::Duration::seconds(clamped);
        }
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(raw) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%a, %d %b %Y %H:%M:%S GMT") {
        return naive.and_utc();
    }
    default_next
}

#[derive(Debug, Clone)]
struct RecordedWindow {
    id: String,
    window_seconds: i64,
    resets_at: DateTime<Utc>,
    used_fraction: f64,
}

#[derive(Debug, Clone)]
struct RecordedReading {
    fingerprint: String,
    plan_name: Option<String>,
    observed_at: DateTime<Utc>,
    windows: Vec<RecordedWindow>,
}

#[derive(Debug, Default)]
pub struct CodexResetObservation {
    previous: HashMap<String, RecordedReading>,
    increases: HashMap<String, CodexResetIncrease>,
}

impl CodexResetObservation {
    pub fn clear(&mut self) {
        self.previous.clear();
        self.increases.clear();
    }

    pub fn observe(
        &mut self,
        reading: &ProviderUsage,
        event: Option<&CodexResetEvent>,
        now: DateTime<Utc>,
    ) {
        if reading.provider_id != "codex" {
            return;
        }
        let id = reading.account_id.clone();
        let stamp = reading.checked_at.as_deref().and_then(parse_iso8601);
        let fp = reading
            .codex_account_fingerprint
            .as_deref()
            .filter(|s| !s.is_empty());

        let (Some(stamp), Some(fp)) = (stamp, fp) else {
            self.previous.remove(&id);
            self.increases.remove(&id);
            return;
        };

        if reading.state != "live"
            || stamp > now
            || (now - stamp).num_seconds() > 180
        {
            self.previous.remove(&id);
            self.increases.remove(&id);
            return;
        }

        let current_windows: Vec<RecordedWindow> = reading
            .windows
            .iter()
            .filter_map(|w| {
                // Only account-wide 5-hour (18000s) or weekly (604800s) windows qualify.
                if !w.id.starts_with("account-") {
                    return None;
                }
                let secs = w.window_seconds?;
                if secs != 18000 && secs != 604800 {
                    return None;
                }
                if !w.used_fraction.is_finite() || !(0.0..=1.0).contains(&w.used_fraction) {
                    return None;
                }
                let resets_at = w.resets_at.as_deref().and_then(parse_iso8601)?;
                Some(RecordedWindow {
                    id: w.id.clone(),
                    window_seconds: secs,
                    resets_at,
                    used_fraction: w.used_fraction,
                })
            })
            .collect();

        let prev = self.previous.insert(
            id.clone(),
            RecordedReading {
                fingerprint: fp.to_string(),
                plan_name: reading.plan_name.clone(),
                observed_at: stamp,
                windows: current_windows.clone(),
            },
        );

        let Some(old) = prev else {
            return;
        };

        if old.fingerprint != fp
            || old.plan_name != reading.plan_name
            || stamp <= old.observed_at
            || (stamp - old.observed_at).num_seconds() > 30 * 60
        {
            self.increases.remove(&id);
            return;
        }

        let Some(ev) = event else {
            return;
        };
        let Some(ev_created) = ev.created_at_dt() else {
            return;
        };
        if ev.kind != CodexResetKind::DirectReset
            || ev.status != CodexResetStatus::Announced
            || !ev.is_displayable(now)
            || stamp < ev_created
        {
            return;
        }

        let mut recovered = false;
        for w in &current_windows {
            if let Some(before) = old
                .windows
                .iter()
                .find(|b| b.id == w.id && b.window_seconds == w.window_seconds)
            {
                if w.resets_at > stamp
                    && before.resets_at > stamp
                    && before.used_fraction - w.used_fraction >= 0.01 - 1e-9
                {
                    recovered = true;
                    break;
                }
            }
        }

        if recovered {
            self.increases.insert(
                id,
                CodexResetIncrease {
                    event_id: ev.id.clone(),
                    detected_at: stamp.to_rfc3339(),
                },
            );
        }
    }

    pub fn active_increases(
        &self,
        event: Option<&CodexResetEvent>,
        now: DateTime<Utc>,
    ) -> BTreeMap<String, CodexResetIncrease> {
        let mut out = BTreeMap::new();
        let Some(ev) = event else {
            return out;
        };
        if ev.kind != CodexResetKind::DirectReset || ev.status != CodexResetStatus::Announced {
            return out;
        }
        for (aid, inc) in &self.increases {
            if inc.event_id != ev.id {
                continue;
            }
            if let Some(at) = parse_iso8601(&inc.detected_at) {
                let age_ms = (now - at).num_milliseconds();
                if (0..=24 * 60 * 60 * 1000).contains(&age_ms) {
                    out.insert(aid.clone(), inc.clone());
                }
            }
        }
        out
    }
}

#[derive(Default)]
struct FeedInner {
    last_checked: Option<DateTime<Utc>>,
    next_allowed_at: Option<DateTime<Utc>>,
    last_succeeded_at: Option<DateTime<Utc>>,
    last_attempt_failed: bool,
    etag: Option<String>,
    events: Vec<CodexResetEvent>,
    is_fetching: bool,
    observation: CodexResetObservation,
}

#[derive(Default)]
pub struct CodexResetService {
    inner: Mutex<FeedInner>,
}

pub fn wants_codex_reset_announcements(settings: &AppSettings) -> bool {
    if !settings.show_codex_reset_announcements {
        return false;
    }
    settings.providers.values().any(|c| {
        c.enabled
            && c.provider_id == "codex"
            && (!settings.monitoring_setup_completed
                || settings.authorized_providers.contains(&c.provider_id))
    })
}

impl CodexResetService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.last_checked = None;
            g.next_allowed_at = None;
            g.last_succeeded_at = None;
            g.last_attempt_failed = false;
            g.etag = None;
            g.events.clear();
            g.is_fetching = false;
            g.observation.clear();
        }
    }

    pub fn observe_reading(&self, reading: &ProviderUsage, now: DateTime<Utc>) {
        if let Ok(mut g) = self.inner.lock() {
            let current_ev = CodexResetEvent::current(&g.events, now);
            g.observation.observe(reading, current_ev.as_ref(), now);
        }
    }

    pub fn snapshot_state(&self, enabled: bool, now: DateTime<Utc>) -> CodexResetState {
        if !enabled {
            return CodexResetState::default();
        }
        let Ok(g) = self.inner.lock() else {
            return CodexResetState::default();
        };
        Self::build_state(&g, now)
    }

    fn build_state(g: &FeedInner, now: DateTime<Utc>) -> CodexResetState {
        let event = CodexResetEvent::current(&g.events, now);
        let is_stale = g.last_attempt_failed
            || g.last_succeeded_at
                .map(|t| (now - t).num_seconds() > STALE_THRESHOLD_SECS)
                .unwrap_or(true);
        let health = CodexResetHealth {
            last_succeeded_at: g.last_succeeded_at.map(|t| t.to_rfc3339()),
            last_attempt_failed: g.last_attempt_failed,
            is_stale,
        };
        let increases = g.observation.active_increases(event.as_ref(), now);
        CodexResetState {
            event,
            health,
            increases,
        }
    }

    pub async fn refresh(
        &self,
        http: &reqwest::Client,
        force: bool,
        now: DateTime<Utc>,
    ) -> CodexResetState {
        let etag_to_send;
        {
            let Ok(mut g) = self.inner.lock() else {
                return CodexResetState::default();
            };
            if !force {
                if let Some(last) = g.last_checked {
                    if (now - last).num_seconds() < POLL_INTERVAL_SECS {
                        return Self::build_state(&g, now);
                    }
                }
            }
            if let Some(next_ok) = g.next_allowed_at {
                if now < next_ok {
                    return Self::build_state(&g, now);
                }
            }
            if g.is_fetching {
                return Self::build_state(&g, now);
            }
            g.last_checked = Some(now);
            g.is_fetching = true;
            g.last_attempt_failed = true;
            etag_to_send = g.etag.clone();
        }

        let mut req = http
            .get(SOURCE_URL)
            .timeout(std::time::Duration::from_secs(20))
            .header("Accept", "application/json")
            .header("Cache-Control", "no-cache");
        if let Some(ref etag) = etag_to_send {
            req = req.header("If-None-Match", etag);
        }

        let outcome = req.send().await;
        match outcome {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let new_etag = resp
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);

                if status == 304 {
                    if let Ok(mut g) = self.inner.lock() {
                        g.is_fetching = false;
                        g.last_attempt_failed = false;
                        g.last_succeeded_at = Some(now);
                        return Self::build_state(&g, now);
                    }
                } else if status == 429 {
                    let retry_until = parse_retry_after(retry_after.as_deref(), now);
                    if let Ok(mut g) = self.inner.lock() {
                        g.is_fetching = false;
                        g.next_allowed_at = Some(retry_until);
                        return Self::build_state(&g, now);
                    }
                } else if (200..300).contains(&status) {
                    if let Ok(bytes) = resp.bytes().await {
                        if let Ok(snap) = decode_snapshot(&bytes) {
                            if let Ok(mut g) = self.inner.lock() {
                                g.is_fetching = false;
                                g.events = snap.events;
                                g.last_attempt_failed = false;
                                g.last_succeeded_at = Some(now);
                                if new_etag.is_some() {
                                    g.etag = new_etag;
                                }
                                g.next_allowed_at = None;
                                return Self::build_state(&g, now);
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }

        if let Ok(mut g) = self.inner.lock() {
            g.is_fetching = false;
            return Self::build_state(&g, now);
        }
        CodexResetState::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UsageWindow;

    fn dt(s: &str) -> DateTime<Utc> {
        parse_iso8601(s).expect("valid rfc3339")
    }

    fn sample_snapshot_json() -> &'static [u8] {
        br#"{
          "events": [
            {
              "id": "ev-announced-1",
              "type": "direct_reset",
              "status": "announced",
              "title": "Codex quota reset",
              "scope": "all",
              "createdAt": "2026-10-25T08:00:00.000Z",
              "updatedAt": "2026-10-25T08:05:00.000Z",
              "confirmedAt": null,
              "schedule": {
                "precision": "window",
                "from": "2026-10-25T10:00:00.000Z",
                "through": "2026-10-25T14:00:00.000Z",
                "label": "10:00-14:00 UTC"
              },
              "posts": [
                {
                  "publishedAt": "2026-10-25T07:55:00.000Z",
                  "text": "OpenAI 宣布今日重置 Codex 额度",
                  "originalText": "OpenAI announced Codex limits will reset today",
                  "fullOriginalText": null,
                  "url": "https://x.com/openai/status/123"
                }
              ],
              "url": "https://aihot.news/events/ev-announced-1"
            }
          ]
        }"#
    }

    #[test]
    fn decode_and_displayable_lifecycle() {
        let snap = decode_snapshot(sample_snapshot_json()).expect("decode ok");
        assert_eq!(snap.events.len(), 1);
        let ev = &snap.events[0];
        assert_eq!(ev.kind, CodexResetKind::DirectReset);
        assert_eq!(ev.status, CodexResetStatus::Announced);

        // Before and during the scheduled window -> displayable
        assert!(ev.is_displayable(dt("2026-10-25T09:00:00Z")));
        assert!(ev.is_displayable(dt("2026-10-25T14:00:00Z")));
        // Immediately after schedule.through -> expires
        assert!(!ev.is_displayable(dt("2026-10-25T14:00:01Z")));
    }

    #[test]
    fn confirmed_event_outranks_active_prediction_and_expires_after_24h() {
        let raw = br#"{
          "events": [
            {
              "id": "pred-1",
              "type": "direct_reset",
              "status": "announced",
              "title": "Prediction",
              "scope": "all",
              "createdAt": "2026-10-25T08:00:00Z",
              "updatedAt": "2026-10-25T08:00:00Z",
              "schedule": {
                "precision": "window",
                "from": "2026-10-25T09:00:00Z",
                "through": "2026-10-25T18:00:00Z",
                "label": "window"
              },
              "posts": [],
              "url": "https://aihot.news/events/pred-1"
            },
            {
              "id": "conf-1",
              "type": "reset_credit",
              "status": "confirmed",
              "title": "Confirmed credit",
              "scope": "all",
              "createdAt": "2026-10-25T10:00:00Z",
              "updatedAt": "2026-10-25T11:00:00Z",
              "confirmedAt": "2026-10-25T11:00:00Z",
              "schedule": null,
              "posts": [],
              "url": "https://aihot.news/events/conf-1"
            }
          ]
        }"#;
        let snap = decode_snapshot(raw).unwrap();
        let picked = CodexResetEvent::current(&snap.events, dt("2026-10-25T12:00:00Z")).unwrap();
        assert_eq!(picked.id, "conf-1");

        // 25 hours after confirmation -> confirmed expires, and pred-1 is also past 18:00Z so none left
        assert!(CodexResetEvent::current(&snap.events, dt("2026-10-26T12:00:01Z")).is_none());
    }

    #[test]
    fn retry_after_supports_seconds_and_http_date() {
        let now = dt("2026-10-25T10:00:00Z");
        assert_eq!(
            parse_retry_after(Some("120"), now),
            dt("2026-10-25T10:02:00Z")
        );
        assert_eq!(
            parse_retry_after(Some("Sun, 25 Oct 2026 10:30:00 GMT"), now),
            dt("2026-10-25T10:30:00Z")
        );
    }

    #[test]
    fn observation_detects_live_quota_increase_and_guards_fingerprint() {
        let snap = decode_snapshot(sample_snapshot_json()).unwrap();
        let ev = &snap.events[0];
        let mut obs = CodexResetObservation::default();

        let make_reading = |ts: &str, used_frac: f64, fp: &str| ProviderUsage {
            account_id: "codex-default".into(),
            provider_id: "codex".into(),
            state: "live".into(),
            plan_name: Some("Pro".into()),
            checked_at: Some(ts.into()),
            codex_account_fingerprint: Some(fp.into()),
            windows: vec![UsageWindow {
                id: "account-primary_window".into(),
                name: "5小时限额".into(),
                used_fraction: used_frac,
                used_percent: used_frac * 100.0,
                resets_at: Some("2026-10-25T15:00:00Z".into()),
                window_seconds: Some(18000),
                exhausted: false,
            }],
            ..Default::default()
        };

        let r1 = make_reading("2026-10-25T10:10:00Z", 0.65, "fp-alice");
        obs.observe(&r1, Some(ev), dt("2026-10-25T10:10:05Z"));
        assert!(obs.active_increases(Some(ev), dt("2026-10-25T10:10:05Z")).is_empty());

        // Second reading 10 minutes later with 60% used (5% available quota recovery)
        let r2 = make_reading("2026-10-25T10:20:00Z", 0.60, "fp-alice");
        obs.observe(&r2, Some(ev), dt("2026-10-25T10:20:05Z"));
        let active = obs.active_increases(Some(ev), dt("2026-10-25T10:20:05Z"));
        assert_eq!(active.get("codex-default").map(|i| i.event_id.as_str()), Some("ev-announced-1"));

        // Account switch (different fingerprint) clears the increase
        let r3 = make_reading("2026-10-25T10:25:00Z", 0.10, "fp-bob");
        obs.observe(&r3, Some(ev), dt("2026-10-25T10:25:05Z"));
        assert!(obs.active_increases(Some(ev), dt("2026-10-25T10:25:05Z")).is_empty());
    }
}
