use clap::Parser;
use crabbase_api::routes::get_app_routes;
use sqlx::migrate;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{error, info, info_span};

use crabbase_api::state::AppState;
use crabbase_core::logging::init_logging;
use crabbase_db::connection::pool;

use crate::cli::{Cli, Commands};
pub mod cli;

#[tokio::main]
async fn main() {
    let _log_guard = init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, host } => {
            if let Err(err) = run_server(host, port).await {
                error!(error = %err, "Application failed to start.");
                std::process::exit(1)
            }
        }
        Commands::Admin { port, host } => {
            if let Err(err) = run_admin(host, port).await {
                error!(error = %err, "Admin dashboard failed to start.");
                std::process::exit(1)
            }
        }
    };
}

async fn run_server(
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Crabbase Application...");
    let db_pool = pool().await?;

    migrate!("../migrations").run(&db_pool).await?;

    info!("Running migrations...");

    if let Err(err) = setup_superuser(&db_pool).await {
        error!(error = %err, "Failed to setup default superuser.");
        return Err(err);
    }

    let app_state = AppState { db: db_pool };

    let api = get_app_routes(app_state).layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
        info_span!("http_request", method = %req.method(), path = %req.uri().path())
    }));

    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("Started server at http://{}:{}", host, port);
    axum::serve(listener, api).await?;

    Ok(())
}

async fn setup_superuser(
    db_pool: &sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Delete the placeholder seed superuser if it exists
    sqlx::query("DELETE FROM _superusers WHERE password_hash = '$2b$12$placeholder_hash_replace_in_production'")
        .execute(db_pool)
        .await?;

    let email = std::env::var("CRABBASE_SUPERUSER_EMAIL")
        .unwrap_or_else(|_| "admin@crabbase.local".to_string());

    let password = std::env::var("CRABBASE_SUPERUSER_PASSWORD").ok();

    // Check if any superuser already exists (in case user created one manually)
    let existing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _superusers")
        .fetch_one(db_pool)
        .await?;

    if existing_count > 0 {
        if let Some(pw) = password {
            info!(
                "Superuser(s) found. Updating password for default superuser '{}'...",
                email
            );
            let password_hash = crabbase_auth::auth::hash_password(&pw)?;
            let token_key = uuid::Uuid::new_v4().to_string();

            let user_exists: Option<String> =
                sqlx::query_scalar("SELECT id FROM _superusers WHERE email = ?")
                    .bind(&email)
                    .fetch_optional(db_pool)
                    .await?;

            if let Some(id) = user_exists {
                sqlx::query("UPDATE _superusers SET password_hash = ?, token_key = ?, updated = strftime('%Y-%m-%d %H:%M:%fZ') WHERE id = ?")
                    .bind(password_hash)
                    .bind(token_key)
                    .bind(id)
                    .execute(db_pool)
                    .await?;
            } else {
                let id = format!("r{}", uuid::Uuid::new_v4().simple());
                let id = id.chars().take(15).collect::<String>();
                sqlx::query("INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES (?, ?, ?, ?, ?)")
                    .bind(id)
                    .bind(&email)
                    .bind(password_hash)
                    .bind(token_key)
                    .bind(1)
                    .execute(db_pool)
                    .await?;
            }
            info!(
                "Default superuser '{}' password updated successfully.",
                email
            );
        } else {
            info!("Superuser(s) already configured in database.");
        }
    } else {
        // No superusers exist, we MUST create one!
        let pw = match password {
            Some(pw) => pw,
            None => {
                let gen_pw: String = uuid::Uuid::new_v4().to_string().chars().take(12).collect();
                info!("====================================================");
                info!("No superusers found in database.");
                info!("Creating default superuser:");
                info!("  Email:    {}", email);
                info!("  Password: {}", gen_pw);
                info!("PLEASE RECORD THIS PASSWORD. IT WILL NOT BE SHOWN AGAIN.");
                info!("====================================================");
                gen_pw
            }
        };

        let password_hash = crabbase_auth::auth::hash_password(&pw)?;
        let token_key = uuid::Uuid::new_v4().to_string();
        let id = format!("r{}", uuid::Uuid::new_v4().simple());
        let id = id.chars().take(15).collect::<String>();

        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&email)
        .bind(password_hash)
        .bind(token_key)
        .execute(db_pool)
        .await?;

        info!("Default superuser '{}' created successfully.", email);
    }

    Ok(())
}

async fn run_admin(
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Admin dashboard on {}:{}", host, port);

    // TODO: implement admin dashboard
    // Simulating admin command running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Admin dashboard...");

    Ok(())
}
