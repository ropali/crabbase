use std::cell::RefCell;
use std::rc::Rc;

use gloo_timers::callback::{Interval, Timeout};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::api::client::ApiClient;
use crate::models::log::{LogEntry, LogStatsResponse};

const PAGE_SIZES: [i64; 3] = [50, 100, 250];
const LIVE_REFRESH_MS: u32 = 5_000;
const SEARCH_DEBOUNCE_MS: u32 = 300;
/// Columns rendered per row: expander, severity, event, actor, target, result,
/// duration, client, timestamp. The expanded detail row spans all of them.
const COLUMN_COUNT: usize = 9;

/// `1H` (UI label) -> `1h` (API parameter).
fn api_range(label: &str) -> &'static str {
    match label {
        "5M" => "5m",
        "15M" => "15m",
        "30M" => "30m",
        "1H" => "1h",
        "2H" => "2h",
        "6H" => "6h",
        "24H" => "24h",
        "7D" => "7d",
        "30D" => "30d",
        _ => "24h",
    }
}

/// Mirrors the server's bucket choice (`granularity` in `routes/logs.rs`) so
/// the chart can label its peak as "per minute/hour/day" without a second
/// round trip.
fn bucket_unit(range: &str) -> &'static str {
    match range {
        "5M" | "15M" | "30M" | "1H" | "2H" | "6H" => "min",
        "24H" | "7D" => "hr",
        _ => "day",
    }
}

/// `2026-11-26T14:22:01.032Z` -> `2026-11-26 14:22:01.032`.
///
/// Deliberately string-based for UTC: the value is UTC from Postgres and only
/// ever displayed, so pulling `chrono` into the wasm bundle buys nothing. For
/// local time we go through the browser's `Date`, the only clock that knows the
/// user's timezone.
///
/// `hour12` comes from the user's saved choice (falling back to the browser
/// locale), because `Intl`'s default is locale-derived and does not reflect the
/// OS's 12/24-hour switch.
fn format_timestamp(raw: &str, local: bool, hour12: bool) -> String {
    if !local {
        let trimmed = raw.trim_end_matches('Z');
        let trimmed = trimmed.split('+').next().unwrap_or(trimmed);
        return trimmed.replacen('T', " ", 1);
    }

    let Some(ms) = parse_utc_ms(raw) else {
        return format_timestamp(raw, false, hour12);
    };

    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    let date_part = format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    );

    if hour12 {
        let hours = date.get_hours();
        let hour = match hours % 12 {
            0 => 12,
            hour => hour,
        };
        let meridiem = if hours < 12 { "AM" } else { "PM" };

        return format!(
            "{date_part} {hour:02}:{:02}:{:02} {meridiem}",
            date.get_minutes(),
            date.get_seconds()
        );
    }

    format!(
        "{date_part} {:02}:{:02}:{:02}",
        date.get_hours(),
        date.get_minutes(),
        date.get_seconds()
    )
}

const HOUR12_STORAGE_KEY: &str = "crabbase_logs_hour12";

/// Saved 12/24-hour choice, or the browser locale's default on first visit.
///
/// `Intl.DateTimeFormat().resolvedOptions().hourCycle` follows the *locale*
/// (`en-GB` -> `h23`, `en-US` -> `h12`); browsers do not expose the operating
/// system's clock-format toggle, so a saved choice is the only reliable signal.
fn initial_hour12() -> bool {
    if let Some(storage) = web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
    {
        match storage
            .get_item(HOUR12_STORAGE_KEY)
            .ok()
            .flatten()
            .as_deref()
        {
            Some("true") => return true,
            Some("false") => return false,
            _ => {}
        }
    }

    browser_prefers_12h()
}

fn save_hour12(value: bool) {
    if let Some(storage) = web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
    {
        let _ = storage.set_item(HOUR12_STORAGE_KEY, if value { "true" } else { "false" });
    }
}

/// Whether the browser locale prefers a 12-hour clock.
///
/// `Intl.DateTimeFormat().resolvedOptions().hourCycle` is `h11`/`h12` for
/// 12-hour locales and `h23`/`h24` for 24-hour locales. Cached because this is
/// only the fallback default.
fn browser_prefers_12h() -> bool {
    thread_local! {
        static PREFERS_12H: std::cell::OnceCell<bool> = std::cell::OnceCell::new();
    }

    PREFERS_12H.with(|cache| *cache.get_or_init(detect_12h))
}

fn detect_12h() -> bool {
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &options,
        &wasm_bindgen::JsValue::from_str("hour"),
        &wasm_bindgen::JsValue::from_str("numeric"),
    );
    let _ = js_sys::Reflect::set(
        &options,
        &wasm_bindgen::JsValue::from_str("minute"),
        &wasm_bindgen::JsValue::from_str("numeric"),
    );

    let formatter = js_sys::Intl::DateTimeFormat::new(&js_sys::Array::new(), &options);
    let resolved = formatter.resolved_options();

    matches!(
        js_sys::Reflect::get(&resolved, &wasm_bindgen::JsValue::from_str("hourCycle"))
            .ok()
            .and_then(|value| value.as_string())
            .as_deref(),
        Some("h11") | Some("h12")
    )
}

/// JS `Date.parse` promises milliseconds, while Postgres emits microseconds
/// (`.140117Z`). Trim the fraction to three digits before parsing.
fn parse_utc_ms(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    let normalized = match raw.find('.') {
        Some(dot) => {
            let rest = &raw[dot + 1..];
            let digits = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            if digits > 3 {
                format!("{}.{}{}", &raw[..dot], &rest[..3], &rest[digits..])
            } else {
                raw.to_string()
            }
        }
        None => raw.to_string(),
    };

    let ms = js_sys::Date::parse(&normalized);
    ms.is_finite().then_some(ms)
}

