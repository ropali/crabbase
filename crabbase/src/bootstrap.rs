use crabbase_core::{config::Config, utils::string_utils::random_str};
use crabbase_db::connection::pool;
use sqlx::migrate;
use sqlx::{PgPool, migrate::Migrator};

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
    setup_superuser(
        &db_pool,
        config.admin_username.clone(),
        config.admin_password.clone(),
    )
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

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
    println!(
        "
        ██████╗██████╗  █████╗ ██████╗ ██████╗  █████╗ ███████╗███████╗
       ██╔════╝██╔══██╗██╔══██╗██╔══██╗██╔══██╗██╔══██╗██╔════╝██╔════╝
       ██║     ██████╔╝███████║██████╔╝██████╔╝███████║███████╗█████╗
       ██║     ██╔══██╗██╔══██║██╔══██╗██╔══██╗██╔══██║╚════██║██╔══╝
       ╚██████╗██║  ██║██║  ██║██████╔╝██████╔╝██║  ██║███████║███████╗
        ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚══════╝
    "
    );

    tracing::info!("version:  {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("database: connected ✓");
    if let Some(ref email) = config.admin_username {
        tracing::info!("superuser: {} configured ✓", email);
    }
    tracing::info!("admin UI: http://{}/admin", config.admin_bind_addr);
    tracing::info!("api:      http://{}/api", config.server_bind_addr);
}

async fn setup_superuser(
    db_pool: &sqlx::PgPool,
    email: Option<String>,
    password: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Delete the placeholder seed superuser if it exists
    sqlx::query("DELETE FROM _superusers WHERE password_hash = '$2b$12$placeholder_hash_replace_in_production'")
        .execute(db_pool)
        .await?;

    // Ensure all existing superusers are verified
    sqlx::query("UPDATE _superusers SET verified = true WHERE verified = false")
        .execute(db_pool)
        .await?;

    // Ensure the _superusers collection has a dynamically generated authToken secret
    let secret = random_str(None);
    sqlx::query("UPDATE _collections SET options = $1::jsonb WHERE name = '_superusers'")
        .bind(format!("{{\"authToken\": {{\"secret\": \"{}\"}}}}", secret))
        .execute(db_pool)
        .await?;

    // Check if any superuser already exists (in case user created one manually)
    let existing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _superusers")
        .fetch_one(db_pool)
        .await?;

    let email = email.unwrap_or_else(|| "admin@crabbase.local".to_string());

    if existing_count > 0 {
        if let Some(pw) = password {
            let password_hash = crabbase_auth::auth::hash_password(&pw)?;
            let token_key = uuid::Uuid::new_v4().to_string();

            let user_exists: Option<String> =
                sqlx::query_scalar("SELECT id FROM _superusers WHERE email = $1")
                    .bind(&email)
                    .fetch_optional(db_pool)
                    .await?;

            if let Some(id) = user_exists {
                sqlx::query("UPDATE _superusers SET password_hash = $1, token_key = $2, updated = now() WHERE id = $3")
                    .bind(password_hash)
                    .bind(token_key)
                    .bind(id)
                    .execute(db_pool)
                    .await?;
            } else {
                let id = format!("r{}", uuid::Uuid::new_v4().simple());
                let id = id.chars().take(15).collect::<String>();
                sqlx::query("INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)")
                    .bind(id)
                    .bind(&email)
                    .bind(password_hash)
                    .bind(token_key)
                    .bind(true)
                    .execute(db_pool)
                    .await?;
            }
        }
    } else {
        // No superusers exist, we MUST create one!
        let pw = password.unwrap_or_else(|| {
            uuid::Uuid::new_v4()
                .to_string()
                .chars()
                .take(12)
                .collect::<String>()
        });

        let password_hash = crabbase_auth::auth::hash_password(&pw)?;
        let token_key = uuid::Uuid::new_v4().to_string();
        let id = format!("r{}", uuid::Uuid::new_v4().simple());
        let id = id.chars().take(15).collect::<String>();

        sqlx::query(
            "INSERT INTO _superusers (id, email, password_hash, token_key, verified) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(&email)
        .bind(password_hash)
        .bind(token_key)
        .bind(true)
        .execute(db_pool)
        .await?;
    }

    Ok(())
}
