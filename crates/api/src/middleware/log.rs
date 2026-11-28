use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    extract::{ConnectInfo, MatchedPath, OriginalUri, Request},
    http::{Extensions, Method, StatusCode, header},
    middleware::Next,
    response::Response,
};
use tracing::Level;

#[derive(Clone, Default)]
pub struct RequestLog {
    actor: Arc<Mutex<Option<Actor>>>,
}

#[derive(Clone, Debug)]
struct Actor {
    id: String,
    collection: String,
}

impl RequestLog {
    /// Public within the crate so the login handler can attribute the row it
    /// produces; everything else goes through [`record_actor`].
    pub(crate) fn set_actor(&self, id: &str, collection: &str) {
        if let Ok(mut slot) = self.actor.lock() {
            *slot = Some(Actor {
                id: id.into(),
                collection: collection.into(),
            });
        }
    }

    fn actor(&self) -> Option<Actor> {
        self.actor.lock().ok().and_then(|slot| slot.clone())
    }
}

pub fn record_actor(extensions: &mut Extensions, actor_id: &str, actor_collection: &str) {
    if let Some(log) = extensions.get::<RequestLog>() {
        log.set_actor(actor_id, actor_collection);
    }
}

pub async fn log_requests(mut req: Request, next: Next) -> Response {
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    let started = Instant::now();
    let method = req.method().clone();

    // Inside a `nest` the request URI has the prefix stripped, while
    // `MatchedPath` keeps it. `OriginalUri` is the path the client actually
    // asked for, so it is both the truthful `path` column and the only thing
    // that lines up segment-for-segment with the route template.
    let path = req
        .extensions()
        .get::<OriginalUri>()
        .map(|uri| uri.0.path().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    // Skip the /logs API path
    if path.contains("/api/logs") {
        return next.run(req).await;
    }

    // The route template (`/api/collections/{name}/records/{id}`), not the
    // concrete URL: low cardinality, safe to group by, no ids or slugs.
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_string());

    let params = route
        .as_deref()
        .map(|template| path_params(template, &path))
        .unwrap_or_default();

    let target_collection = param_value(&params, "name");
    let target_id = param_value(&params, "id");
    let ip_address = client_ip(&req);
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let log = RequestLog::default();
    req.extensions_mut().insert(log.clone());

    let response = next.run(req).await;

    let status = response.status();
    let elapsed = started.elapsed();
    let actor = log.actor();
    let action = action_for(&method, route.as_deref().unwrap_or(&path));

    let outcome = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => "denied",
        s if s.is_client_error() || s.is_server_error() => "failure",
        _ => "success",
    };

    // `event!` needs a literal level, so the field list lives in a macro and
    // each branch supplies its own `Level::`.
    macro_rules! emit {
        ($lvl:expr) => {
            tracing::event!(
                target: "crabbase_api::request",
                $lvl,
                action = %action,
                method = %method,
                path = %path,
                status_code = status.as_u16(),
                duration_ms = elapsed.as_secs_f64() * 1000.0,
                ip_address = %ip_address.as_deref().unwrap_or(""),
                user_agent = %user_agent.as_deref().unwrap_or(""),
                actor_id = %actor.as_ref().map(|a| a.id.as_str()).unwrap_or(""),
                actor_collection = %actor.as_ref().map(|a| a.collection.as_str()).unwrap_or(""),
                target_collection = %target_collection.as_deref().unwrap_or(""),
                target_id = %target_id.as_deref().unwrap_or(""),
                route = %route.as_deref().unwrap_or(""),
                outcome = outcome,
                "http request"
            )
        };
    }

    if status.is_server_error() {
        emit!(Level::ERROR);
    } else if status.is_client_error() {
        emit!(Level::WARN);
    } else {
        emit!(Level::INFO);
    }

    response
}

