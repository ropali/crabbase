use std::collections::HashSet;

use crabbase_core::utils::string_utils::random_str;
use crabbase_db::connection::pool;
use crabbase_db::repositories::auth::AuthUser;
use sqlx::migrate;
use sqlx::{PgPool, migrate::Migrator};

use crate::config::{Config, InitialUser};
use crate::errors::AppError;

// Embed all .sql files from migrations/ into the binary at compile time
static MIGRATOR: Migrator = migrate!("../migrations");

pub async fn bootstrap(config: &Config) -> Result<PgPool, AppError> {
    // Create DB connection pool
    let db_pool = pool(&config.database_url).map_err(|e| AppError::Database(e.to_string()))?;

    // Verify database connection
    verify_connection(&db_pool, &config.database_url).await?;

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

fn print_startup_info(config: &Config) {
    tracing::info!("version:  {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("database: connected ✓");
    tracing::info!("Database: Migration Applied ✓");
    if let Some(_) = config.initial_users {
        tracing::info!(
            "Intial Superuser: {} configured ✓",
            config.initial_users.iter().len(),
        );
    }
    tracing::info!("admin UI: http://{}/admin", config.admin_bind_addr);
    tracing::info!("api:      http://{}/api", config.server_bind_addr);
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
