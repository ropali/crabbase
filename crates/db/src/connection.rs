use std::str::FromStr;

use sqlx::{PgPool, Pool, Postgres, postgres::PgConnectOptions};

pub async fn pool() -> Result<Pool<Postgres>, sqlx::Error> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

    let opts = PgConnectOptions::from_str(&db_url)?;

    let pool = PgPool::connect_with(opts).await?;

    Ok(pool)
}
