use crabbase_core::errors::APIError;
use crabbase_db::repositories::files::{FileBackend, FileRecord, FilesRepository, NewFile};
use crabbase_db::repositories::settings::SettingsRepository;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::storage::{LocalStorage, S3Config, S3Storage, StorageBackend, StorageConfig};
use crate::{FileStorage, StorageError};

/// Result of a successful upload. `record` is `None` only when the metadata
/// write failed after the bytes were already stored — see
/// [`FileUploadService::upload_bytes`].
#[derive(Debug, Clone, Serialize)]
pub struct UploadedFile {
    pub key: String,
    pub backend: FileBackend,
    pub record: Option<FileRecord>,
}

pub struct FileUploadService {
    pub settings_repo: SettingsRepository,
    pub files_repo: FilesRepository,
}

/// Strips a client-supplied filename down to a single safe path component.
///
/// The name is used to build the object key, so anything path-like has to go:
/// `../../etc/passwd` would otherwise be written outside the upload root on the
/// local backend, and would be echoed back in `Content-Disposition` on the way
/// out.
fn sanitize_file_name(file_name: &str) -> String {
    const MAX_NAME_LEN: usize = 255;

    let base = file_name.rsplit(['/', '\\']).next().unwrap_or("").trim();

    let cleaned: String = base.chars().filter(|c| !c.is_control()).collect();

    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return "unnamed".to_string();
    }

    if cleaned.len() > MAX_NAME_LEN {
        return cleaned.chars().take(MAX_NAME_LEN).collect();
    }

    cleaned
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn map_storage_error(op: &str, e: StorageError) -> APIError {
    match e {
        StorageError::NotFound(key) => APIError::NotFound {
            resource: format!("File {key}"),
        },
        other => APIError::Internal {
            message: format!("{op} failed: {other}"),
            details: serde_json::Value::Null,
        },
    }
}

impl FileUploadService {
    pub fn new(settings_repo: SettingsRepository, files_repo: FilesRepository) -> Self {
        Self {
            settings_repo,
            files_repo,
        }
    }

    /// The active storage config, or a local-on-disk default when unset.
    async fn load_storage_config(&self) -> Result<StorageConfig, APIError> {
        let settings = self
            .settings_repo
            .get::<StorageConfig>("storage")
            .await
            .map_err(|e| APIError::Internal {
                message: format!("Failed to load storage settings: {e}"),
                details: serde_json::Value::String("storage config is unreadable".to_string()),
            })?;

        Ok(settings.unwrap_or_default())
    }

    fn s3_from_config(config: S3Config, bucket: String) -> S3Storage {
        S3Storage::new(S3Config {
            bucket,
            region: config.region,
            access_key_id: config.access_key_id,
            secret_access_key: config.secret_access_key,
            endpoint_url: config.endpoint_url.filter(|s| !s.is_empty()),
            force_path_style: config.force_path_style,
        })
    }

    /// Where **new** uploads go. Driven by the active storage config alone —
    /// existing files never route through here.
    pub async fn resolve_storage(&self) -> Result<Box<dyn FileStorage>, APIError> {
        let config = self.load_storage_config().await?;

        match config.storage_type {
            StorageBackend::S3 => {
                if config.s3.bucket.is_empty() || config.s3.access_key_id.is_empty() {
                    return Err(APIError::Internal {
                        message: "S3 config is missing.".to_string(),
                        details: serde_json::Value::String(
                            "S3 storage type is selected but config is not set or missing"
                                .to_string(),
                        ),
                    });
                }
                Ok(Box::new(Self::s3_from_config(
                    config.s3.clone(),
                    config.s3.bucket.clone(),
                )))
            }
            StorageBackend::Local => Ok(Box::new(config.local)),
        }
    }

    /// Where an **existing** file lives, rebuilt from its `_files` row rather
    /// than from the active config. This is what keeps a file readable after
    /// the storage config changes out from under it.
    async fn storage_for(&self, row: &FileRecord) -> Result<Box<dyn FileStorage>, APIError> {
        match row.backend {
            FileBackend::Local => Ok(Box::new(LocalStorage::new(&row.container))),
            FileBackend::S3 => {
                // The bucket is per-file, but credentials and endpoint are
                // deployment-level, so those still come from the config.
                let config = self.load_storage_config().await?;
                if config.s3.access_key_id.is_empty() {
                    return Err(APIError::Internal {
                        message: "Cannot reach S3 file: S3 credentials are not configured"
                            .to_string(),
                        details: serde_json::Value::String(format!(
                            "file {} lives in bucket {} but the S3 block of the storage \
                             setting is empty",
                            row.key, row.container
                        )),
                    });
                }
                Ok(Box::new(Self::s3_from_config(
                    config.s3.clone(),
                    row.container.clone(),
                )))
            }
        }
    }

