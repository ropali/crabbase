//! Trace context carried onto audit events.
//!
//! `_logs` columns (`trace_id`, `span_id`, `request_id`) stay NULL until a
//! tracing backend or request middleware is wired up to populate these
//! values at the point an audit event is emitted.

#[derive(Debug, Clone, Default)]
pub struct TraceCtx {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub request_id: Option<String>,
}

impl TraceCtx {
    pub fn new(
        trace_id: Option<String>,
        span_id: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            request_id,
        }
    }
}
