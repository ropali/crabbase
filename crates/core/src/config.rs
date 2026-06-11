use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_bind_addr: String,
    pub max_db_conn: u32,
    pub admin_bind_addr: String,
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if let Err(e) = dotenvy::dotenv() {
            if let dotenvy::Error::Io(io_err) = &e
                && io_err.kind() == std::io::ErrorKind::NotFound
            {
                return Ok(Self::from_env());
            }
            return Err(Box::new(e));
        }

        Ok(Self::from_env())
    }

    fn from_env() -> Self {
        let defaults = Self::default();

        let database_url = env::var("DATABASE_URL")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or(defaults.database_url);

        let server_bind_addr = env::var("SERVER_BIND_ADDR")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or(defaults.server_bind_addr);

        let max_db_conn = env::var("MAX_DB_CONN")
            .ok()
            .filter(|v| !v.is_empty())
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(defaults.max_db_conn);

        let admin_addr = env::var("ADMIN_BIND_ADDR")
            .or_else(|_| env::var("ADMIN_BIND_ADD"))
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or(defaults.admin_bind_addr);

        let admin_username = env::var("ADMIN_USERNAME")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| defaults.admin_username.unwrap());

        let admin_password = env::var("ADMIN_PASSWORD")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                defaults
                    .admin_password
                    .expect("ADMIN_PASSWORD env variable is not set.")
            });

        Self {
            database_url,
            server_bind_addr,
            max_db_conn,
            admin_bind_addr: admin_addr,
            admin_username: Some(admin_username),
            admin_password: Some(admin_password),
        }
    }

    fn default() -> Self {
        Self {
            database_url: "postgres://postgres:postgres@localhost:5432/crabbase".to_string(),
            server_bind_addr: "0.0.0.0:8989".to_string(),
            max_db_conn: 10,
            admin_bind_addr: "0.0.0.0:9898".to_ascii_lowercase(),
            admin_username: Some("admin@crabbase.local".to_string()),
            admin_password: None,
        }
    }
}