    /// Every backend that could hold a legacy file, most likely first. Used
    /// only for keys with no usable `_files` row, so the cost is paid rarely.
    async fn candidate_backends(&self) -> Result<Vec<Box<dyn FileStorage>>, APIError> {
        let config = self.load_storage_config().await?;
        let mut candidates: Vec<Box<dyn FileStorage>> = Vec::with_capacity(2);

        match config.storage_type {
            StorageBackend::S3 => {
                if !config.s3.bucket.is_empty() && !config.s3.access_key_id.is_empty() {
                    candidates.push(Box::new(Self::s3_from_config(
                        config.s3.clone(),
                        config.s3.bucket.clone(),
                    )));
                }
                candidates.push(Box::new(config.local));
            }
            StorageBackend::Local => {
                candidates.push(Box::new(config.local.clone()));
                if !config.s3.bucket.is_empty() && !config.s3.access_key_id.is_empty() {
                    candidates.push(Box::new(Self::s3_from_config(
                        config.s3.clone(),
                        config.s3.bucket.clone(),
                    )));
                }
            }
        }

        Ok(candidates)
    }

    /// Stores bytes and records where they went.
    pub async fn upload_bytes(
        &self,
        file_name: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<UploadedFile, APIError> {
        let file_name = sanitize_file_name(file_name);
        let storage = self.resolve_storage().await?;
        let backend = storage.backend();
        let container = storage.container().into_owned();

        let key = format!("uploads/{}/{}", Uuid::new_v4(), file_name);
        let size_bytes = bytes.len() as i64;
        let checksum = sha256_hex(&bytes);

        // Bytes first: a row must never point at an object that isn't there.
        storage
            .put_object(&key, bytes, content_type)
            .await
            .map_err(|e| APIError::Internal {
                message: format!("put_object failed: {e}"),
                details: serde_json::Value::Null,
            })?;

        // Best-effort metadata. Failing the request here would report an error
        // for an upload that actually succeeded, and a retry would orphan the
        // object we just stored. Instead log it: the object stays reachable and
        // `locate_and_heal` adopts it on first read.
        let record = match self
            .files_repo
            .create(&NewFile {
                key: key.clone(),
                backend,
                container,
                original_name: file_name,
                content_type: content_type.to_string(),
                size_bytes,
                checksum: Some(checksum),
            })
            .await
        {
            Ok(row) => Some(row),
            Err(e) => {
                warn!(key = %key, error = %e, "stored object but failed to record its metadata");
                None
            }
        };

        Ok(UploadedFile {
            key,
            backend,
            record,
        })
    }

    /// Fetches a file's bytes, routing on its recorded location.
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, APIError> {
        let mut recorded_error: Option<StorageError> = None;

        if let Some(row) = self.files_repo.get_by_key(key).await? {
            match self.storage_for(&row).await {
                Ok(storage) => match storage.get_object(key).await {
                    Ok(bytes) => return Ok(bytes),
                    Err(e) => {
                        // Not just a missing object: a half-finished migration or
                        // an edited-away config shows up as a transport error
                        // too. Neither should cost the caller their file while
                        // another backend might still hold it.
                        warn!(
                            key = %key,
                            backend = %row.backend,
                            container = %row.container,
                            error = %e,
                            "recorded location did not yield the file, searching other backends"
                        );
                        recorded_error = Some(e);
                    }
                },
                Err(e) => {
                    warn!(
                        key = %key,
                        backend = %row.backend,
                        error = ?e,
                        "recorded location is unusable, searching other backends"
                    );
                    recorded_error = Some(StorageError::Config(format!("{e:?}")));
                }
            }
        }

        match self
            .locate_and_heal(key, self.files_repo.get_by_key(key).await?)
            .await
        {
            Ok(bytes) => Ok(bytes),
            // Surface why the recorded location failed rather than a bare 404.
            Err(_) => match recorded_error {
                Some(e) => Err(map_storage_error("get_object", e)),
                None => Err(APIError::NotFound {
                    resource: format!("File {key}"),
                }),
            },
        }
    }

    /// Finds a file whose recorded location does not hold it — either it
    /// predates the table, or it was moved without the row being updated — and
    /// repairs the row on a hit.
    ///
    /// This is what lets existing local files stay readable after a switch to
    /// S3 without a blocking backfill: each file is repaired the first time
    /// somebody asks for it, and never probed twice.
    async fn locate_and_heal(
        &self,
        key: &str,
        existing: Option<FileRecord>,
    ) -> Result<Vec<u8>, APIError> {
        for storage in self.candidate_backends().await? {
            let backend = storage.backend();
            let container = storage.container().into_owned();

            match storage.get_object(key).await {
                Ok(bytes) => {
                    self.repair_row(key, existing.as_ref(), backend, &container, &bytes)
                        .await;

                    return Ok(bytes);
                }
                Err(StorageError::NotFound(_)) => continue,
                // One unreachable backend must not hide a hit in another.
                Err(e) => warn!(
                    key = %key,
                    backend = %backend,
                    error = %e,
                    "skipping backend during file lookup"
                ),
            }
        }

        Err(APIError::NotFound {
            resource: format!("File {key}"),
        })
    }

    /// Points the `_files` row at where the bytes actually turned up. A
    /// missing row is created; a stale one is repointed, so the next read skips
    /// the probe entirely.
    async fn repair_row(
        &self,
        key: &str,
        existing: Option<&FileRecord>,
        backend: FileBackend,
        container: &str,
        bytes: &[u8],
    ) {
        let content_type = mime_guess::from_path(key)
            .first_or_octet_stream()
            .to_string();

        match existing {
            Some(row) if row.backend == backend && row.container == container => {}
            Some(_) => {
                match self
                    .files_repo
                    .update_location_by_key(key, backend, container)
                    .await
                {
                    Ok(_) => debug!(key = %key, backend = %backend, "repointed stale file row"),
                    Err(e) => warn!(
                        key = %key,
                        error = %e,
                        "found file at a new location but failed to repoint its row"
                    ),
                }
            }
            None => {
                let original_name = object_file_name(key).into_owned();
                match self
                    .files_repo
                    .create(&NewFile {
                        key: key.to_string(),
                        backend,
                        container: container.to_string(),
                        original_name,
                        content_type,
                        size_bytes: bytes.len() as i64,
                        checksum: Some(sha256_hex(bytes)),
                    })
                    .await
                {
                    Ok(_) => debug!(key = %key, backend = %backend, "adopted untracked file"),
                    Err(e) => warn!(
                        key = %key,
                        error = %e,
                        "found untracked file but failed to record it"
                    ),
                }
            }
        }
    }

    /// Removes a file. The row goes first so a failure mid-way leaves an
    /// orphaned object (recoverable, and adopted by `locate_and_heal`) instead
    /// of a row pointing at bytes that are gone.
    pub async fn delete(&self, key: &str) -> Result<(), APIError> {
        if let Some(row) = self.files_repo.delete_by_key(key).await? {
            match self.storage_for(&row).await?.delete_object(key).await {
                Ok(()) => return Ok(()),
                // Already gone is a success as far as the caller is concerned.
                Err(StorageError::NotFound(_)) => return Ok(()),
                Err(e) => {
                    return Err(APIError::Internal {
                        message: format!("delete_object failed: {e}"),
                        details: serde_json::Value::Null,
                    });
                }
            }
        }

        // Untracked (pre-table) file: still clear the bytes.
        self.resolve_storage()
            .await?
            .delete_object(key)
            .await
            .map_err(|e| APIError::Internal {
                message: format!("delete_object failed: {e}"),
                details: serde_json::Value::Null,
            })
    }

    pub async fn presigned_put_url(
        &self,
        key: &str,
        expires_in: Duration,
        content_type: &str,
    ) -> Result<String, APIError> {
        let storage = self.resolve_storage().await?;
        storage
            .presign_put(key, expires_in, content_type)
            .await
            .map_err(|e| map_storage_error("presigned_put_url", e))
    }
}

/// Builds the key a new upload will be stored under, for callers that need to
/// reserve one up front (e.g. presigned uploads).
pub fn build_object_key(file_name: &str) -> String {
    format!(
        "uploads/{}/{}",
        Uuid::new_v4(),
        sanitize_file_name(file_name)
    )
}

/// Unused today, kept so callers can render a stored file name without
/// duplicating the basename rule.
pub fn object_file_name(key: &str) -> Cow<'_, str> {
    key.rsplit('/').next().unwrap_or(key).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::time::Duration;

