use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    extract::{ConnectInfo, MatchedPath, Request},
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
    fn set_actor(&self, id: &str, collection: &str) {
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
    let path = req.uri().path().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_string());
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
