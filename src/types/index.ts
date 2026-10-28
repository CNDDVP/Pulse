export interface UsageWindow {
  id: string; name: string; used_fraction: number; used_percent: number;
  resets_at: string | null; window_seconds: number | null; exhausted: boolean;
}
export interface Balance {
  currency: string;
  amount: number;
  expires_at?: string | null;
}
export interface HourlyUsage {
  timestamp: number;
  model_id: string;
  calls: number;
  credit_consumed: number;
}
export interface CodexResetCredits {
  status: "available" | "unreported" | "codex_missing";
  count?: number | null;
  next_expiry?: string | null;
}
export interface UsageMetricPoint {
  t: string;
  v: number;
}
export interface UsageMetric {
  id: string;
  label: string;
  value: number;
  unit?: string | null;
  history?: UsageMetricPoint[];
}
export interface ProviderUsage {
  account_id: string; provider_id: string; display_name: string;
  state: "live" | "stale" | "loading" | "unavailable" | "error";
  primary_percent: number | null; plan_name: string | null; is_active: boolean;
  windows: UsageWindow[]; balances: Balance[];
  error_code: string | null; error_message: string | null; source: string;
  checked_at: string | null; last_success_at: string | null; retry_after_seconds: number | null;
  duration_ms: number | null;
  /** Round4 项目一：旁路实时输出速率（tok/min，后端已取整并把 <1 收敛为 None）。
   *  null/缺省 = 渠道日志无 usage 字段或速率不足 1，前端显示「—」，不编造。 */
  tok_per_min?: number | null;
  hourly_usages?: HourlyUsage[];
  web_auth_required?: boolean;
  codex_reset_credits?: CodexResetCredits | null;
  metrics?: UsageMetric[] | null;
}
export interface ProviderConfig {
  provider_id: string; label: string; enabled: boolean; order: number; use_local: boolean;
  credential_configured: boolean; primary_window: string | null;
  /** Window the outer elapsed-time ring tracks; null = soonest reset. */
  elapsed_window: string | null;
  elapsed_period_days?: number | null;
  /** #rrggbb override for the ring colour; null = pressure colour. */
  ring_color: string | null;
  /** Low-balance notice line, compared only against `low_balance_currency`. */
  low_balance: number | null; low_balance_currency: string | null;
  /** "icon" (default) or "bot" — animated mark instead of the provider badge. */
  mark_mode: string | null;
  bot_persona: string | null; bot_shape: string | null; bot_color: string | null;
  /** Optional second quota drawn as an inner ring; null = single ring. */
  secondary_window: string | null;
  /** Antigravity only: split Gemini / Claude-GPT into separate rail slots. */
  split_model_groups: boolean;
  /** Opt-in: show official Codex reset credits on the hover card. */
  show_codex_reset_credits?: boolean;
  /** Opt-in: show detailed card (window value estimate, 1d/7d/31d local usage, 31d chart, top model, cache hit rate, prompt cache countdown). */
  detailed_card?: boolean;
  /** Balance ring calculation basis: "since_top_up" | "balance_only" | "budget". */
  balance_basis?: "since_top_up" | "balance_only" | "budget" | null;
  /** User budget amount when balance_basis === "budget". */
  balance_budget?: number | null;
  /** Extension identifier when provider_id === "extension". */
  extension_id?: string | null;
}
export interface NotificationSettings { threshold: number | null; on_spent: boolean; on_reset: boolean; on_failure: boolean }
export interface HotkeySettings { open_settings: string | null; toggle_rail: string | null }
/** 订阅记录（Round4 项目三）：按来源手动登记订阅价，字段与后端 SubscriptionRecord 对齐（snake_case 上链路）。 */
export interface SubscriptionRecord {
  price: number;
  /** 三位大写币种代码（如 USD/CNY）。 */
  currency: string;
  /** 订阅周期天数（1~366）。 */
  cycle_days: number;
  /** YYYY-MM-DD 或空串（未设置）。 */
  start_date: string;
  note: string;
}
export interface MonitorOption { name: string; label: string }
export interface NetworkProxySettings {
  mode: "auto" | "manual_http" | "manual_socks5";
  host: string;
  port: number;
}
export interface AppSettings {
  rail_warnings?: RailWarnings;
  schema_version: number; generation?: number; dock_side: "right" | "left" | "top" | "free";
  auto_collapse_seconds: number; theme: "obsidian" | "translucent";
  refresh_interval_seconds: number; display_mode: "used" | "remaining";
  forecast: boolean; show_elapsed: boolean; follow_active_display: boolean; hide_fullscreen: boolean;
  monitor_name: string | null; free_x: number; free_y: number;
  warning_threshold: number; show_rail: boolean; reduce_motion: boolean; start_behavior: string;
  monitoring_setup_completed: boolean; token_spend_enabled: boolean;
  authorized_providers: string[]; network_proxy: NetworkProxySettings;
  collapsed_bar_color_mode?: "auto" | "rainbow" | "custom";
  collapsed_bar_color?: string | null;
  /** Round4 项目二：成本显示币种与 USD→CNY 固定汇率（纯本地设置，不联网取汇）。换算只发生在展示层，估算与导出保持 USD 原值。 */
  display_currency?: "USD" | "CNY";
  usd_cny_rate?: number;
  /** Round4 项目三：订阅记录（来源标识 → 价格/币种/周期/开始日/备注），走既有设置持久化。 */
  subscriptions?: Record<string, SubscriptionRecord>;
  /** Round5A 项目四：自定义扫描路径（来源 → 绝对路径目录列表，每来源上限 20）。
   *  与默认扫描根只按文件路径去重：同一文件复制进多个目录（或目录嵌套默认根内）会重复计数。 */
  token_spend_extra_paths?: Record<string, string[]>;
  /** Round5B 项目二：WSL 用量（opt-in 默认关）：开启后经 wsl.exe 只读读取默认发行版内的
   *  claude/qwen projects 转录并合并统计；不可用/失败静默降级并在统计说明中标注。 */
  token_spend_wsl?: boolean;
  /** Round5c 项目一：界面语言（"zh" | "en"，默认 zh）。走既有设置通道持久化；
   *  后端侧已登记同名默认字段（deny_unknown_fields），翻译只在前端词典内完成。 */
  language?: "zh" | "en";
  /** Round5d 项目二：Discord 状态广播（opt-in 默认关）。开启后仅在本机与 Discord 客户端
   *  经本地 IPC 通信，广播聚合信息（正在写代码/空闲、启用账号数、今日 token 总量），
   *  不广播账号名/供应商明细；连接失败静默。 */
  discord_presence_enabled?: boolean;
  /** Round 6 多设备同步（docs/ROUND6_PLAN.md）："off" | "host" | "connect"。
   *  host=本实例开 hub（45539）；connect=作为客户端连其他实例的 hub。访问密钥不落
   *  settings.json——分别存 Windows 凭据管理器（sync-hub-secret / sync-client-secret）。 */
  sync_mode?: "off" | "host" | "connect";
  /** connect 模式的远端 hub 地址（http(s)://host[:port]，不含路径）；host/off 时忽略。 */
  sync_connect_url?: string;
  /** Codex 公开重置卡预告与确认消息（AI Hot News，默认开启）。 */
  show_codex_reset_announcements?: boolean;
  /** 用量报告（Recap）中用于计算「订阅回本倍数」的月订阅价格（0 = 不启用回本卡）。 */
  recap_monthly_price?: number;
  /** 用量报告（Recap）隐私开关：隐藏项目文件夹名称。 */
  recap_hide_projects?: boolean;
  notifications: NotificationSettings; hotkeys: HotkeySettings;
  providers: Record<string, ProviderConfig>;
}