    // ── Test infrastructure ───────────────────────────────────────────────────

    struct Harness {
        service: FileUploadService,
        root: PathBuf,
        _pool: sqlx::Pool<sqlx::Postgres>,
    }

    impl Harness {
        fn root(&self) -> &PathBuf {
            &self.root
        }

        /// A harness whose active storage is `local` rooted at a temp dir.
        async fn setup(root: PathBuf) -> Harness {
            Self::setup_with(root, StorageBackend::Local).await
        }

        async fn setup_with(root: PathBuf, storage_type: StorageBackend) -> Harness {
            let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/crabbase".to_string()
            });

            let schema = format!("files_svc_{}", Uuid::new_v4().simple());
            let schema_ident = format!("\"{}\"", schema);

            let init_pool = PgPoolOptions::new()
                .max_connections(1)
                .connect(&db_url)
                .await
                .unwrap();

            sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema_ident} CASCADE"))
                .execute(&init_pool)
                .await
                .unwrap();
            sqlx::query(&format!("CREATE SCHEMA {schema_ident}"))
                .execute(&init_pool)
                .await
                .unwrap();
            init_pool.close().await;

            let mut options: sqlx::postgres::PgConnectOptions = db_url.parse().unwrap();
            options = options.options([("search_path", schema.as_str())]);

            let pool = PgPoolOptions::new()
                .max_connections(2)
                .connect_with(options)
                .await
                .unwrap();

            sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

            let settings_repo = SettingsRepository::new(pool.clone());
            settings_repo
                .set(
                    "storage",
                    &StorageConfig {
                        storage_type,
                        local: LocalStorage::new(&root),
                        s3: s3_config(),
                    },
                )
                .await
                .unwrap();

            let service = FileUploadService::new(settings_repo, FilesRepository::new(pool.clone()));

            Harness {
                service,
                root,
                _pool: pool,
            }
        }
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn s3_config() -> S3Config {
        S3Config {
            bucket: "crabbase-uploads".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            // Deliberately unroutable: any test that accidentally reaches the
            // default backend instead of the recorded one fails fast.
            endpoint_url: Some("http://127.0.0.1:1".to_string()),
            force_path_style: true,
        }
    }

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("crabbase-svc-{tag}-{}", Uuid::new_v4()))
    }

    // ── Filename sanitising ───────────────────────────────────────────────────

    #[test]
    fn test_sanitize_strips_path_components() {
        assert_eq!(sanitize_file_name("report.pdf"), "report.pdf");
        assert_eq!(sanitize_file_name("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_file_name("..\\..\\windows\\system32"), "system32");
        assert_eq!(sanitize_file_name("/absolute/report.pdf"), "report.pdf");
        assert_eq!(sanitize_file_name("dir/sub/report.pdf"), "report.pdf");
    }

    #[test]
    fn test_sanitize_replaces_unusable_names() {
        assert_eq!(sanitize_file_name(""), "unnamed");
        assert_eq!(sanitize_file_name("   "), "unnamed");
        assert_eq!(sanitize_file_name(".."), "unnamed");
        assert_eq!(sanitize_file_name("."), "unnamed");
        assert_eq!(sanitize_file_name("re\u{0}port\n.pdf"), "report.pdf");
    }

    #[test]
    fn test_sanitize_caps_length() {
        let long = "a".repeat(400);
        assert_eq!(sanitize_file_name(&long).len(), 255);
    }

    #[test]
    fn test_build_object_key_is_always_relative() {
        let key = build_object_key("../../evil.sh");
        assert!(key.starts_with("uploads/"));
        assert!(!key.contains(".."));
        assert!(key.ends_with("/evil.sh"));
    }

    // ── Upload / download ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_upload_then_download_roundtrip() {
        let h = Harness::setup(temp_root("roundtrip")).await;

        let uploaded = h
            .service
            .upload_bytes("hello.txt", "text/plain", b"hello crabbase".to_vec())
            .await
            .unwrap();

        assert!(uploaded.key.starts_with("uploads/"));
        assert!(uploaded.key.ends_with("/hello.txt"));
        assert_eq!(uploaded.backend, FileBackend::Local);
        assert!(uploaded.record.is_some());

        let bytes = h.service.download(&uploaded.key).await.unwrap();
        assert_eq!(bytes, b"hello crabbase");
    }

    #[tokio::test]
    async fn test_upload_records_location_size_and_checksum() {
        let root = temp_root("metadata");
        let h = Harness::setup(root.clone()).await;

        let bytes = b"checksum me".to_vec();
        let uploaded = h
            .service
            .upload_bytes("data.bin", "application/octet-stream", bytes.clone())
            .await
            .unwrap();

        let row = uploaded.record.expect("metadata row");
        assert_eq!(row.key, uploaded.key);
        assert_eq!(row.backend, FileBackend::Local);
        assert_eq!(row.container, root.to_string_lossy());
        assert_eq!(row.original_name, "data.bin");
        assert_eq!(row.content_type, "application/octet-stream");
        assert_eq!(row.size_bytes, bytes.len() as i64);
        assert_eq!(row.checksum.as_deref(), Some(sha256_hex(&bytes).as_str()));
    }

    #[tokio::test]
    async fn test_upload_writes_bytes_inside_the_upload_root() {
        let root = temp_root("traversal");
        let h = Harness::setup(root.clone()).await;

        // A traversing filename must not land outside the configured root.
        let uploaded = h
            .service
            .upload_bytes("../../../../tmp/pwned.txt", "text/plain", b"x".to_vec())
            .await
            .unwrap();

        assert!(!uploaded.key.contains(".."));
        let written = root.join(&uploaded.key);
        assert!(written.exists(), "expected bytes at {written:?}");
        assert!(!PathBuf::from("/tmp/crabbase-svc-traversal-../../../../tmp/pwned.txt").exists());
    }

    #[tokio::test]
    async fn test_download_missing_file_returns_not_found() {
        let h = Harness::setup(temp_root("missing")).await;

        let err = h
            .service
            .download("uploads/does-not-exist/nope.png")
            .await
            .unwrap_err();

        assert!(matches!(err, APIError::NotFound { .. }));
    }

    // ── The scenario this whole design exists for ─────────────────────────────

    #[tokio::test]
    async fn test_file_stays_readable_after_storage_config_changes() {
        let root = temp_root("switch");
        let h = Harness::setup(root.clone()).await;

        // Uploaded while local was the active backend.
        let local_file = h
            .service
            .upload_bytes("before.pdf", "application/pdf", b"old bytes".to_vec())
            .await
            .unwrap();
        assert_eq!(local_file.backend, FileBackend::Local);

        // Admin switches the active backend to S3. The default backend is now
        // unroutable, so a lookup that consulted the config instead of the row
        // would fail here.
        h.service
            .settings_repo
            .set(
                "storage",
                &StorageConfig {
                    storage_type: StorageBackend::S3,
                    local: LocalStorage::new(&root),
                    s3: s3_config(),
                },
            )
            .await
            .unwrap();

        // The old file still resolves, via its recorded location.
        let bytes = h.service.download(&local_file.key).await.unwrap();
        assert_eq!(bytes, b"old bytes");

        // And the row was not silently rewritten to the new backend.
        let row = h
            .service
            .files_repo
            .get_by_key(&local_file.key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.backend, FileBackend::Local);
        assert_eq!(row.container, root.to_string_lossy());
    }

    #[tokio::test]
    async fn test_new_uploads_follow_the_current_config() {
        let root = temp_root("follow");
        let h = Harness::setup(root.clone()).await;

        // Pretend a file was recorded on S3, then flip the config to local.
        h.service
            .files_repo
            .create(&NewFile {
                key: "uploads/legacy/report.pdf".to_string(),
                backend: FileBackend::S3,
                container: "crabbase-uploads".to_string(),
                original_name: "report.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                size_bytes: 3,
                checksum: None,
            })
            .await
            .unwrap();

        h.service
            .settings_repo
            .set(
                "storage",
                &StorageConfig {
                    storage_type: StorageBackend::Local,
                    local: LocalStorage::new(&root),
                    s3: s3_config(),
                },
            )
            .await
            .unwrap();

        let fresh = h
            .service
            .upload_bytes("new.txt", "text/plain", b"new".to_vec())
            .await
            .unwrap();

        assert_eq!(fresh.backend, FileBackend::Local);
        assert!(root.join(&fresh.key).exists());
    }

    // ── Read repair ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_download_adopts_file_that_has_no_row() {
        let root = temp_root("heal");
        let h = Harness::setup(root.clone()).await;

        // A file written before the _files table existed: bytes on disk, no
        // metadata row.
        let key = "uploads/pre-table/legacy.png";
        let local = LocalStorage::new(&root);
        local
            .put_object(key, b"legacy bytes".to_vec(), "image/png")
            .await
            .unwrap();

        assert!(
            h.service
                .files_repo
                .get_by_key(key)
                .await
                .unwrap()
                .is_none()
        );

        let bytes = h.service.download(key).await.unwrap();
        assert_eq!(bytes, b"legacy bytes");

        // Healed: the row now records where the bytes actually are.
        let row = h
            .service
            .files_repo
            .get_by_key(key)
            .await
            .unwrap()
            .expect("row should be backfilled");
        assert_eq!(row.backend, FileBackend::Local);
        assert_eq!(row.container, root.to_string_lossy());
        assert_eq!(row.size_bytes, b"legacy bytes".len() as i64);
        assert_eq!(row.content_type, "image/png");
    }

    #[tokio::test]
    async fn test_download_recovers_from_stale_backend_pointer() {
        let root = temp_root("stale");
        let h = Harness::setup(root.clone()).await;

        // Row says S3, but the bytes are actually on local disk — the shape you
        // get from a half-finished migration.
        h.service
            .files_repo
            .create(&NewFile {
                key: "uploads/stale/moved.pdf".to_string(),
                backend: FileBackend::S3,
                container: "crabbase-uploads".to_string(),
                original_name: "moved.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                size_bytes: 5,
                checksum: None,
            })
            .await
            .unwrap();

        LocalStorage::new(&root)
            .put_object(
                "uploads/stale/moved.pdf",
                b"moved".to_vec(),
                "application/pdf",
            )
            .await
            .unwrap();

        let bytes = h.service.download("uploads/stale/moved.pdf").await.unwrap();
        assert_eq!(bytes, b"moved");

        // Repointed, not merely served: the next read must not probe S3 again.
        let row = h
            .service
            .files_repo
            .get_by_key("uploads/stale/moved.pdf")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.backend, FileBackend::Local);
        assert_eq!(row.container, root.to_string_lossy());

        assert_eq!(
            h.service.download("uploads/stale/moved.pdf").await.unwrap(),
            b"moved"
        );
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_clears_row_and_bytes() {
        let root = temp_root("delete");
        let h = Harness::setup(root.clone()).await;

        let uploaded = h
            .service
            .upload_bytes("gone.txt", "text/plain", b"bye".to_vec())
            .await
            .unwrap();
        let path = root.join(&uploaded.key);
        assert!(path.exists());

        h.service.delete(&uploaded.key).await.unwrap();

        assert!(!path.exists(), "bytes should be freed");
        assert!(
            h.service
                .files_repo
                .get_by_key(&uploaded.key)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_delete_works_for_untracked_file() {
        let root = temp_root("delete_untracked");
        let h = Harness::setup(root.clone()).await;

        let key = "uploads/pre-table/old.png";
        LocalStorage::new(&root)
            .put_object(key, b"old".to_vec(), "image/png")
            .await
            .unwrap();

        h.service.delete(key).await.unwrap();
        assert!(!root.join(key).exists());
    }

    #[tokio::test]
    async fn test_delete_tolerates_already_missing_bytes() {
        let h = Harness::setup(temp_root("delete_missing")).await;

        let uploaded = h
            .service
            .upload_bytes("vanish.txt", "text/plain", b"x".to_vec())
            .await
            .unwrap();

        // Yank the bytes out from under the row.
        std::fs::remove_file(h.root().join(&uploaded.key)).unwrap();

        h.service.delete(&uploaded.key).await.unwrap();
        assert!(
            h.service
                .files_repo
                .get_by_key(&uploaded.key)
                .await
                .unwrap()
                .is_none()
        );
    }

    // ── Presign ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_presign_reports_unsupported_backend_clearly() {
        let h = Harness::setup(temp_root("presign")).await;

        let err = h
            .service
            .presigned_put_url(
                "uploads/x/direct.bin",
                Duration::from_secs(900),
                "application/octet-stream",
            )
            .await
            .unwrap_err();

        match err {
            APIError::Internal { message, .. } => {
                assert!(message.contains("Presign error"), "got: {message}");
            }
            e => panic!("unexpected error: {e:?}"),
        }
    }

    // ── Storage identity ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_storage_accessors_expose_recorded_identity() {
        let h = Harness::setup(temp_root("identity")).await;

        let storage = h.service.resolve_storage().await.unwrap();
        assert_eq!(storage.backend(), FileBackend::Local);
        assert_eq!(storage.container().into_owned(), h.root().to_string_lossy());
    }
}
