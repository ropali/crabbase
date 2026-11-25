use std::collections::HashSet;

use crabbase_core::utils::string_utils::{quote_ident, random_str};
use crabbase_db::connection::pool;
use crabbase_db::repositories::auth::AuthUser;
use sqlx::migrate;
use sqlx::{PgPool, migrate::Migrator};

use crate::config::{Config, InitialUser, LoggingDatabaseSection};
use crate::errors::AppError;

// Embed all .sql files from migrations/ into the binary at compile time
static MIGRATOR: Migrator = migrate!("../migrations");

// The audit-log schema lives in its own migration directory: it has its own
// `_sqlx_migrations` history (in the log schema) and can move to a separate
// database later without touching the application migrations.
static LOG_MIGRATOR: Migrator = migrate!("../migrations-logs");

pub async fn bootstrap(config: &Config) -> Result<PgPool, AppError> {
    let db_pool = pool(&config.database_url, &config.database.schema)
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Verify database connection
    verify_connection(&db_pool, &config.database_url).await?;

    // The schema must exist before the migrator can put its bookkeeping table
    // (`_sqlx_migrations`) there.
    ensure_schema(&db_pool, &config.database.schema).await?;

    // Run migrations
    run_migrations(&db_pool).await?;

    // setup superuser

    if let Some(intial_users) = &config.initial_users {
        setup_superuser(&db_pool, intial_users)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    }

    // Print Info
    print_startup_info(config);

    Ok(db_pool)
}

/// Migrate the audit-log target. Called BEFORE `init_logging` so the DDL
/// cannot race the writer's first flush. No-op when database logging is off.
///
/// The connection comes from `[logging.database]`, so a dedicated log database
/// (recommended), a separate schema in the application database, or even the
/// application schema itself all work. Creating the *database* is the user's
/// job; the schema is created here.
pub async fn run_log_migrations(config: &Config) -> Result<(), AppError> {
    let Some(section) = config.logging.database.as_ref().filter(|d| d.enabled) else {
        return Ok(());
    };

    let setup = section.to_setup();
    let log_pool =
        pool(&setup.database_url, &setup.schema).map_err(|e| AppError::Database(e.to_string()))?;

    verify_log_connection(&log_pool, section).await?;
    ensure_schema(&log_pool, &setup.schema).await?;

    if shares_app_schema(config, &log_pool).await? {
        // Two sqlx migrators cannot share one schema: both would want the same
        // `_sqlx_migrations` table. Apply the log DDL directly instead — it is
        // written with IF NOT EXISTS, so re-running on every boot is a no-op.
        eprintln!(
            "Warning: logs share the application schema `{}`. Log migrations are applied \
             directly and must stay idempotent. A dedicated schema or database is recommended.",
            setup.schema
        );
        apply_log_ddl(&log_pool).await?;
    } else {
        LOG_MIGRATOR
            .run(&log_pool)
            .await
            .map_err(|e| AppError::Database(format!("log migration failed: {e}")))?;
    }

    log_pool.close().await;

    Ok(())
}

async fn verify_log_connection(
    pool: &PgPool,
    section: &LoggingDatabaseSection,
) -> Result<(), AppError> {
    sqlx::query("SELECT 1").fetch_one(pool).await.map_err(|e| {
        AppError::Database(format!(
            r#"
            Cannot connect to the log database `{}` at {}:{}.

            Create it first, then restart:
                CREATE DATABASE {};

            Or point [logging.database] at an existing database/schema.

            Underlying error: {e}
        "#,
            section.database, section.host, section.port, section.database
        ))
    })?;

    Ok(())
}

/// True when the log pool and the application pool resolve unqualified names
/// in the same place (same server + database + schema).
async fn shares_app_schema(config: &Config, log_pool: &PgPool) -> Result<bool, AppError> {
    let app_pool = pool(&config.database_url, &config.database.schema)
        .map_err(|e| AppError::Database(e.to_string()))?;

    let app = server_identity(&app_pool).await?;
    let logs = server_identity(log_pool).await?;

    app_pool.close().await;

    Ok(app == logs)
}