fn bucket_label(raw: &str, range: &str, local: bool, hour12: bool) -> String {
    let stamp = format_timestamp(raw, local, hour12);
    match range {
        "5M" | "15M" | "30M" | "1H" | "2H" | "6H" | "24H" => {
            stamp.get(11..16).unwrap_or(&stamp).to_string()
        }
        _ => stamp.get(5..10).unwrap_or(&stamp).to_string(),
    }
}

const MINUTE_MS: f64 = 60_000.0;
const HOUR_MS: f64 = 3_600_000.0;
const DAY_MS: f64 = 86_400_000.0;

fn bucket_span_ms(range: &str) -> f64 {
    match bucket_unit(range) {
        "min" => MINUTE_MS,
        "hr" => HOUR_MS,
        _ => DAY_MS,
    }
}

/// One bucket boundary as `MM-DD HH:MM` (minute/hour buckets) or `MM-DD`
/// (day buckets). Postgres returns UTC instants; `local` switches the getters
/// rather than re-parsing, so the value is correct in both zones.
fn format_bucket_moment(ms: f64, range: &str, local: bool, hour12: bool) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    let (month, day, hours, minutes) = if local {
        (
            date.get_month() + 1,
            date.get_date(),
            date.get_hours(),
            date.get_minutes(),
        )
    } else {
        (
            date.get_utc_month() + 1,
            date.get_utc_date(),
            date.get_utc_hours(),
            date.get_utc_minutes(),
        )
    };

    let date_part = format!("{month:02}-{day:02}");
    if bucket_unit(range) == "day" {
        return date_part;
    }

    let clock = if hour12 {
        let hour = match hours % 12 {
            0 => 12,
            hour => hour,
        };
        let meridiem = if hours < 12 { "AM" } else { "PM" };
        format!("{hour:02}:{minutes:02} {meridiem}")
    } else {
        format!("{hours:02}:{minutes:02}")
    };

    format!("{date_part} {clock}")
}

/// Tooltip window for a bucket: `"11-28 14:00–15:00 · local"` for hourly
/// buckets, or `"11-28"` for daily ones. The end date is dropped when the
/// bucket stays inside one calendar day.
fn bucket_window(raw: &str, range: &str, local: bool, hour12: bool) -> String {
    let Some(start) = parse_utc_ms(raw) else {
        return bucket_label(raw, range, local, hour12);
    };

    let start_label = format_bucket_moment(start, range, local, hour12);
    if bucket_unit(range) == "day" {
        return start_label;
    }

    let end_label = format_bucket_moment(start + bucket_span_ms(range), range, local, hour12);
    let end_label = match end_label.strip_prefix(&start_label[..5]) {
        Some(rest) => rest.trim_start_matches(' ').to_string(),
        None => end_label,
    };

    let zone = if local { "local" } else { "UTC" };
    format!("{start_label}–{end_label} · {zone}")
}

fn format_count(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }

    if value < 0 { format!("-{out}") } else { out }
}

fn format_duration(value: Option<f64>) -> String {
    match value {
        Some(ms) if ms >= 10.0 => format!("{ms:.0}ms"),
        Some(ms) => format!("{ms:.1}ms"),
        None => "—".to_string(),
    }
}

/// Optional column -> displayable text.
fn opt(value: &Option<String>) -> String {
    value
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("—")
        .to_string()
}

fn severity_class(severity: &str) -> &'static str {
    match severity {
        "ERROR" => "bg-error-container text-on-error-container border-error/30",
        "WARN" => "bg-yellow-100 text-yellow-700 border-yellow-200",
        "DEBUG" => "bg-blue-100 text-blue-700 border-blue-200",
        "TRACE" => "bg-surface-container text-on-surface-variant border-outline-variant",
        _ => "bg-green-100 text-green-700 border-green-200",
    }
}

fn outcome_class(outcome: &str) -> &'static str {
    match outcome {
        "denied" => "bg-error-container text-on-error-container border-error/30",
        "failure" => "bg-yellow-100 text-yellow-700 border-yellow-200",
        _ => "bg-surface-container text-on-surface-variant border-outline-variant",
    }
}

fn status_class(status: i32) -> &'static str {
    if status >= 500 {
        "bg-error-container text-on-error-container border-error/30"
    } else if status >= 400 {
        "bg-yellow-100 text-yellow-700 border-yellow-200"
    } else {
        "bg-surface-container text-on-surface-variant border-outline-variant"
    }
}

fn has_payload(entry: &LogEntry) -> bool {
    entry
        .data
        .as_object()
        .map(|object| !object.is_empty())
        .unwrap_or(false)
}

/// One label/value pair inside the expanded detail row.
fn detail_field(label: &str, value: String) -> Html {
    detail_field_with_hint(label, value, None)
}

