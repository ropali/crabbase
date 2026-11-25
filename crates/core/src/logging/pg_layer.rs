use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::mpsc;
use tracing::{Event, Subscriber, metadata::Level};
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
            "actor_id" => self.actor_id = Some(clean_quotes(v)),
            "actor_collection" => self.actor_collection = Some(clean_quotes(v)),
            "target_collection" => self.target_collection = Some(clean_quotes(v)),
            "target_id" => self.target_id = Some(clean_quotes(v)),
            "method" => self.method = Some(clean_quotes(v)),
            "path" => self.path = Some(clean_quotes(v)),
            "status_code" => self.status_code = v.parse().ok(),
            "duration_ms" => self.duration_ms = v.parse().ok(),
            "ip_address" => self.ip_address = Some(clean_quotes(v)),
            "user_agent" => self.user_agent = Some(clean_quotes(v)),
            "request_id" => self.request_id = Some(clean_quotes(v)),
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
                    .insert(other.to_string(), Value::String(v));
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