export interface RailAccountRule {
  mode: 'primary' | 'all' | 'window';
  window_id: string | null;
  balances: Record<string, {yellow:number;red:number}>;
}
export interface RailWarnings {
  scope:'all'|'selected'; account_ids:string[]; custom_thresholds:boolean;
  yellow:number; red:number; accounts:Record<string,RailAccountRule>;
}

/** 一次全量刷新的结果（A07）：读数快照 + 实际发起/被冷却跳过的账号数。 */
export interface RefreshSummary { readings: ProviderUsage[]; initiated: number; skipped: number; }

export interface ProxyDetection {
  mode: string;
  detected_type: string;
  address: string | null;
  detail: string;
}

export interface NetworkTestResult {
  ok: boolean;
  target: string;
  status: number | null;
  duration_ms: number;
  error: string | null;
}

export interface NotificationSendResult {
  success: boolean;
  stage: string;
  setting: string;
  test_id: string;
  error: string | null;
  hint: string | null;
}

export interface NotificationFullStatus {
  identity_status: "registered" | "moved" | "missing_shortcut" | "missing_aumid" | "unregistered";
  shortcut_path: string | null;
  shortcut_target: string | null;
  current_exe: string;
  windows_toasts_enabled: boolean | null;
  app_notification_setting: "enabled" | "disabled_for_app" | "disabled_for_user" | "disabled_by_policy" | "disabled_by_manifest" | "unknown";
  is_portable: boolean;
  plugin_permission: string;
  last_error: string | null;
}

