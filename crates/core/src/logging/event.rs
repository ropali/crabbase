use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub severity: Severity,
    pub action: String,
    pub message: String,
    pub actor_id: Option<String>,
    pub actor_collection: Option<String>,
    pub target_collection: Option<String>,
    pub target_id: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<u16>,
    pub duration_ms: Option<f64>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub outcome: Outcome,
    pub data: Value,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum Outcome {
    Success,
    Denied,
    Failure,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Denied => "denied",
            Outcome::Failure => "failure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Severity {
    // Maps to the legacy "level INTEGER" so both columns agree
    pub fn as_level(self) -> i32 {
        match self {
            Severity::Trace => 0,
            Severity::Debug => 1,
            Severity::Info => 2,
            Severity::Warn => 3,
            Severity::Error => 4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Trace => "TRACE",
            Severity::Debug => "DEBUG",
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
            Severity::Error => "ERROR",
        }
    }
}

impl From<&tracing::Level> for Severity {
    fn from(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::TRACE => Severity::Trace,
            tracing::Level::DEBUG => Severity::Debug,
            tracing::Level::INFO => Severity::Info,
            tracing::Level::WARN => Severity::Warn,
            tracing::Level::ERROR => Severity::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_maps_to_legacy_level_integers() {
        assert_eq!(Severity::Trace.as_level(), 0);
        assert_eq!(Severity::Debug.as_level(), 1);
        assert_eq!(Severity::Info.as_level(), 2);
        assert_eq!(Severity::Warn.as_level(), 3);
        assert_eq!(Severity::Error.as_level(), 4);
    }

    #[test]
    fn severity_strings_match_the_db_column() {
        assert_eq!(Severity::Warn.as_str(), "WARN");
        assert_eq!(Severity::Error.as_str(), "ERROR");
    }

    #[test]
    fn tracing_levels_map_onto_severities() {
        assert_eq!(Severity::from(&tracing::Level::INFO), Severity::Info);
        assert_eq!(Severity::from(&tracing::Level::ERROR), Severity::Error);
    }

    #[test]
    fn outcome_strings_match_the_db_column() {
        assert_eq!(Outcome::Success.as_str(), "success");
        assert_eq!(Outcome::Denied.as_str(), "denied");
        assert_eq!(Outcome::Failure.as_str(), "failure");
    }
}
