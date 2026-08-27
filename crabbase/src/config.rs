use std::{env, os, path::Path};

use clap::builder::Str;
use serde::Deserialize;

fn read_from_env(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn parse_env_value(key: &str) -> String {
    if let Some(parsed_val) = read_from_env(&key.replacen("$", key, 1)) {
        return parsed_val;
    }

    key.to_string()
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FileConfig {
    pub database: Option<DatabaseSection>,
    pub server: Option<ServerSection>,
    pub admin: Option<AdminSection>,
    pub initial_users: Option<Vec<InitialUser>>,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct DatabaseSection {
    pub host: String,
    pub port: u16,
    pub user: String,
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
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_bind_addr: String,
    pub max_db_conn: u32,
    pub admin_bind_addr: String,
    pub initial_users: Option<Vec<InitialUser>>,
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

        let cfg = Config {
            database_url: db.make_url(),
            server_bind_addr: format!("{}:{}", server.host, server.port),
            max_db_conn: db.max_connections,
            admin_bind_addr: format!("{}:{}", admin.host, admin.port),
            initial_users: file_cfg.initial_users,
        };

        Ok(cfg)
    }
}