/// Like [`detail_field`], with an optional info icon whose native tooltip
/// explains a jargon-looking field.
fn detail_field_with_hint(label: &str, value: String, hint: Option<&str>) -> Html {
    html! {
        <div class="min-w-0">
            <div class="flex items-center gap-1 mb-0.5">
                <div class="font-label-xs text-label-xs uppercase tracking-widest text-on-surface-variant font-bold">
                    {label.to_string()}
                </div>
                {
                    if let Some(hint) = hint {
                        html! {
                            <span
                                class="material-symbols-outlined text-[13px] leading-none text-on-surface-variant/50 cursor-help"
                                title={hint.to_string()}
                            >
                                {"info"}
                            </span>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
            <div class="font-code-md text-code-md text-on-background break-all">{value}</div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct CopyableDetailFieldProps {
    pub label: String,
    pub value: Option<String>,
    /// Explanatory tooltip shown as an info icon next to the label.
    #[prop_or_default]
    pub hint: Option<String>,
}

/// A detail field for an identifier, with a copy-to-clipboard button.
/// Missing ids render as `—` and get no button.
#[function_component(CopyableDetailField)]
fn copyable_detail_field(props: &CopyableDetailFieldProps) -> Html {
    let copied = use_state(|| false);
    let value = props
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let on_copy = {
        let value = value.clone();
        let copied = copied.clone();
        Callback::from(move |_| {
            let Some(value) = value.clone() else {
                return;
            };
            if let Some(window) = web_sys::window() {
                let _ = window.navigator().clipboard().write_text(&value);
                copied.set(true);

                let copied_reset = copied.clone();
                Timeout::new(1_500, move || copied_reset.set(false)).forget();
            }
        })
    };

    html! {
        <div class="min-w-0">
            <div class="flex items-center gap-1 mb-0.5">
                <div class="font-label-xs text-label-xs uppercase tracking-widest text-on-surface-variant font-bold">
                    {props.label.clone()}
                </div>
                {
                    if let Some(hint) = props.hint.as_ref() {
                        html! {
                            <span
                                class="material-symbols-outlined text-[13px] leading-none text-on-surface-variant/50 cursor-help"
                                title={hint.clone()}
                            >
                                {"info"}
                            </span>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
            <div class="flex items-start gap-1.5">
                <div class="font-code-md text-code-md text-on-background break-all">
                    {value.clone().unwrap_or_else(|| "—".to_string())}
                </div>
                {
                    if value.is_some() {
                        html! {
                            <button
                                type="button"
                                onclick={on_copy}
                                title={if *copied { "Copied" } else { "Copy to clipboard" }}
                                aria-label={format!("Copy {}", props.label.to_ascii_lowercase())}
                                class={classes!(
                                    "shrink-0", "-mt-0.5", "p-0.5", "rounded", "transition-colors",
                                    "hover:bg-surface-container-high",
                                    if *copied {
                                        "text-primary"
                                    } else {
                                        "text-on-surface-variant/50 hover:text-primary"
                                    }
                                )}
                            >
                                <span class="material-symbols-outlined text-[15px] leading-none">
                                    { if *copied { "check" } else { "content_copy" } }
                                </span>
                            </button>
                        }
                    } else {
                        html! {}
                    }
                }
            </div>
        </div>
    }
}

/// Everything stored for one event, including the raw `data` payload.
fn detail_panel(entry: &LogEntry, local: bool, hour12: bool) -> Html {
    let pretty =
        serde_json::to_string_pretty(&entry.data).unwrap_or_else(|_| entry.data.to_string());
    let route = entry
        .data
        .get("route")
        .and_then(|value| value.as_str())
        .unwrap_or("—")
        .to_string();

    html! {
        <div class="bg-surface-container-low/60 border-t border-outline-variant px-4 py-4 flex flex-col gap-4">
            <div class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-x-4 gap-y-3">
                <CopyableDetailField label={"Log id".to_string()} value={Some(entry.id.clone())} />
                { detail_field("Action", opt(&entry.action)) }
                { detail_field("Message", entry.message.clone()) }
                { detail_field("Severity", entry.severity.clone()) }
                { detail_field("Level", entry.level.to_string()) }
                { detail_field("Outcome", entry.outcome.clone()) }
                { detail_field("Method", opt(&entry.method)) }
                { detail_field("Path", opt(&entry.path)) }
                { detail_field("Route", route) }
                { detail_field("Status code", entry.status_code.map(|s| s.to_string()).unwrap_or_else(|| "—".to_string())) }
                { detail_field("Duration", format_duration(entry.duration_ms)) }
                <CopyableDetailField
                    label={"Actor id".to_string()}
                    value={entry.actor_id.clone()}
                    hint={Some("The authenticated user's row id. The collection that holds that user is shown in Actor collection (for example users or _superusers). Empty for anonymous requests.".to_string())}
                />
                { detail_field_with_hint(
                    "Actor collection",
                    opt(&entry.actor_collection),
                    Some("The collection that holds the authenticated user — for example users or _superusers. It comes from the auth token, not from the request URL."),
                ) }
                { detail_field_with_hint(
                    "Target collection",
                    opt(&entry.target_collection),
                    Some("The {name} segment of the request URL — the collection the request operated on. Empty for routes without a collection, such as backups, jobs and crons."),
                ) }
                <CopyableDetailField
                    label={"Target id".to_string()}
                    value={entry.target_id.clone()}
                    hint={Some("The {id} segment of the matched route: the primary key of the addressed record or resource (record, backup, job or cron). Empty when the route has no {id}.".to_string())}
                />
                { detail_field("IP address", opt(&entry.ip_address)) }
                { detail_field("User agent", opt(&entry.user_agent)) }
                { detail_field(if local { "Timestamp (local)" } else { "Timestamp (UTC)" }, format_timestamp(&entry.created, local, hour12)) }
            </div>

            <div>
                <div class="font-label-xs text-label-xs uppercase tracking-widest text-on-surface-variant font-bold mb-1">
                    {"Data"}
                </div>
                <pre class="bg-surface-container-low border border-outline-variant rounded-lg p-3 font-code-md text-code-md text-on-surface overflow-x-auto whitespace-pre-wrap break-all">
                    {pretty}
                </pre>
            </div>
        </div>
    }
}

#[function_component(ActivityLogs)]
pub fn activity_logs() -> Html {
    // Filters
    let search_input = use_state(String::new);
    let applied_search = use_state(String::new);
    let selected_level = use_state(|| "ALL".to_string());
    let time_range = use_state(|| "1H".to_string());
    let superuser_only = use_state(|| false);
    let is_live = use_state(|| true);
    // Timestamps are stored and returned in UTC; this only changes rendering.
    let show_local = use_state(|| true);
    // 12-hour rendering for local timestamps; saved across reloads.
    let hour12 = use_state(initial_hour12);
    let page = use_state(|| 1i64);
    let per_page = use_state(|| 50i64);

    // Data
    let entries = use_state(Vec::<LogEntry>::new);
    let total = use_state(|| 0i64);
    let stats = use_state(|| None::<LogStatsResponse>);
    let stats_loading = use_state(|| true);
    let enabled = use_state(|| true);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let expanded_id = use_state(|| None::<String>);

    // Refresh trigger. `tick` is a state handle read by the fetch effect's
    // dependency tuple; `counter` lives in a RefCell so the interval callback
    // always increments from the latest value instead of a stale capture.
    let tick = use_state(|| 0u32);
    let counter = use_mut_ref(|| 0u32);

    let bump = {
        let tick = tick.clone();
        let counter = counter.clone();
        Callback::from(move |_: ()| {
            let mut count = counter.borrow_mut();
            *count += 1;
            tick.set(*count);
        })
    };

    // Live polling
    {
        let bump = bump.clone();
        use_effect_with(*is_live, move |live| {
            let interval = (*live).then(|| Interval::new(LIVE_REFRESH_MS, move || bump.emit(())));
            move || drop(interval)
        });
    }

    // Fetch whenever a filter, the page or the refresh tick changes.
    {
        let entries = entries.clone();
        let total = total.clone();
        let stats = stats.clone();
        let stats_loading = stats_loading.clone();
        let enabled = enabled.clone();
        let loading = loading.clone();
        let error = error.clone();

        let deps = (
            *page,
            *per_page,
            (*selected_level).clone(),
            (*time_range).clone(),
            (*applied_search).clone(),
            *superuser_only,
            *tick,
        );

        use_effect_with(deps, move |deps| {
            let (page, per_page, level, range, search, superusers, _) = deps.clone();
            let range = api_range(&range);

            loading.set(true);
            stats_loading.set(true);

            spawn_local(async move {
                let client = ApiClient::default();

                match client
                    .list_logs(page, per_page, &level, range, &search, superusers)
                    .await
                {
                    Ok(response) => {
                        enabled.set(response.enabled);
                        entries.set(response.items);
                        total.set(response.total);
                        error.set(None);
                    }
                    Err(err) => {
                        error.set(Some(err.to_string()));
                        entries.set(Vec::new());
                        total.set(0);
                    }
                }

                loading.set(false);

                // A failed stats call must not blank the table, so it is
                // fetched separately and only replaces the chart on success.
                if let Ok(fresh) = client.get_log_stats(range).await {
                    stats.set(Some(fresh));
                }
                stats_loading.set(false);
            });

            || ()
        });
    }

    let on_search_input = {
        let search_input = search_input.clone();
        let applied_search = applied_search.clone();
        let page = page.clone();
        let debounce: Rc<RefCell<Option<Timeout>>> = use_mut_ref(|| None::<Timeout>);

        Callback::from(move |e: InputEvent| {
            let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() else {
                return;
            };
            let value = target.value();
            search_input.set(value.clone());
            page.set(1);

            // Debounced so typing does not fire a request per keystroke.
            let applied_search = applied_search.clone();
            *debounce.borrow_mut() = Some(Timeout::new(SEARCH_DEBOUNCE_MS, move || {
                applied_search.set(value);
            }));
        })
    };

    let on_level_change = {
        let selected_level = selected_level.clone();
        let page = page.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                selected_level.set(target.value());
                page.set(1);
            }
        })
    };

    let on_range_change = {
        let time_range = time_range.clone();
        let page = page.clone();
        Callback::from(move |range: String| {
            time_range.set(range);
            page.set(1);
        })
    };

    let toggle_superuser = {
        let superuser_only = superuser_only.clone();
        let page = page.clone();
        Callback::from(move |_| {
            superuser_only.set(!*superuser_only);
            page.set(1);
        })
    };

    let toggle_live = {
        let is_live = is_live.clone();
        Callback::from(move |_| is_live.set(!*is_live))
    };

    let toggle_local = {
        let show_local = show_local.clone();
        Callback::from(move |_| show_local.set(!*show_local))
    };

    let toggle_hour12 = {
        let hour12 = hour12.clone();
        Callback::from(move |_| {
            let next = !*hour12;
            hour12.set(next);
            save_hour12(next);
        })
    };

    let on_page_change = {
        let page = page.clone();
        let total = total.clone();
        let per_page = per_page.clone();
        move |delta: i64| {
            let pages = ((*total + *per_page - 1) / *per_page).max(1);
            let next = (*page + delta).clamp(1, pages);
            page.set(next);
        }
    };

    let on_per_page_change = {
        let per_page = per_page.clone();
        let page = page.clone();
        Callback::from(move |e: Event| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                if let Ok(value) = target.value().parse::<i64>() {
                    per_page.set(value);
                    page.set(1);
                }
            }
        })
    };

    let toggle_row = {
        let expanded_id = expanded_id.clone();
        Callback::from(move |id: String| {
            if expanded_id.as_ref() == Some(&id) {
                expanded_id.set(None);
            } else {
                expanded_id.set(Some(id));
            }
        })
    };

    // ── Telemetry strip ──────────────────────────────────────────────────
    let bucket_slice = stats
        .as_ref()
        .map(|value| value.buckets.as_slice())
        .unwrap_or(&[]);
    let max_bucket = bucket_slice
        .iter()
        .map(|bucket| bucket.count)
        .max()
        .unwrap_or(0)
        .max(1);

    let stats_total = stats.as_ref().map(|value| value.total).unwrap_or(0);
    let stats_errors = stats.as_ref().map(|value| value.errors).unwrap_or(0);
    let stats_avg = stats
        .as_ref()
        .and_then(|value| value.avg_duration_ms)
        .map(|ms| format!("Avg {ms:.0}ms"))
        .unwrap_or_else(|| "Avg —".to_string());

    let current_range = (*time_range).clone();
    // 6H buckets by the minute can be 360 bars; a 4px gap per bar would push
    // the axis past its container, so tighten the gap when it gets dense.
    let bar_gap = if bucket_slice.len() > 48 {
        "gap-px"
    } else {
        "gap-1"
    };

    // ── Chart annotations ────────────────────────────────────────────────
    // The stats endpoint zero-fills every bucket, so the first and last bars
    // are the window's actual edges and the axis labels are exact.
    let peak_count = bucket_slice
        .iter()
        .map(|bucket| bucket.count)
        .max()
        .unwrap_or(0);
    let stats_warnings = stats.as_ref().map(|value| value.warnings).unwrap_or(0);
    let first_bucket_label = bucket_slice
        .first()
        .map(|bucket| bucket_label(&bucket.bucket, &current_range, *show_local, *hour12));
    let last_bucket_label = bucket_slice
        .last()
        .map(|bucket| bucket_label(&bucket.bucket, &current_range, *show_local, *hour12));

    // ── Pagination footer ────────────────────────────────────────────────
    let total_pages = ((*total + *per_page - 1) / *per_page).max(1);
    let first_row = if *total == 0 {
        0
    } else {
        (*page - 1) * *per_page + 1
    };
    let last_row = ((*page) * *per_page).min(*total);

    html! {
        <div class="flex flex-col flex-1 h-full overflow-hidden bg-surface relative">
            /* Sticky Sub-header Bar */
            <div class="sticky top-0 z-20 bg-surface border-b border-outline-variant px-gutter py-3 flex flex-col md:flex-row gap-4 items-center justify-between shadow-sm">
                <div class="flex items-center gap-3 w-full md:w-auto flex-wrap">
                    <div class="relative w-full md:w-80">
                        <span class="material-symbols-outlined absolute left-3 top-1/2 -translate-y-1/2 text-on-surface-variant text-[18px]">{"search"}</span>
                        <input
                            class="w-full bg-surface-container-low border border-outline-variant rounded-lg pl-10 pr-4 py-1.5 font-body-sm text-body-sm focus:ring-2 focus:ring-primary focus:border-primary outline-none transition-all"
                            placeholder="Filter logs by action, path, actor, status..."
                            type="text"
                            value={(*search_input).clone()}
                            oninput={on_search_input}
                        />
                    </div>

                    <div class="relative flex items-center">
                        <select
                            value={(*selected_level).clone()}
                            onchange={on_level_change}
                            class="bg-surface-container-low border border-outline-variant rounded-lg px-3 py-1.5 font-body-sm text-body-sm text-on-surface focus:ring-2 focus:ring-primary focus:border-primary outline-none cursor-pointer font-bold"
                        >
                            <option value="ALL">{"All Levels"}</option>
                            <option value="Error">{"Error"}</option>
                            <option value="Warn">{"Warn"}</option>
                            <option value="Info">{"Info"}</option>
                            <option value="Debug">{"Debug"}</option>
                            <option value="Trace">{"Trace"}</option>
                        </select>
                    </div>

                    <div class="h-6 w-[1px] bg-outline-variant hidden md:block mx-1"></div>

                    <label class="hidden md:flex items-center gap-2 cursor-pointer group select-none">
                        <input
                            type="checkbox"
                            checked={*superuser_only}
                            onclick={toggle_superuser}
                            class="w-4 h-4 text-primary border-outline-variant rounded focus:ring-primary cursor-pointer"
                        />
                        <span class="font-body-sm text-body-sm text-on-surface-variant group-hover:text-on-surface transition-colors whitespace-nowrap">
                            {"Superuser logs"}
                        </span>
                    </label>
                </div>

                <div class="flex items-center gap-4">
                    <div class="flex bg-surface-container-low border border-outline-variant rounded-lg p-1">
                        { for ["5M", "15M", "30M", "1H", "2H", "6H", "24H", "7D", "30D"].iter().map(|range| {
                            let value = range.to_string();
                            let is_active = current_range == value;
                            let on_range_change = on_range_change.clone();

                            html! {
                                <button
                                    onclick={move |_| on_range_change.emit(value.clone())}
                                    class={classes!(
                                        "px-3", "py-1", "rounded-md", "font-label-xs", "text-label-xs",
                                        "font-bold", "transition-all", "cursor-pointer",
                                        if is_active { "bg-surface shadow-sm text-primary" } else { "text-on-surface-variant hover:text-on-surface" }
                                    )}
                                >
                                    {range}
                                </button>
                            }
                        }) }
                    </div>

                    <div class="h-6 w-[1px] bg-outline-variant hidden md:block"></div>

                    <button
                        onclick={toggle_local}
                        class={classes!(
                            "px-3", "py-1.5", "rounded-lg", "font-bold", "font-body-sm", "text-body-sm",
                            "border", "transition-all", "cursor-pointer", "flex", "items-center", "gap-2",
                            if *show_local { "border-primary/40 text-primary bg-primary/5" } else { "border-outline-variant text-on-surface-variant hover:bg-surface-container" }
                        )}
                        title={if *show_local { "Timestamps shown in your local timezone — click for UTC" } else { "Timestamps shown in UTC — click for your local timezone" }}
                    >
                        <span class="material-symbols-outlined text-[18px]">{"schedule"}</span>
                        {if *show_local { "Local time" } else { "UTC" }}
                    </button>

                    <button
                        onclick={toggle_hour12}
                        disabled={!*show_local}
                        class={classes!(
                            "px-3", "py-1.5", "rounded-lg", "font-bold", "font-body-sm", "text-body-sm",
                            "border", "transition-all", "flex", "items-center", "gap-2", "whitespace-nowrap",
                            if !*show_local {
                                "border-outline-variant text-on-surface-variant/40 cursor-not-allowed"
                            } else if *hour12 {
                                "border-primary/40 text-primary bg-primary/5 cursor-pointer"
                            } else {
                                "border-outline-variant text-on-surface-variant hover:bg-surface-container cursor-pointer"
                            }
                        )}
                        title={if *show_local {
                            if *hour12 { "Local timestamps use a 12-hour clock — click for 24-hour" } else { "Local timestamps use a 24-hour clock — click for 12-hour" }
                        } else {
                            "Switch to Local time to choose a 12/24-hour clock"
                        }}
                    >
                        {if *hour12 { "12h" } else { "24h" }}
                    </button>

                    <button
                        onclick={{
                            let bump = bump.clone();
                            Callback::from(move |_| bump.emit(()))
                        }}
                        class="px-3 py-1.5 rounded-lg font-bold font-body-sm text-body-sm border border-outline-variant text-on-surface-variant hover:bg-surface-container transition-all cursor-pointer flex items-center gap-2"
                        title="Refresh now"
                    >
                        <span class="material-symbols-outlined text-[18px]">{"refresh"}</span>
                        {"Refresh"}
                    </button>

                    <button
                        onclick={toggle_live}
                        class={classes!(
                            "px-3", "py-1.5", "rounded-lg", "font-bold", "flex", "items-center", "gap-2",
                            "transition-all", "font-body-sm", "text-body-sm", "shadow-sm", "cursor-pointer",
                            if *is_live { "bg-primary text-on-primary" } else { "bg-surface-container text-on-surface-variant border border-outline-variant" }
                        )}
                        title="Toggle automatic refresh every 5 seconds"
                    >
                        <span class="material-symbols-outlined text-[18px]">
                            {if *is_live { "sensors" } else { "pause_circle" }}
                        </span>
                        {if *is_live { "Live" } else { "Paused" }}
                    </button>
                </div>
            </div>

            /* Telemetry strip */
            <div class="bg-surface border-b border-outline-variant h-32 relative shrink-0 z-30">
                <div class="absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent pointer-events-none rounded-b"></div>

                /* Summary row */
                <div class="absolute top-2 left-4 right-4 flex items-center justify-between gap-4 text-label-xs uppercase tracking-widest font-bold text-on-surface-variant pointer-events-none z-10">
                    <div class="flex gap-6">
                        <span>{format!("{} Events", format_count(stats_total))}</span>
                        <span class={classes!("font-extrabold", if stats_errors > 0 { "text-primary" } else { "" })}>
                            {format!("{} Errors", format_count(stats_errors))}
                        </span>
                        <span class={classes!(if stats_warnings > 0 { "text-yellow-600" } else { "" })}>
                            {format!("{} Warnings", format_count(stats_warnings))}
                        </span>
                        <span>{stats_avg}</span>
                    </div>
                    {
                        if peak_count > 0 {
                            html! {
                                <span class="text-on-surface-variant/60 whitespace-nowrap">
                                    {format!("Peak {} / {}", format_count(peak_count), bucket_unit(&current_range))}
                                </span>
                            }
                        } else {
                            html! {}
                        }
                    }
                </div>

                /* Horizontal gridlines: top, thirds and baseline */
                <div class="absolute left-4 right-4 bottom-6 h-[4.5rem] flex flex-col justify-between pointer-events-none">
                    { for (0..4).map(|line| html! {
                        <div class={classes!(
                            "border-t",
                            if line == 3 { "border-outline-variant" } else { "border-outline-variant/25" }
                        )}></div>
                    }) }
                </div>

                /* Bars */
                <div class={classes!("absolute", "left-4", "right-4", "bottom-6", "h-[4.5rem]", "flex", "items-end", bar_gap)}>
                    {
                        if *stats_loading && stats.is_none() {
                            html! {
                                <div class="w-full h-full flex items-end gap-1 animate-pulse">
                                    { for (0..28).map(|bar| {
                                        let height = 12 + ((bar * 37) % 78);
                                        html! {
                                            <div class="flex-1 bg-primary/10 rounded-t-sm" style={format!("height: {height}%")}></div>
                                        }
                                    }) }
                                </div>
                            }
                        } else if bucket_slice.is_empty() || stats_total == 0 {
                            html! {
                                <div class="w-full h-full flex flex-col items-center justify-center gap-1 text-on-surface-variant/60">
                                    <span class="material-symbols-outlined text-[22px]">{"monitoring"}</span>
                                    <span class="font-label-xs text-label-xs uppercase tracking-widest">{"No activity in this window"}</span>
                                    <span class="text-[10px] font-normal normal-case tracking-normal">
                                        {"Try a wider range — only events at or above the server's min_level are stored."}
                                    </span>
                                </div>
                            }
                        } else {
                            html! {
                                <>
                                    { for bucket_slice.iter().enumerate().map(|(index, bucket)| {
                                        let is_empty = bucket.count == 0;
                                        let pct = ((bucket.count * 100) / max_bucket).clamp(3, 100);
                                        let window = bucket_window(&bucket.bucket, &current_range, *show_local, *hour12);
                                        let count_label = format!("{} events", format_count(bucket.count));
                                        let aria = format!("{count_label}, {window}");
                                        let is_latest = index + 1 == bucket_slice.len();
                                        let is_first = index == 0;

                                        html! {
                                            <div
                                                class={classes!(
                                                    "flex-1", "min-w-0", "relative", "group", "self-stretch",
                                                    "flex", "items-end", "cursor-pointer"
                                                )}
                                                aria-label={aria}
                                            >
                                                <div
                                                    class={classes!(
                                                        "w-full", "rounded-t-sm", "transition-all", "duration-150",
                                                        if is_empty {
                                                            "bg-outline-variant/50"
                                                        } else if is_latest {
                                                            "bg-primary/60"
                                                        } else {
                                                            "bg-primary/30"
                                                        },
                                                        "group-hover:bg-primary"
                                                    )}
                                                    style={if is_empty {
                                                        "height: 2px".to_string()
                                                    } else {
                                                        format!("height: {}%", pct)
                                                    }}
                                                ></div>
                                                /* Pinned to the top of the plot, not the bar: a tall
                                                   bar used to push the tooltip over the summary row. */
                                                <div class={classes!(
                                                    "opacity-0", "group-hover:opacity-100", "absolute", "top-1",
                                                    if is_first { "left-0" } else if is_latest { "right-0" } else { "left-1/2" },
                                                    if is_first || is_latest { "" } else { "-translate-x-1/2" },
                                                    "bg-inverse-surface", "text-inverse-on-surface", "px-2.5", "py-1",
                                                    "rounded-md", "text-[11px]", "font-bold", "whitespace-nowrap", "shadow-xl",
                                                    "font-code-md", "z-50", "pointer-events-none", "transition-all", "duration-150",
                                                    "border", "border-outline/20"
                                                )}>
                                                    <div>{count_label}</div>
                                                    <div class="mt-0.5 text-[10px] font-medium opacity-70 font-body-sm">{window}</div>
                                                </div>
                                            </div>
                                        }
                                    }) }
                                </>
                            }
                        }
                    }
                </div>

                /* Time axis: the stats endpoint zero-fills the window, so the
                   first and last bars are the exact window edges. */
                <div class="absolute bottom-1 left-4 right-4 flex justify-between text-[10px] font-code-md text-on-surface-variant/70 pointer-events-none">
                    <span>{first_bucket_label.unwrap_or_default()}</span>
                    <span class={classes!(if bucket_slice.len() < 2 { "invisible" } else { "" })}>
                        {last_bucket_label.unwrap_or_default()}
                    </span>
                </div>
            </div>

            /* Log Table */
            <main class="flex-1 overflow-y-auto bg-surface-container-lowest">
                <div class="w-full h-full flex flex-col justify-between">
                    <div class="overflow-x-auto">
                        <table class="w-full text-left border-collapse min-w-[1100px]">
                            <thead class="bg-surface-dim/30 sticky top-0 border-b border-outline-variant z-10 backdrop-blur-sm">
                                <tr>
                                    <th class="py-cell_padding_v px-cell_padding_h w-8"></th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Timestamp"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Severity"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Event"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Actor"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Target"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Result"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Duration"}</th>
                                    <th class="py-cell_padding_v px-cell_padding_h font-label-xs text-label-xs text-on-surface-variant uppercase tracking-wider font-bold">{"Client"}</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-outline-variant/40">
                                {
                                    if !*enabled {
                                        html! {
                                            <tr>
                                                <td class="py-12 text-center" colspan={COLUMN_COUNT.to_string()}>
                                                    <div class="opacity-60 flex flex-col items-center gap-1">
                                                        <span class="material-symbols-outlined text-[48px] text-outline">{"database_off"}</span>
                                                        <p class="font-body-sm text-body-sm">
                                                            {"Database logging is disabled on this server."}
                                                        </p>
                                                        <p class="font-code-md text-code-md text-on-surface-variant">
                                                            {"Enable [logging.database] in crabbase.toml and restart."}
                                                        </p>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    } else if let Some(err) = &*error {
                                        html! {
                                            <tr>
                                                <td class="py-12 text-center" colspan={COLUMN_COUNT.to_string()}>
                                                    <div class="flex flex-col items-center gap-1 text-error">
                                                        <span class="material-symbols-outlined text-[48px]">{"error"}</span>
                                                        <p class="font-body-sm text-body-sm">{err.clone()}</p>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    } else if *loading && entries.is_empty() {
                                        html! {
                                            <tr>
                                                <td class="py-12 text-center" colspan={COLUMN_COUNT.to_string()}>
                                                    <div class="opacity-40 flex flex-col items-center">
                                                        <span class="material-symbols-outlined text-[48px] mb-2 text-outline animate-spin">{"progress_activity"}</span>
                                                        <p class="font-body-sm text-body-sm italic">{"Loading activity..."}</p>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    } else if entries.is_empty() {
                                        html! {
                                            <tr>
                                                <td class="py-12 text-center" colspan={COLUMN_COUNT.to_string()}>
                                                    <div class="opacity-40 flex flex-col items-center">
                                                        <span class="material-symbols-outlined text-[48px] mb-2 text-outline">{"search_off"}</span>
                                                        <p class="font-body-sm text-body-sm italic">{"No activity logs match your filter criteria."}</p>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    } else {
                                        html! {
                                            { for entries.iter().map(|item| {
                                                let id = item.id.clone();
                                                let is_open = (*expanded_id).as_ref() == Some(&id);
                                                let toggle_row = toggle_row.clone();
                                                let row_id = id.clone();
                                                let is_failure = item.outcome != "success";
                                                let payload = has_payload(item);

                                                let subtitle = match &item.method {
                                                    Some(method) => format!("{method} {}", opt(&item.path)),
                                                    None => item.message.clone(),
                                                };
                                                let target = match (&item.target_collection, &item.target_id) {
                                                    (Some(collection), Some(target_id)) => format!("{collection} / {target_id}"),
                                                    (Some(collection), None) => collection.clone(),
                                                    (None, Some(target_id)) => target_id.clone(),
                                                    (None, None) => "—".to_string(),
                                                };

                                                html! {
                                                    <>
                                                        <tr
                                                            onclick={Callback::from(move |_| toggle_row.emit(row_id.clone()))}
                                                            class={classes!(
                                                                "log-row", "hover:bg-surface-container-low", "transition-colors",
                                                                "group", "cursor-pointer", "align-top",
                                                                if is_open { "bg-surface-container" } else { "" },
                                                                if is_failure { "border-l-4 border-l-error/40" } else { "" }
                                                            )}
                                                        >
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <span class={classes!(
                                                                    "material-symbols-outlined", "text-[18px]", "text-on-surface-variant",
                                                                    "transition-transform", "duration-150",
                                                                    if is_open { "rotate-90" } else { "" }
                                                                )}>
                                                                    {"chevron_right"}
                                                                </span>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h text-on-surface-variant whitespace-nowrap font-code-md text-code-md">
                                                                {format_timestamp(&item.created, *show_local, *hour12)}
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <span class={classes!("px-2", "py-0.5", "rounded-full", "text-[10px]", "font-bold", "uppercase", "tracking-tight", "border", severity_class(&item.severity))}>
                                                                    {item.severity.clone()}
                                                                </span>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <div class="flex flex-col gap-0.5">
                                                                    <span class="font-code-md text-code-md font-bold text-on-background">
                                                                        {item.action.clone().unwrap_or_else(|| "—".to_string())}
                                                                    </span>
                                                                    <span class="font-code-md text-[11px] text-on-surface-variant">
                                                                        {subtitle}
                                                                    </span>
                                                                </div>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <div class="flex flex-col gap-0.5">
                                                                    <span class="font-code-md text-code-md text-on-background">
                                                                        {item.actor_id.clone().filter(|v| !v.is_empty()).unwrap_or_else(|| "anonymous".to_string())}
                                                                    </span>
                                                                    {
                                                                        if let Some(collection) = item.actor_collection.as_ref().filter(|v| !v.is_empty()) {
                                                                            html! { <span class="font-code-md text-[10px] text-on-surface-variant">{collection.clone()}</span> }
                                                                        } else {
                                                                            html! {}
                                                                        }
                                                                    }
                                                                </div>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <span class="font-code-md text-code-md text-on-surface-variant">{target}</span>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <div class="flex gap-1.5 flex-wrap">
                                                                    {
                                                                        if let Some(status) = item.status_code {
                                                                            html! {
                                                                                <span class={classes!("px-1.5", "py-0.5", "rounded", "text-[10px]", "font-medium", "border", status_class(status))}>
                                                                                    {status}
                                                                                </span>
                                                                            }
                                                                        } else {
                                                                            html! {}
                                                                        }
                                                                    }
                                                                    <span class={classes!("px-1.5", "py-0.5", "rounded", "text-[10px]", "font-medium", "border", outcome_class(&item.outcome))}>
                                                                        {item.outcome.clone()}
                                                                    </span>
                                                                </div>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h whitespace-nowrap">
                                                                <span class="px-1.5 py-0.5 rounded bg-surface-container text-on-surface-variant text-[10px] font-medium border border-outline-variant">
                                                                    {format_duration(item.duration_ms)}
                                                                </span>
                                                            </td>
                                                            <td class="py-cell_padding_v px-cell_padding_h">
                                                                <div class="flex items-center gap-1.5">
                                                                    <span class="font-code-md text-code-md text-on-surface-variant whitespace-nowrap">
                                                                        {opt(&item.ip_address)}
                                                                    </span>
                                                                    {
                                                                        if payload {
                                                                            html! {
                                                                                <span
                                                                                    class="px-1 py-0.5 rounded bg-primary/10 text-primary text-[9px] font-bold border border-primary/20"
                                                                                    title="This event carries a data payload"
                                                                                >
                                                                                    {"{ }"}
                                                                                </span>
                                                                            }
                                                                        } else {
                                                                            html! {}
                                                                        }
                                                                    }
                                                                </div>
                                                            </td>
                                                        </tr>
                                                        {
                                                            if is_open {
                                                                html! {
                                                                    <tr>
                                                                        <td colspan={COLUMN_COUNT.to_string()} class="p-0">
                                                                            { detail_panel(item, *show_local, *hour12) }
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            } else {
                                                                html! {}
                                                            }
                                                        }
                                                    </>
                                                }
                                            }) }
                                        }
                                    }
                                }
                            </tbody>
                        </table>
                    </div>

                    /* Pagination */
                    <div class="p-3 bg-surface border-t border-outline-variant flex justify-between items-center shrink-0 gap-4">
                        <span class="font-label-xs text-label-xs text-on-surface-variant uppercase font-bold tracking-widest whitespace-nowrap">
                            {format!("Showing {first_row}-{last_row} of {}", format_count(*total))}
                        </span>

                        <div class="flex items-center gap-2">
                            <button
                                disabled={*page <= 1}
                                onclick={{
                                    let change = on_page_change.clone();
                                    Callback::from(move |_| change(-1))
                                }}
                                class={classes!(
                                    "px-3", "py-1.5", "rounded-lg", "border", "border-outline-variant", "font-bold",
                                    "font-body-sm", "text-body-sm", "flex", "items-center", "gap-1", "transition-all",
                                    if *page <= 1 { "opacity-40 cursor-not-allowed" } else { "hover:bg-surface-container cursor-pointer text-primary" }
                                )}
                            >
                                <span class="material-symbols-outlined text-[18px]">{"chevron_left"}</span>
                                {"Prev"}
                            </button>

                            <span class="font-code-md text-code-md text-on-surface-variant px-2">
                                {format!("{} / {}", *page, total_pages)}
                            </span>

                            <button
                                disabled={*page >= total_pages}
                                onclick={{
                                    let change = on_page_change.clone();
                                    Callback::from(move |_| change(1))
                                }}
                                class={classes!(
                                    "px-3", "py-1.5", "rounded-lg", "border", "border-outline-variant", "font-bold",
                                    "font-body-sm", "text-body-sm", "flex", "items-center", "gap-1", "transition-all",
                                    if *page >= total_pages { "opacity-40 cursor-not-allowed" } else { "hover:bg-surface-container cursor-pointer text-primary" }
                                )}
                            >
                                {"Next"}
                                <span class="material-symbols-outlined text-[18px]">{"chevron_right"}</span>
                            </button>
                        </div>

                        <div class="flex items-center gap-2 font-label-xs text-label-xs text-on-surface-variant uppercase font-bold tracking-widest">
                            {"Rows:"}
                            <select
                                onchange={on_per_page_change}
                                value={per_page.to_string()}
                                class="bg-transparent border-none font-label-xs text-label-xs focus:ring-0 cursor-pointer text-primary font-bold"
                            >
                                { for PAGE_SIZES.iter().map(|size| html! {
                                    <option value={size.to_string()}>{size.to_string()}</option>
                                }) }
                            </select>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    }
}
