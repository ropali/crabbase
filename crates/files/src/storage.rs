use async_trait::async_trait;
use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, config::Config, presigning::PresigningConfig, primitives::ByteStream};
use crabbase_db::repositories::files::FileBackend;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug)]
pub enum StorageError {
    Upload(String),
    Download(String),
    Delete(String),
    Presign(String),
    NotFound(String),
    Config(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upload(msg) => write!(f, "Upload error: {msg}"),
            Self::Download(msg) => write!(f, "Download error: {msg}"),
            Self::Delete(msg) => write!(f, "Delete error: {msg}"),
            Self::Presign(msg) => write!(f, "Presign error: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::Config(msg) => write!(f, "Storage configuration error: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[serde(alias = "S3", alias = "s3")]
    S3,
    #[default]
    #[serde(alias = "Local", alias = "local")]
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub region: String,
    #[serde(
        rename = "accessKey",
        alias = "access_key_id",
        alias = "accessKeyId",
        default
    )]
    pub access_key_id: String,
    #[serde(
        rename = "secret",
        alias = "secret_access_key",
        alias = "secretAccessKey",
        default
    )]
    pub secret_access_key: String,
    #[serde(
        rename = "endpoint",
        alias = "endpoint_url",
        alias = "endpointUrl",
        default
    )]
    pub endpoint_url: Option<String>,
    #[serde(default)]
    pub force_path_style: bool,
}

#[async_trait]
pub trait FileStorage: Send + Sync {
    /// Engine backing this instance. Recorded on every file written through it
    /// so the file can be resolved again later, independently of whichever
    /// backend the active storage config points at.
    fn backend(&self) -> FileBackend;

    /// Local base path, or S3 bucket name. Persisted alongside the file as
    /// `_files.container`.
    fn container(&self) -> Cow<'_, str>;

    /// Uploads an object payload to storage.
    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, StorageError>;

    /// Fetches an object payload from storage.
    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Deletes an object from storage.
    async fn delete_object(&self, key: &str) -> Result<(), StorageError>;

    /// Generates a presigned PUT URL for direct client uploads.
    /// Only supported by S3 storage.
    async fn presign_put(
        &self,
        _key: &str,
        _expires_in: Duration,
        _content_type: &str,
    ) -> Result<String, StorageError> {
        Err(StorageError::Presign(
            "Presigned URLs are not supported by this storage backend".to_string(),
        ))
    }
}

#[derive(Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    pub fn new(config: S3Config) -> Self {
        let credentials = Credentials::new(
            config.access_key_id,
            config.secret_access_key,
            None,
            None,
            "crabbase",
        );

        let mut builder = Config::builder()
            .behavior_version_latest()
            .region(Region::new(config.region))
            .credentials_provider(credentials)
            .force_path_style(config.force_path_style);

        if let Some(endpoint) = config.endpoint_url {
            builder = builder.endpoint_url(endpoint);
        }

        let client = Client::from_conf(builder.build());
        Self {
            client,
            bucket: config.bucket,
        }
    }

    pub fn from_client(client: Client, bucket: impl Into<String>) -> Self {
        Self {
            client,
            bucket: bucket.into(),
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}

#[async_trait]
impl FileStorage for S3Storage {
    fn backend(&self) -> FileBackend {
        FileBackend::S3
    }

    fn container(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.bucket)
    }

    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| StorageError::Upload(e.to_string()))?;

        Ok(key.to_string())
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("NotFound") {
                    StorageError::NotFound(key.to_string())
                } else {
                    StorageError::Download(err_str)
                }
            })?;

        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| StorageError::Download(e.to_string()))?
            .into_bytes();

        Ok(bytes.to_vec())
    }

    async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Delete(e.to_string()))?;

        Ok(())
    }

    async fn presign_put(
        &self,
        key: &str,
        expires_in: Duration,
        content_type: &str,
    ) -> Result<String, StorageError> {
        let presign_config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::Presign(e.to_string()))?;

        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presign_config)
            .await
            .map_err(|e| StorageError::Presign(e.to_string()))?;

        Ok(presigned.uri().to_string())
    }
}

fn default_local_path() -> PathBuf {
    PathBuf::from("./uploads")
}

/// Local filesystem storage implementation (fallback or local disk mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalStorage {
    #[serde(
        alias = "local_path",
        alias = "base_path",
        default = "default_local_path"
    )]
    base_path: PathBuf,
}

impl Default for LocalStorage {
    fn default() -> Self {
        Self {
            base_path: default_local_path(),
        }
    }
}

impl LocalStorage {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    pub fn base_path(&self) -> PathBuf {
        let s = self.base_path.to_string_lossy();
        if s.starts_with("$HOME") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(s.replacen("$HOME", &home, 1));
            }
        }
        self.base_path.clone()
    }
}

#[async_trait]
impl FileStorage for LocalStorage {
    fn backend(&self) -> FileBackend {
        FileBackend::Local
    }

    /// The configured path, not the `$HOME`-expanded one — it has to survive a
    /// round-trip through the database and rebuild the same storage later.
    fn container(&self) -> Cow<'_, str> {
        self.base_path.to_string_lossy()
    }

    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        _content_type: &str,
    ) -> Result<String, StorageError> {
        let path = self.base_path().join(key.trim_start_matches('/'));
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Upload(e.to_string()))?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| StorageError::Upload(e.to_string()))?;
        Ok(key.to_string())
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.base_path().join(key.trim_start_matches('/'));
        tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(key.to_string())
            } else {
                StorageError::Download(e.to_string())
            }
        })
    }

    async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        let path = self.base_path().join(key.trim_start_matches('/'));
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| StorageError::Delete(e.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    #[serde(default)]
    pub storage_type: StorageBackend,
    #[serde(default)]
    pub local: LocalStorage,
    #[serde(default)]
    pub s3: S3Config,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_storage_put_get_delete() {
        let temp_dir = std::env::temp_dir().join(format!("crabbase-test-{}", uuid::Uuid::new_v4()));
        let storage: Box<dyn FileStorage> = Box::new(LocalStorage::new(&temp_dir));

        let key = "test/sub/hello.txt";
        let data = b"Hello, Crabbase!".to_vec();

        // Put
        let saved_key = storage
            .put_object(key, data.clone(), "text/plain")
            .await
            .unwrap();
        assert_eq!(saved_key, key);

        // Get
        let retrieved = storage.get_object(key).await.unwrap();
        assert_eq!(retrieved, data);

        // Delete
        storage.delete_object(key).await.unwrap();

        // Verify Not Found
        let err = storage.get_object(key).await.unwrap_err();
        match err {
            StorageError::NotFound(_) => {}
            _ => panic!("Expected NotFound error, got {err:?}"),
        }

        // Cleanup
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[test]
    fn test_s3_storage_builder() {
        let cfg = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "test".to_string(),
            secret_access_key: "test".to_string(),
            endpoint_url: Some("http://localhost:4566".to_string()),
            force_path_style: true,
        };

        let storage = S3Storage::new(cfg);
        assert_eq!(storage.bucket(), "test-bucket");
    }

    #[test]
    fn test_storage_config_deserialization() {
        let json = r#"{
            "storageType": "local",
            "local": {
                "local_path": "$HOME/crabbase/files/"
            },
            "s3": {
                "endpoint": "",
                "bucket": "crabbase_files",
                "region": "us-east-1",
                "accessKey": "",
                "secret": ""
            }
        }"#;

        let cfg: StorageConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.storage_type, StorageBackend::Local);
        assert_eq!(cfg.s3.bucket, "crabbase_files");
    }
}