fn action_for(method: &Method, route: &str) -> String {
    let path = route.strip_prefix("/api").unwrap_or(route);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let last_static = segments.iter().rev().find(|s| !s.starts_with('{'));

    if segments.first() == Some(&"auth") {
        return format!("auth.{}", last_static.copied().unwrap_or("request"));
    }

    let resource = match segments.first().copied() {
        Some("collections") | Some("collection") => {
            if segments.iter().any(|s| *s == "records" || *s == "record") {
                "record"
            } else if segments.iter().any(|s| *s == "files") {
                "file"
            } else {
                "collection"
            }
        }
        Some("files") => "file",
        Some("settings") => "setting",
        Some("jobs") => "job",
        Some("crons") => "cron",
        Some("backups") => "backup",
        Some(other) => other,
        None => "request",
    };

    let verb = match *method {
        Method::POST => match last_static.copied() {
            Some("truncate") => "truncate",
            Some("upload") => "upload",
            _ => "create",
        },
        Method::PATCH | Method::PUT => "update",
        Method::DELETE => "delete",
        Method::GET | Method::HEAD => {
            if segments.last().is_some_and(|s| s.starts_with('{')) {
                "view"
            } else {
                "list"
            }
        }
        _ => "request",
    };

    format!("{resource}.{verb}")
}

/// Pairs the `{param}` segments of a matched route template with the concrete
/// segments of the requested path.
///
/// `/api/collections/{name}/records/{id}` against
/// `/api/collections/demo/records/abc` yields `[("name","demo"), ("id","abc")]`.
/// A trailing `{*rest}` capture is matched against the first remaining
/// segment only; it is always the last capture in axum, so nothing after it
/// can be misaligned.
fn path_params(template: &str, path: &str) -> Vec<(String, String)> {
    template
        .split('/')
        .zip(path.split('/'))
        .filter_map(|(segment, value)| {
            let name = segment.strip_prefix('{')?.strip_suffix('}')?;
            let name = name.strip_prefix('*').unwrap_or(name);
            (!name.is_empty() && !value.is_empty()).then(|| (name.to_string(), value.to_string()))
        })
        .collect()
}

fn param_value(params: &[(String, String)], key: &str) -> Option<String> {
    params
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

/// Client address for the audit trail.
///
/// Proxy headers win because in the common deployment (container behind nginx,
/// Caddy, or a cloud load balancer) the socket address is the proxy itself.
/// Only trust these if the edge strips client-supplied values.
fn client_ip(req: &Request) -> Option<String> {
    if let Some(forwarded) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|first| !first.is_empty())
    {
        return Some(forwarded.to_string());
    }

    if let Some(real_ip) = req
        .headers()
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(real_ip.to_string());
    }

    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_template_drives_the_action() {
        assert_eq!(
            action_for(&Method::POST, "/api/collections/{name}/records"),
            "record.create"
        );
        assert_eq!(
            action_for(&Method::GET, "/api/collections/{name}/records/{id}"),
            "record.view"
        );
        assert_eq!(
            action_for(&Method::POST, "/api/collections/{name}/truncate"),
            "collection.truncate"
        );
        assert_eq!(
            action_for(&Method::POST, "/api/auth/{name}/login"),
            "auth.login"
        );
    }

    #[test]
    fn path_params_align_template_with_request() {
        let params = path_params(
            "/api/collections/{name}/records/{id}",
            "/api/collections/demo/records/abc",
        );

        assert_eq!(param_value(&params, "name").as_deref(), Some("demo"));
        assert_eq!(param_value(&params, "id").as_deref(), Some("abc"));

        assert!(path_params("/api/logs", "/api/logs").is_empty());
    }

    #[test]
    fn wildcard_params_use_the_first_segment() {
        let params = path_params(
            "/api/collections/{name}/files/{*file_id}",
            "/api/collections/demo/files/2026/11/a.png",
        );

        assert_eq!(param_value(&params, "name").as_deref(), Some("demo"));
        assert_eq!(param_value(&params, "file_id").as_deref(), Some("2026"));
    }
}
