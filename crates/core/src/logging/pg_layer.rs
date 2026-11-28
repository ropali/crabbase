use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::mpsc;
use tracing::{Event, Subscriber, level_filters::LevelFilter, metadata::Level};
use tracing_subscriber::{
    layer::{Context, Layer},
    registry::LookupSpan,
};

use super::event::{AuditEvent, Outcome, Severity};
use super::redact::redact;
use super::trace_ctx::TraceCtx;

/// `record_debug` for a `&str` field yields `"\"the value\""`. Strip the quotes.
fn clean_quotes(s: String) -> String {
    s.trim_matches('"').to_string()
}

/// Like [`clean_quotes`], but an empty value means "the field was not set".
///
/// The request middleware always emits every column because `tracing`'s
/// `Value` trait is sealed — a field cannot be omitted at the call site — and
/// uses `""` for the optional ones (`actor_id`, `ip_address`, …). Mapping the
/// empty string back to `None` keeps those columns `NULL`, so `IS NULL`
/// filters work and the `(actor_id, created DESC)` index is not dominated by
/// one giant `''` group.
fn optional(s: String) -> Option<String> {
    let value = clean_quotes(s);
    (!value.is_empty()).then_some(value)
}

/// A `data = %json_string` field arrives debug-quoted (`"{\"rows\":3}"`).
/// Parse the outer JSON string first, then the object it contains.
fn parse_json_field(raw: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(raw).ok()? {
        Value::String(text) => serde_json::from_str(&text).ok(),
        other => Some(other),
    }
}

#[derive(Default)]
pub struct FieldVisitor {
    message: Option<String>,
    action: Option<String>,
    actor_id: Option<String>,
    actor_collection: Option<String>,
    target_collection: Option<String>,
    target_id: Option<String>,
    method: Option<String>,
    path: Option<String>,
    status_code: Option<u16>,
    duration_ms: Option<f64>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    outcome: Option<Outcome>,
    request_id: Option<String>,
    data: Value,
}

impl FieldVisitor {
    /// `serde_json::Value`'s own `Default` is `Null`; `as_object_mut` on it
    /// would panic on the first extra field an event carries.
    fn new() -> Self {
        Self {
            data: json!({}),
            ..Self::default()
        }
    }
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn core::fmt::Debug) {
        let v = format!("{value:?}");

        match field.name() {
            "message" => self.message = Some(clean_quotes(v)),
            "action" => self.action = Some(clean_quotes(v)),
            "actor_id" => self.actor_id = optional(v),
            "actor_collection" => self.actor_collection = optional(v),
            "target_collection" => self.target_collection = optional(v),
            "target_id" => self.target_id = optional(v),
            "method" => self.method = optional(v),
            "path" => self.path = optional(v),
            "status_code" => self.status_code = v.parse().ok(),
            "duration_ms" => self.duration_ms = v.parse().ok(),
            "ip_address" => self.ip_address = optional(v),
            "user_agent" => self.user_agent = optional(v),
            "request_id" => self.request_id = optional(v),
            "data" => {
                if let Some(Value::Object(map)) = parse_json_field(&v) {
                    if let Some(target) = self.data.as_object_mut() {
                        target.extend(map);
                    }
                }
            }
            "outcome" => {
                self.outcome = match clean_quotes(v).as_str() {
                    "denied" => Some(Outcome::Denied),
                    "failure" => Some(Outcome::Failure),
                    _ => Some(Outcome::Success),
                }
            }
            // Everything else becomes a JSON key in `data`.
            // This is where "?field=…", "email=…", "collection=notes" end up.
            other => {
                self.data
                    .as_object_mut()
                    .expect("data initialised as object")
                    .insert(other.to_string(), Value::String(clean_quotes(v)));
            }
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.record_debug(field, &value);
    }
}

pub struct PgLayer {
    tx: mpsc::Sender<AuditEvent>,
    filter: Level,
    dropped: Arc<AtomicU64>,
}

impl PgLayer {
    pub fn new(tx: mpsc::Sender<AuditEvent>, filter: Level, dropped: Arc<AtomicU64>) -> Self {
        Self {
            tx,
            filter,
            dropped,
        }
    }
}

impl<S> Layer<S> for PgLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    /// Declare the database layer's own verbosity to the registry.
    ///
    /// Without this the global max-level is whatever the console/file
    /// `EnvFilter`s say, and `tracing` drops events *before* dispatching them
    /// to any layer. `RUST_LOG=warn` with `min_level = "info"` would then
    /// silently persist nothing, even though `on_event` never got the chance
    /// to accept the event.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::from_level(self.filter))
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let meta = event.metadata();

        if *meta.level() > self.filter {
            return;
        }

        // Read the trace context off the enclosing span

        let trace = ctx
            .event_span(event)
            .and_then(|span| span.extensions().get::<TraceCtx>().cloned());

        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let ev = AuditEvent {
            severity: Severity::from(meta.level()),
            action: visitor.action.unwrap_or_else(|| meta.target().to_string()),
            message: visitor.message.unwrap_or_default(),
            actor_id: visitor.actor_id,
            actor_collection: visitor.actor_collection,
            target_collection: visitor.target_collection,
            target_id: visitor.target_id,
            method: visitor.method,
            path: visitor.path,
            status_code: visitor.status_code,
            duration_ms: visitor.duration_ms,
            ip_address: visitor.ip_address,
            user_agent: visitor.user_agent,
            outcome: visitor.outcome.unwrap_or(Outcome::Success),
            data: redact(visitor.data),
            trace_id: trace.as_ref().and_then(|t| t.trace_id.clone()),
            span_id: trace.as_ref().and_then(|t| t.span_id.clone()),
            request_id: visitor.request_id,
        };

        if self.tx.try_send(ev).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_maps_empty_debug_strings_to_none() {
        // `record_debug` renders `""` as the two-character string `""`.
        assert_eq!(optional("\"\"".to_string()), None);
        assert_eq!(
            optional("\"user-1\"".to_string()).as_deref(),
            Some("user-1")
        );
        // Non-string values never carry quotes and must survive untouched.
        assert_eq!(optional("42".to_string()).as_deref(), Some("42"));
    }

    #[test]
    fn clean_quotes_strips_both_sides() {
        assert_eq!(clean_quotes("\"abc\"".to_string()), "abc");
        assert_eq!(clean_quotes("42".to_string()), "42");
    }

    #[test]
    fn parses_escaped_json_data_field() {
        // What `record_debug` produces for `data = %"{\"rows\":3}"`.
        let quoted = "\"{\\\"rows\\\":3}\"";
        assert_eq!(parse_json_field(quoted).unwrap()["rows"], 3);

        // A raw object (non-string field) also works.
        assert_eq!(parse_json_field("{\"rows\":3}").unwrap()["rows"], 3);
    }
}
