use std::path::Path;
use std::time::Duration;

use crabbase_core::logging::PgLogSetup;
use crabbase_core::logging::WriterConfig;
use crabbase_core::utils::string_utils::is_valid_ident;
use serde::Deserialize;

fn deserialize_env_or_value<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if let Some(var_name) = s.strip_prefix("$") {
        std::env::var(var_name)
            .map_err(|_| serde::de::Error::custom(format!("env variable `{var_name}` not set.")))
    } else {
        Ok(s)
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FileConfig {
    pub database: Option<DatabaseSection>,
    pub server: Option<ServerSection>,
    pub admin: Option<AdminSection>,
    pub initial_users: Option<Vec<InitialUser>>,
    pub logging: Option<LoggingSection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseSection {
    pub host: String,
    pub port: u16,
    pub user: String,

    #[serde(deserialize_with = "deserialize_env_or_value")]
    pub password: String, // env var name holding the password
    pub database: String,
    pub schema: String,
    pub max_connections: u32,
}

impl Default for DatabaseSection {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: "DATABASE_PASSWORD".to_string(),
            database: "crabbase".to_string(),
            schema: "crabbase".to_string(),
            max_connections: 10,
        }
    }
}

impl DatabaseSection {
    pub fn make_url(&self) -> String {
        String::from(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        ))
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub host: String,
    pub port: u16,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8989,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct AdminSection {
    pub host: String,
    pub port: u16,
}

impl Default for AdminSection {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitialUser {
    pub name: String,
    pub email: String,

    #[serde(deserialize_with = "deserialize_env_or_value")]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LoggingSection {
    pub database: Option<LoggingDatabaseSection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingDatabaseSection {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(deserialize_with = "deserialize_env_or_value")]
    pub password: String,
    pub database: String,
    /// Schema holding `_logs` and its own `_sqlx_migrations` table. Works the
    /// same whether `host`/`database` point at the app database or a separate
    /// one — pointing them elsewhere just moves the log schema to that server.
    pub schema: String,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub queue_capacity: usize,
    pub pool_max_connections: u32,
    pub insert_timeout_ms: u64,
    pub synchronous_commit: bool,
    pub min_level: String,
}

impl Default for LoggingDatabaseSection {
    fn default() -> Self {
        Self {
            enabled: false,
            batch_size: 100,
            flush_interval_ms: 250,
            queue_capacity: 8_192,
            pool_max_connections: 2,
            insert_timeout_ms: 2_000,
            synchronous_commit: false,
            min_level: "warn".into(),
            host: String::default(),
            port: 5432,
            user: String::default(),
            password: String::default(),
            database: String::default(),
            schema: "logs".into(),
        }
    }
}

impl LoggingDatabaseSection {
    pub fn to_setup(&self) -> PgLogSetup {
        let database_url = format!(
            "postgres://{}:{}@{}/{}",
            self.user,
            self.password,
            format!("{}:{}", self.host, self.port),
            self.database
        );
        PgLogSetup {
            database_url: database_url.to_string(),
            schema: self.schema.clone(),
            writer: WriterConfig {
                batch_size: self.batch_size,
                flush_interval: Duration::from_millis(self.flush_interval_ms),
                queue_capacity: self.queue_capacity,
                pool_max_connections: self.pool_max_connections,
                insert_timeout: Duration::from_millis(self.insert_timeout_ms),
                sync_commit: self.synchronous_commit,
            },
            min_level: self.min_level.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("logging.database.host is required".into());
        }

        if self.port == 0 {
            return Err("logging.database.port is required".into());
        }

        if self.user.is_empty() {
            return Err("logging.database.user is required".into());
        }

        if self.password.is_empty() {
            return Err("logging.database.password is required".into());
        }

        if self.database.is_empty() {
            return Err("logging.database.database is required".into());
        }

        if !is_valid_ident(&self.schema) {
            return Err(format!(
                "logging.database.schema `{}` is not a valid Postgres identifier",
                self.schema
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database: DatabaseSection,
    pub server_bind_addr: String,
    pub max_db_conn: u32,
    pub admin_bind_addr: String,
    pub initial_users: Option<Vec<InitialUser>>,
    pub logging: LoggingSection,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Config, Box<dyn std::error::Error>> {
        let file = match path {
            Some(p) => {
                let raw = std::fs::read_to_string(p)
                    .map_err(|e| format!("failed to read config {}: {e}", p.display()))?;

                toml::from_str::<FileConfig>(&raw)
                    .map_err(|e| format!("invalid config {}: {e}", p.display()))?
            }
            None => FileConfig::default(),
        };

        Self::from_file(file)
    }

    fn from_file(file_cfg: FileConfig) -> Result<Config, Box<dyn std::error::Error>> {
        let db = file_cfg.database.expect("Database is not configured");
        let server = file_cfg.server.expect("Server is details not configured");
        let admin = file_cfg.admin.expect("Admin details not configured");

        if !is_valid_ident(&db.schema) {
            return Err(format!(
                "Invalid configuration: database.schema `{}` is not a valid Postgres identifier",
                db.schema
            )
            .into());
        }

        let logging = file_cfg.logging.unwrap_or_default();

        if let Some(logging_db) = &logging.database {
            if logging_db.enabled {
                logging_db
                    .validate()
                    .map_err(|e| format!("Invalid configuration: {e}"))?;
            }
        }

        let cfg = Config {
            database_url: db.make_url(),
            database: db.clone(),
            server_bind_addr: format!("{}:{}", server.host, server.port),
            max_db_conn: db.max_connections,
            admin_bind_addr: format!("{}:{}", admin.host, admin.port),
            initial_users: file_cfg.initial_users,
            logging: logging,
        };

        Ok(cfg)
    }
}
