use std::str::FromStr;

use sqlx::{PgPool, Pool, Postgres, postgres::PgConnectOptions};

/// Build a lazy pool whose connections resolve unqualified names against
/// `schema` via the `search_path` startup option.
///
/// Setting it here (rather than with `SET search_path` after connecting) means
/// every pooled connection starts in the schema, including the one the
/// migration runner borrows. That is what makes `CREATE TABLE x` and
/// `_sqlx_migrations` land in the configured schema instead of `public`.
pub fn pool(db_url: &str, schema: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    let opts = PgConnectOptions::from_str(db_url)?.options([("search_path", schema)]);

    let pool = PgPool::connect_lazy_with(opts);

    Ok(pool)
}