// ---------------------------------------------------------------------------
// 多设备同步（Round 6，docs/ROUND6_PLAN.md）：与 src-tauri sync_hub / commands 对齐
// ---------------------------------------------------------------------------

/** 一次上推/拉取的结果记录（诚实降级：失败与时间都暴露给 UI）。 */
export interface SyncPollRecord {
  at: string;
  ok: boolean;
  error?: string | null;
  devices?: number | null;
  version?: number | null;
}

/** sync_hub_status 命令返回：设置页「多设备同步」区块的数据源。 */
export interface SyncHubStatusInfo {
  mode: string;
  connect_url: string;
  hub_running: boolean;
  hub_port: number;
  hub_error: string | null;
  device_count: number;
  hub_secret_configured: boolean;
  client_secret_configured: boolean;
  last_poll: SyncPollRecord | null;
  last_push: SyncPollRecord | null;
}

/** 单行日聚合（day/source/model + 四列 token 计数），与后端 SyncDayRow 同构。 */
export interface SyncDayRow {
  day: string; source: string; model: string;
  input: number; output: number; cache_read: number; cache_write: number;
}

/** hub 侧设备记录；last_active 由 hub 收到推送时盖时间戳（不信任客户端自报）。 */
export interface SyncDeviceRecord {
  device_id: string; device_name: string; app_version: string;
  last_active: string; days: SyncDayRow[];
}

/** GET /v1/devices 响应 & sync-devices.json 落盘格式（客户端合并视图快照）。 */
export interface SyncDevicesSnapshot {
  version: number;
  generated_at?: string | null;
  devices: SyncDeviceRecord[];
}

// ---------------------------------------------------------------------------
// Phase 1: Codex Public Reset Announcements (guanbear/Pulse-codex-reset)
// ---------------------------------------------------------------------------

export type CodexResetKind = "direct_reset" | "reset_credit";
export type CodexResetStatus = "announced" | "confirmed";
export type CodexResetPrecision = "exact" | "approximate" | "deadline" | "date" | "window";

export interface CodexResetSchedule {
  precision: CodexResetPrecision;
  from: string;
  through: string;
  label: string;
}

export interface CodexResetPost {
  published_at: string;
  text: string;
  original_text?: string | null;
  full_original_text?: string | null;
  url: string;
}

export interface CodexResetEvent {
  id: string;
  kind: CodexResetKind;
  status: CodexResetStatus;
  title: string;
  scope: string;
  created_at: string;
  updated_at: string;
  confirmed_at?: string | null;
  schedule?: CodexResetSchedule | null;
  posts: CodexResetPost[];
  url: string;
}

export interface CodexResetHealth {
  last_succeeded_at: string | null;
  last_attempt_failed: boolean;
  is_stale: boolean;
}

export interface CodexResetIncrease {
  event_id: string;
  detected_at: string;
}

export interface CodexResetState {
  event: CodexResetEvent | null;
  health: CodexResetHealth;
  increases: Record<string, CodexResetIncrease>;
}

// ---------------------------------------------------------------------------
// Phase 2: Codex Signals (Signs of a weaker model)
// ---------------------------------------------------------------------------

export interface CodexModelSignal {
  model: string;
  responses: number;
  reached_lattice: number;
  on_lattice: number;
  share: number | null;
  insufficient_data: boolean;
}

export interface CodexSettingChange {
  kind: "model" | "effort";
  asked: string;
  ran: string;
  timestamp: string;
  session_title: string;
}

