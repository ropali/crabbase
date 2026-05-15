use sqlx::{Pool, Sqlite};

struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub fn new(p: Pool<Sqlite>) -> Self {
        Self { pool: p }
    }

    pub async fn create_table(name: &String) -> bool {
        let q = r#"
            CREATE TABLE IF NOT EXISTS $1 ();
        "#;

        true
    }
}
