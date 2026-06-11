use std::str::FromStr;

use sqlx::{PgPool, Pool, Postgres, postgres::PgConnectOptions};

pub fn pool(db_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    let opts = PgConnectOptions::from_str(db_url)?;

    let pool = PgPool::connect_lazy_with(opts);

    Ok(pool)
}