export interface CodexSignalsReport {
  span_days: number | null;
  scanned_sessions: number;
  judged_sessions: number;
  unjudged_sessions: number;
  models: CodexModelSignal[];
  changes: CodexSettingChange[];
  total_changes: number;
}

// ---------------------------------------------------------------------------
// Phase 3a: Account Detailed Card & Prompt Cache Countdown
// ---------------------------------------------------------------------------

export interface WindowValueEstimate {
  window_id: string;
  full_usd: number;
  used_usd: number;
  unpriced_tokens: number;
}

export interface CardActivitySummary {
  is_account_wide: boolean;
  today_tokens: number;
  today_cost_usd: number;
  today_unpriced: number;
  week_tokens: number;
  week_cost_usd: number;
  week_unpriced: number;
  month_tokens: number;
  month_cost_usd: number;
  month_unpriced: number;
  days_31: { day: string; tokens: number }[];
  top_model: { name: string; share_percent: number } | null;
  cache_hit_rate: number | null;
}

export interface PromptCacheChat {
  id: string;
  title: string;
  replied_at: string;
  expires_at: string;
  lifetime_seconds: number;
  is_minimum: boolean;
}

export interface PromptCacheStatus {
  provider_id: string;
  lifetime_seconds: number;
  is_minimum: boolean;
  live_chats: PromptCacheChat[];
  recently_lapsed: boolean;
}

export interface AccountCardDetail {
  account_id: string;
  window_estimates: Record<string, WindowValueEstimate>;
  activity: CardActivitySummary | null;
  prompt_cache: PromptCacheStatus | null;
}

// ---------------------------------------------------------------------------
// Phase 3b: Daily / Weekly / Monthly / Yearly Recap
// ---------------------------------------------------------------------------

export type RecapPeriodKind = "day" | "week" | "month" | "year";

export interface RecapPeriodInfo {
  kind: RecapPeriodKind;
  key: string;
  label: string;
  start: string;
  end: string;
  in_progress: boolean;
}

export interface RecapDayItem {
  day: string;
  tokens: number;
  cost_usd: number;
  unrecorded: boolean;
  future: boolean;
}

export interface RecapMonthItem {
  month: string;
  label: string;
  tokens: number;
  cost_usd: number;
  unrecorded: boolean;
  future: boolean;
}

export interface RecapModelItem {
  model: string;
  tokens: number;
  share: number;
  cost_usd: number;
}

export interface RecapAgentItem {
  source: string;
  label: string;
  tokens: number;
  share: number;
  active_days: string[];
  active_months: string[];
}

export interface RecapProjectItem {
  name: string;
  tokens: number;
  share: number;
  sessions: number;
}

export interface RecapBusiestHours {
  start_hour: number;
  end_hour: number;
  share: number;
}

export interface RecapWorkSplit {
  weekday_share: number;
  weekend_share: number;
}

export interface RecapReport {
  period: RecapPeriodInfo;
  records_begin: string | null;
  tokens: number;
  previous_tokens: number | null;
  cost_usd: number;
  unpriced_tokens: number;
  partial: boolean;
  active_days: number;
  observed_days: number;
  total_days: number;
  daily_average_tokens: number;
  sessions: number;
  current_streak: number;
  longest_streak: number;
  streak_chain: { day: string; active: boolean }[];
  busiest_day: { day: string; tokens: number } | null;
  days: RecapDayItem[];
  months: RecapMonthItem[];
  hours: number[];
  peak_hour: number | null;
  busiest_hours: RecapBusiestHours | null;
  first_hour: number | null;
  latest_minute: number | null;
  late_nights: number;
  persona: "night_owl" | "early_bird" | "day_shift" | "all_day";
  work_split: RecapWorkSplit | null;
  cache_hit_rate: number | null;
  cache_savings_usd: number;
  paid_months: number;
  models: RecapModelItem[];
  agents: RecapAgentItem[];
  projects: RecapProjectItem[];
}

// ---------------------------------------------------------------------------
// Phase 4: Extensions System
// ---------------------------------------------------------------------------

export interface ExtensionEntry {
  id: string;
  name: string;
  folder_name: string;
  executable: string;
  timeout_seconds: number;
  account_id: string;
  enabled: boolean;
  status: "ready" | "invalid_manifest" | "unsupported_schema" | "duplicate_id" | "executable_outside_folder" | "executable_missing";
  error: string | null;
}

