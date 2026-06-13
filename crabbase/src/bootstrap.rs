use crabbase_core::{config::Config, utils::string_utils::random_str};
use crabbase_db::connection::pool;
use crabbase_db::repositories::auth::AuthUser;
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
    tracing::info!("Database: Migration Applied ✓");
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
    let email = email.unwrap_or_else(|| "admin@crabbase.local".to_string());

    // Check if any superuser already exists (in case user created one manually)
    let existing_user = sqlx::query_as::<_, AuthUser>("SELECT * FROM _superusers WHERE email = $1")
        .bind(&email)
        .fetch_one(db_pool)
        .await;

    match existing_user {
        Ok(user) => {
            // Check if user has token key set
            if user.token_key.is_empty() {
                sqlx::query("UPDATE _superusers SET token_key = $1 WHERE email = $2")
                    .bind(random_str(None))
                    .bind(email)
                    .execute(db_pool)
                    .await?;
            }
        }
        Err(_) => {
            // No superusers exist, we MUST create one!
            let pw = password.expect("Admin password not set while creating superuser");
            let password_hash = crabbase_auth::auth::hash_password(&pw)?;
            let token_key = random_str(None);
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
    }

    Ok(())
}
