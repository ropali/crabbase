use crabbase_core::errors::APIError;
use crabbase_db::repositories::settings::SettingsRepository;
use uuid::Uuid;

use crate::storage::{LocalStorage, StorageConfig};
use crate::{FileStorage, StorageTypes};
use crate::{S3Config, S3Storage};

pub struct FileUploadService {
    pub settings_repo: SettingsRepository,
}

impl FileUploadService {
    pub fn new(settings_repo: SettingsRepository) -> Self {
        Self { settings_repo }
    }

    pub async fn resolve_storage(&self) -> Result<Box<dyn FileStorage>, APIError> {
        let settings = self
            .settings_repo
            .get::<StorageConfig>("storage")
            .await
            .map_err(|e| APIError::Internal {
                message: format!(
                    "Failed to load storage
  settings: {e}"
                ),
                details: serde_json::Value::String("storage config is missing".to_string()),
            })?;

        if let Some(storage_cfg) = settings {
            match storage_cfg.storage_type {
                StorageTypes::S3 => {
                    let s3 = storage_cfg.s3;
                    if !s3.bucket.is_empty() && !s3.access_key_id.is_empty() {
                        return Ok(Box::new(S3Storage::new(S3Config {
                            bucket: s3.bucket,
                            region: s3.region,
                            access_key_id: s3.access_key_id,
                            secret_access_key: s3.secret_access_key,
                            endpoint_url: s3.endpoint_url.filter(|s| !s.is_empty()),
                            force_path_style: s3.force_path_style,
                        })));
                    } else {
                        return Err(APIError::Internal {
                            message: "S3 config is missing.".to_string(),
                            details: serde_json::Value::String(
                                "S3 storage type is selected but config is not set or missing"
                                    .to_string(),
                            ),
                        });
                    }
                }
                StorageTypes::Local => {
                    return Ok(Box::new(storage_cfg.local));
                }
            }
        }

        // Fallback default
        Ok(Box::new(LocalStorage::new("./uploads")))
    }

    /// Multipart upload: store bytes and return the storage key.
    pub async fn upload_bytes(
        &self,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<String, APIError> {
        let storage = self.resolve_storage().await?;
        let key = format!("uploads/{}/{}", Uuid::new_v4(), file_name);
        storage
            .put_object(&key, bytes, content_type)
            .await
            .map_err(|e| APIError::Internal {
                message: format!("put_object failed: {e}"),
                details: serde_json::Value::Null,
            })?;
        Ok(key)
    }

    pub async fn download(&self, key: &str) -> Result<Vec<u8>, APIError> {
        let storage = self.resolve_storage().await?;
        storage
            .get_object(key)
            .await
            .map_err(|e| APIError::NotFound {
                resource: format!("File not found {key}: {e}"),
            })
    }

    pub async fn delete(&self, key: &str) -> Result<(), APIError> {
        let storage = self.resolve_storage().await?;
        storage
            .delete_object(key)
            .await
            .map_err(|e| APIError::Internal {
                message: format!("delete_object failed: {e}"),
                details: serde_json::Value::Null,
            })
    }
}