/// The identity of the namespace a connection writes to. `current_schema()`
/// is NULL when the configured schema does not exist yet, which naturally
/// compares unequal.
async fn server_identity(
    pool: &PgPool,
) -> Result<(Option<String>, Option<i32>, String, Option<String>), AppError> {
    sqlx::query_as(
        r#"
        SELECT
            inet_server_addr()::text,
            inet_server_port(),
            current_database()::text,
            current_schema()::text
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

/// Apply the log migrations as plain, idempotent DDL (no history table). Only
/// used when logs share the application schema, where a second
/// `_sqlx_migrations` is impossible.
async fn apply_log_ddl(pool: &PgPool) -> Result<(), AppError> {
    for migration in LOG_MIGRATOR
        .iter()
        .filter(|m| m.migration_type.is_up_migration())
    {
        sqlx::raw_sql(migration.sql.as_ref())
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::Database(format!(
                    "log migration {} ({}) failed: {e}",
                    migration.version, migration.description
                ))
            })?;
    }

    Ok(())
}

async fn verify_connection(pool: &PgPool, url: &str) -> Result<(), AppError> {
    sqlx::query("SELECT 1").fetch_one(pool).await.map_err(|_| {
        AppError::Database(format!(
            r#"
            Cannot connect to Postgres.

            Tried: {url}

            Common fixes:
                • Check DATABASE_URL is set correctly
                • Ensure Postgres is running and reachable
                • For SSL issues try: ?sslmode=require or ?sslmode=disable
                • Check firewall / VPC rules if using a cloud provider
        "#
        ))
    })?;

    Ok(())
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| AppError::Database(format!("migration failed: {e}")))
}

async fn ensure_schema(pool: &PgPool, schema: &str) -> Result<(), AppError> {
    if schema == "public" {
        return Ok(());
    }

    sqlx::query(&format!(
        "CREATE SCHEMA IF NOT EXISTS {}",
        quote_ident(schema)
    ))
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("failed to create schema `{schema}`: {e}")))?;

    Ok(())
}

fn print_startup_info(config: &Config) {
    tracing::info!("version:  {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("database: connected ✓");
    tracing::info!("schema: {} ✓", config.database.schema);
    tracing::info!("Database: Migration Applied ✓");

    if let Some(logging) = config.logging.database.as_ref().filter(|d| d.enabled) {
        tracing::info!("Log target: {}/{} ✓", logging.database, logging.schema);
    }

    if let Some(users) = &config.initial_users {
        tracing::info!("Initial Superuser: {} configured ✓", users.len(),);
    }
}

async fn setup_superuser(
    db_pool: &sqlx::PgPool,
    users: &Vec<InitialUser>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let emails: Vec<&str> = users.iter().map(|u| u.email.as_str()).collect();

    // Check if any superuser already exists (in case user created one manually)
    let existing_users =
        sqlx::query_as::<_, AuthUser>("SELECT * FROM _superusers WHERE email = ANY($1)")
            .bind(&emails)
            .fetch_all(db_pool)
            .await?;

    let existing_emails: HashSet<&str> = existing_users.iter().map(|u| u.email.as_str()).collect();

    for user in users {
        if existing_emails.contains(user.email.as_str()) {
            continue;
        }

        let pw = &user.password;
        let password_hash = crabbase_auth::auth::hash_password(&pw)?;
        let token_key = random_str(None);
        let id = uuid::Uuid::new_v4();

        sqlx::query(
                        "INSERT INTO _superusers (id, email, password, token_key, verified) VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(id)
                    .bind(&user.email)
                    .bind(password_hash)
                    .bind(token_key)
                    .bind(true)
                    .execute(db_pool)
                    .await?;
    }

    Ok(())
}
