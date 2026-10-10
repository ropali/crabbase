use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use crabbase_core::errors::RepositoryError;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::query_builder::QueryBuilder;
use sqlx::{FromRow, Pool, Postgres};
use uuid::Uuid;

/// Engine holding an object's bytes.
///
/// Persisted as the same lowercase strings the `storage` setting uses
/// (`'local'` / `'s3'`) so config and metadata stay comparable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FileBackend {
    #[default]
    Local,
    S3,
}

impl FileBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }
}

impl fmt::Display for FileBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FileBackend {
    type Err = RepositoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "s3" => Ok(Self::S3),
            other => Err(RepositoryError::Validation {
                message: format!("Unknown file backend '{other}'"),
                field: Some("backend".to_string()),
            }),
        }
    }
}

/// A file's metadata row: the pointer that says which backend currently holds
/// the bytes for `key`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub id: String,
    pub key: String,
    pub backend: FileBackend,
    /// Local base path, or S3 bucket name.
    pub container: String,
    pub original_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl<'r> FromRow<'r, PgRow> for FileRecord {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        // Unreachable while the _files_backend_check CHECK constraint holds.
        let backend_raw: String = row.try_get("backend")?;
        let backend: FileBackend = backend_raw.parse().map_err(|_: RepositoryError| {
            sqlx::Error::Protocol(format!("unknown file backend {backend_raw:?}"))
        })?;

        Ok(Self {
            id: row.try_get::<Uuid, _>("id")?.to_string(),
            key: row.try_get("key")?,
            backend,
            container: row.try_get("container")?,
            original_name: row.try_get("original_name")?,
            content_type: row.try_get("content_type")?,
            size_bytes: row.try_get("size_bytes")?,
            checksum: row.try_get("checksum")?,
            created: row.try_get::<DateTime<Utc>, _>("created")?,
            updated: row.try_get::<DateTime<Utc>, _>("updated")?,
        })
    }
}

/// Payload for registering a freshly stored object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewFile {
    pub key: String,
    pub backend: FileBackend,
    pub container: String,
    pub original_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum: Option<String>,
}

/// Mutable metadata. Location changes go through
/// [`FilesRepository::update_location`] instead.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdate {
    pub original_name: Option<String>,
    pub content_type: Option<String>,
}

/// Sort column for [`FilesRepository::list`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileSortKey {
    #[default]
    Created,
    SizeBytes,
    Key,
}

impl FileSortKey {
    fn as_column(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::SizeBytes => "size_bytes",
            Self::Key => "key",
        }
    }
}

/// Filter + pagination for [`FilesRepository::list`].
#[derive(Debug, Clone, Default)]
pub struct FileFilter {
    pub backend: Option<FileBackend>,
    pub container: Option<String>,
    /// Matches `key LIKE '<prefix>%'` — useful for resolving a whole
    /// `uploads/<uuid>/` subtree at once.
    pub key_prefix: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub order_by: FileSortKey,
    pub ascending: bool,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

const MAX_KEY_LEN: usize = 1024;

/// Rejects keys that are empty, absolute, or able to escape the container when
/// joined onto a local base path. Applied on the write path so a hostile key
/// can never reach the table.
fn validate_key(key: &str) -> Result<(), RepositoryError> {
    let invalid = |message: &str| RepositoryError::Validation {
        message: message.to_string(),
        field: Some("key".to_string()),
    };

    if key.is_empty() {
        return Err(invalid("File key must not be empty"));
    }
    if key.len() > MAX_KEY_LEN {
        return Err(invalid(&format!(
            "File key must be at most {MAX_KEY_LEN} bytes"
        )));
    }
    if key.starts_with('/') {
        return Err(invalid("File key must be relative, not absolute"));
    }
    if key.contains('\0') || key.contains('\\') {
        return Err(invalid("File key must not contain NUL or backslash"));
    }
    if key.split('/').any(|segment| segment == "..") {
        return Err(invalid("File key must not contain '..' path segments"));
    }
    if key.contains("//") {
        return Err(invalid("File key must not contain empty path segments"));
    }

    Ok(())
}

fn parse_id(id: &str) -> Result<Uuid, RepositoryError> {
    Uuid::parse_str(id).map_err(|_| RepositoryError::Validation {
        message: format!("'{id}' is not a valid file id"),
        field: Some("id".to_string()),
    })
}

fn validate_container(container: &str) -> Result<(), RepositoryError> {
    if container.trim().is_empty() {
        return Err(RepositoryError::Validation {
            message: "File container must not be empty".to_string(),
            field: Some("container".to_string()),
        });
    }
    Ok(())
}

pub struct FilesRepository {
    db: Pool<Postgres>,
}

impl FilesRepository {
    pub fn new(db: Pool<Postgres>) -> Self {
        Self { db }
    }

    /// Registers an object's location. Call this *after* the bytes are stored,
    /// so a row never points at an object that was never written.
    pub async fn create(&self, new_file: &NewFile) -> Result<FileRecord, RepositoryError> {
        validate_key(&new_file.key)?;
        validate_container(&new_file.container)?;

        if new_file.size_bytes < 0 {
            return Err(RepositoryError::Validation {
                message: "File size must not be negative".to_string(),
                field: Some("size_bytes".to_string()),
            });
        }

        let row = sqlx::query_as::<_, FileRecord>(
            r#"
            INSERT INTO _files (key, backend, container, original_name, content_type, size_bytes, checksum)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&new_file.key)
        .bind(new_file.backend.as_str())
        .bind(&new_file.container)
        .bind(&new_file.original_name)
        .bind(&new_file.content_type)
        .bind(new_file.size_bytes)
        .bind(&new_file.checksum)
        .fetch_one(&self.db)
        .await?;

        Ok(row)
    }

    /// Hot path: resolves the backend that holds a key. This is the lookup that
    /// stays correct across a storage config change.
    pub async fn get_by_key(&self, key: &str) -> Result<Option<FileRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, FileRecord>("SELECT * FROM _files WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.db)
            .await?;

        Ok(row)
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<FileRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, FileRecord>("SELECT * FROM _files WHERE id = $1")
            .bind(parse_id(id)?)
            .fetch_optional(&self.db)
            .await?;

        Ok(row)
    }

    pub async fn list(&self, filter: &FileFilter) -> Result<Vec<FileRecord>, RepositoryError> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM _files WHERE TRUE");

        if let Some(backend) = filter.backend {
            qb.push(" AND backend = ").push_bind(backend.as_str());
        }
        if let Some(container) = &filter.container {
            qb.push(" AND container = ").push_bind(container);
        }
        if let Some(prefix) = &filter.key_prefix {
            // Escape LIKE wildcards so a prefix is matched literally.
            let escaped = prefix
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            qb.push(" AND key LIKE ").push_bind(format!("{escaped}%"));
            qb.push(" ESCAPE '\\'");
        }
        if let Some(after) = filter.created_after {
            qb.push(" AND created > ").push_bind(after);
        }
        if let Some(before) = filter.created_before {
            qb.push(" AND created < ").push_bind(before);
        }

        qb.push(" ORDER BY ")
            .push(filter.order_by.as_column())
            .push(if filter.ascending { " ASC" } else { " DESC" });

        // Stable tiebreaker so paginating an equal-value column cannot skip or
        // repeat rows.
        qb.push(", id ASC");

        if let Some(limit) = filter.limit {
            qb.push(" LIMIT ").push_bind(limit);
        }
        if let Some(offset) = filter.offset {
            qb.push(" OFFSET ").push_bind(offset);
        }

        let rows = qb
            .build_query_as::<FileRecord>()
            .fetch_all(&self.db)
            .await?;

        Ok(rows)
    }

    pub async fn count(&self) -> Result<i64, RepositoryError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _files")
            .fetch_one(&self.db)
            .await?;

        Ok(count)
    }

    /// Remaining rows for one backend, used to size a migration job.
    pub async fn count_by_backend(&self, backend: FileBackend) -> Result<i64, RepositoryError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _files WHERE backend = $1")
            .bind(backend.as_str())
            .fetch_one(&self.db)
            .await?;

        Ok(count)
    }

    /// Edits display metadata. The location columns are deliberately not
    /// reachable from here.
    pub async fn update(
        &self,
        id: &str,
        update: &FileUpdate,
    ) -> Result<Option<FileRecord>, RepositoryError> {
        let id = parse_id(id)?;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE _files SET updated = now()");

        if let Some(original_name) = &update.original_name {
            qb.push(", original_name = ").push_bind(original_name);
        }
        if let Some(content_type) = &update.content_type {
            qb.push(", content_type = ").push_bind(content_type);
        }

        qb.push(" WHERE id = ").push_bind(id);
        qb.push(" RETURNING *");

        let row = qb
            .build_query_as::<FileRecord>()
            .fetch_optional(&self.db)
            .await?;

        Ok(row)
    }

    /// Repoints a file at a different backend/container. Call this only after
    /// the copy is verified, and keep the source object until then.
    pub async fn update_location(
        &self,
        id: &str,
        backend: FileBackend,
        container: &str,
    ) -> Result<Option<FileRecord>, RepositoryError> {
        validate_container(container)?;

        let row = sqlx::query_as::<_, FileRecord>(
            r#"
            UPDATE _files
            SET backend = $1, container = $2, updated = now()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(backend.as_str())
        .bind(container)
        .bind(parse_id(id)?)
        .fetch_optional(&self.db)
        .await?;

        Ok(row)
    }

    /// Key-addressed variant of [`Self::update_location`] for callers that only
    /// hold the key (e.g. resolving a record's file field).
    pub async fn update_location_by_key(
        &self,
        key: &str,
        backend: FileBackend,
        container: &str,
    ) -> Result<Option<FileRecord>, RepositoryError> {
        validate_container(container)?;

        let row = sqlx::query_as::<_, FileRecord>(
            r#"
            UPDATE _files
            SET backend = $1, container = $2, updated = now()
            WHERE key = $3
            RETURNING *
            "#,
        )
        .bind(backend.as_str())
        .bind(container)
        .bind(key)
        .fetch_optional(&self.db)
        .await?;

        Ok(row)
    }

    /// Drops the metadata row and returns what it pointed at, so the caller can
    /// delete the object afterwards. Order matters: lose the row first and the
    /// bytes become unreachable orphans.
    pub async fn delete(&self, id: &str) -> Result<Option<FileRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, FileRecord>("DELETE FROM _files WHERE id = $1 RETURNING *")
            .bind(parse_id(id)?)
            .fetch_optional(&self.db)
            .await?;

        Ok(row)
    }

    pub async fn delete_by_key(&self, key: &str) -> Result<Option<FileRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, FileRecord>("DELETE FROM _files WHERE key = $1 RETURNING *")
            .bind(key)
            .fetch_optional(&self.db)
            .await?;

        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    // ── Test infrastructure ───────────────────────────────────────────────────

    async fn setup_pool(schema: &str) -> Pool<Postgres> {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/crabbase".to_string());

        let init_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .unwrap();

        let schema_ident = format!("\"{}\"", schema);
        let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE;", schema_ident))
            .execute(&init_pool)
            .await;

        sqlx::query(&format!("CREATE SCHEMA {};", schema_ident))
            .execute(&init_pool)
            .await
            .unwrap();

        init_pool.close().await;

        let mut options: sqlx::postgres::PgConnectOptions = db_url.parse().unwrap();
        options = options.options([("search_path", schema)]);

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        pool
    }

    fn make_repo(pool: Pool<Postgres>) -> FilesRepository {
        FilesRepository::new(pool)
    }

    fn sample_file(key: &str) -> NewFile {
        NewFile {
            key: key.to_string(),
            backend: FileBackend::Local,
            container: "$HOME/crabbase/files/".to_string(),
            original_name: "report.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 2048,
            checksum: Some("abc123".to_string()),
        }
    }

    // ── Backend enum ──────────────────────────────────────────────────────────

    #[test]
    fn test_backend_parses_storage_config_strings() {
        assert_eq!("local".parse::<FileBackend>().unwrap(), FileBackend::Local);
        assert_eq!("s3".parse::<FileBackend>().unwrap(), FileBackend::S3);
        assert_eq!("S3".parse::<FileBackend>().unwrap(), FileBackend::S3);
        assert_eq!(
            " local ".parse::<FileBackend>().unwrap(),
            FileBackend::Local
        );
        assert_eq!(FileBackend::default(), FileBackend::Local);
    }

    #[test]
    fn test_backend_rejects_unknown_value() {
        let err = "gcs".parse::<FileBackend>().unwrap_err();
        match err {
            RepositoryError::Validation { message, field } => {
                assert!(message.contains("gcs"));
                assert_eq!(field.as_deref(), Some("backend"));
            }
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_backend_serialises_as_lowercase() {
        assert_eq!(
            serde_json::to_string(&FileBackend::Local).unwrap(),
            "\"local\""
        );
        assert_eq!(serde_json::to_string(&FileBackend::S3).unwrap(), "\"s3\"");
    }

    // ── Create / read ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_and_get_by_key_roundtrip() {
        let repo = make_repo(setup_pool("files_create_roundtrip").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        assert_eq!(created.key, "uploads/abc/report.pdf");
        assert_eq!(created.backend, FileBackend::Local);
        assert_eq!(created.container, "$HOME/crabbase/files/");
        assert_eq!(created.original_name, "report.pdf");
        assert_eq!(created.content_type, "application/pdf");
        assert_eq!(created.size_bytes, 2048);
        assert_eq!(created.checksum.as_deref(), Some("abc123"));
        assert!(!created.id.is_empty());

        let fetched = repo
            .get_by_key("uploads/abc/report.pdf")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched, created);
    }

    #[tokio::test]
    async fn test_create_accepts_s3_backend() {
        let repo = make_repo(setup_pool("files_create_s3").await);

        let mut new_file = sample_file("uploads/abc/photo.jpg");
        new_file.backend = FileBackend::S3;
        new_file.container = "crabbase-uploads".to_string();

        let created = repo.create(&new_file).await.unwrap();
        assert_eq!(created.backend, FileBackend::S3);
        assert_eq!(created.container, "crabbase-uploads");
    }

    #[tokio::test]
    async fn test_get_by_id_roundtrip() {
        let repo = make_repo(setup_pool("files_get_by_id").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let fetched = repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(fetched, created);
    }

    #[tokio::test]
    async fn test_get_returns_none_for_missing_row() {
        let repo = make_repo(setup_pool("files_get_missing").await);

        assert!(
            repo.get_by_key("uploads/nope/missing.pdf")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            repo.get_by_id("00000000-0000-0000-0000-000000000000")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_get_by_id_rejects_malformed_uuid() {
        let repo = make_repo(setup_pool("files_get_bad_id").await);

        let err = repo.get_by_id("not-a-uuid").await.unwrap_err();
        assert!(matches!(err, RepositoryError::Validation { .. }));
    }

    #[tokio::test]
    async fn test_create_rejects_duplicate_key() {
        let repo = make_repo(setup_pool("files_dup_key").await);

        repo.create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();
        let err = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap_err();

        assert!(matches!(err, RepositoryError::DuplicateKey(_)));
    }

    // ── Key validation ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_rejects_unsafe_keys() {
        let repo = make_repo(setup_pool("files_bad_keys").await);

        // Path traversal, absolute, and empty keys must never reach the table.
        for bad in [
            "../../etc/passwd",
            "uploads/../../../etc/passwd",
            "/etc/passwd",
            "",
            "uploads//report.pdf",
            "uploads\\report.pdf",
        ] {
            let mut new_file = sample_file(bad);
            new_file.key = bad.to_string();
            let err = repo.create(&new_file).await.unwrap_err();
            match err {
                RepositoryError::Validation { field, .. } => {
                    assert_eq!(field.as_deref(), Some("key"), "key {bad:?} should fail");
                }
                e => panic!("unexpected error for {bad:?}: {e:?}"),
            }
        }

        assert_eq!(repo.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_create_rejects_empty_container_and_negative_size() {
        let repo = make_repo(setup_pool("files_bad_fields").await);

        let mut no_container = sample_file("uploads/abc/report.pdf");
        no_container.container = "   ".to_string();
        assert!(matches!(
            repo.create(&no_container).await.unwrap_err(),
            RepositoryError::Validation { .. }
        ));

        let mut negative = sample_file("uploads/abc/other.pdf");
        negative.size_bytes = -1;
        assert!(matches!(
            repo.create(&negative).await.unwrap_err(),
            RepositoryError::Validation { .. }
        ));
    }

    // ── List ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_filters_by_backend_and_container() {
        let repo = make_repo(setup_pool("files_list_filter").await);

        let mut local = sample_file("uploads/a/one.pdf");
        local.container = "$HOME/crabbase/files/".to_string();
        repo.create(&local).await.unwrap();

        let mut s3 = sample_file("uploads/a/two.pdf");
        s3.backend = FileBackend::S3;
        s3.container = "crabbase-uploads".to_string();
        repo.create(&s3).await.unwrap();

        let other_container = FileFilter {
            backend: Some(FileBackend::Local),
            container: Some("/mnt/other".to_string()),
            ..Default::default()
        };
        assert!(repo.list(&other_container).await.unwrap().is_empty());

        let local_only = FileFilter {
            backend: Some(FileBackend::Local),
            ..Default::default()
        };
        let rows = repo.list(&local_only).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "uploads/a/one.pdf");
    }

    #[tokio::test]
    async fn test_list_by_key_prefix() {
        let repo = make_repo(setup_pool("files_list_prefix").await);

        repo.create(&sample_file("uploads/aaa/one.pdf"))
            .await
            .unwrap();
        repo.create(&sample_file("uploads/bbb/two.pdf"))
            .await
            .unwrap();
        repo.create(&sample_file("avatars/me.png")).await.unwrap();

        let rows = repo
            .list(&FileFilter {
                key_prefix: Some("uploads/aaa/".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "uploads/aaa/one.pdf");
    }

    #[tokio::test]
    async fn test_list_escapes_like_wildcards_in_prefix() {
        let repo = make_repo(setup_pool("files_list_prefix_wildcard").await);

        repo.create(&sample_file("uploads/100%/one.pdf"))
            .await
            .unwrap();
        repo.create(&sample_file("uploads/1000/two.pdf"))
            .await
            .unwrap();

        let rows = repo
            .list(&FileFilter {
                key_prefix: Some("uploads/100%/".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "uploads/100%/one.pdf");
    }

    #[tokio::test]
    async fn test_list_paginates_newest_first_by_default() {
        let repo = make_repo(setup_pool("files_list_paginate").await);

        for i in 0..3 {
            repo.create(&sample_file(&format!("uploads/{i}/file.pdf")))
                .await
                .unwrap();
        }

        let newest = repo
            .list(&FileFilter {
                limit: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(newest.len(), 2);

        let next_page = repo
            .list(&FileFilter {
                limit: Some(2),
                offset: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(next_page.len(), 1);

        let seen: Vec<&str> = newest
            .iter()
            .chain(next_page.iter())
            .map(|f| f.key.as_str())
            .collect();
        let unique: std::collections::HashSet<&&str> = seen.iter().collect();
        assert_eq!(seen.len(), 3, "pagination must not repeat rows");
        assert_eq!(unique.len(), 3);
    }

    #[tokio::test]
    async fn test_list_sorts_by_key_ascending() {
        let repo = make_repo(setup_pool("files_list_sort").await);

        for name in ["c.pdf", "a.pdf", "b.pdf"] {
            repo.create(&sample_file(&format!("uploads/1/{name}")))
                .await
                .unwrap();
        }

        let rows = repo
            .list(&FileFilter {
                order_by: FileSortKey::Key,
                ascending: true,
                ..Default::default()
            })
            .await
            .unwrap();

        let keys: Vec<&str> = rows.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["uploads/1/a.pdf", "uploads/1/b.pdf", "uploads/1/c.pdf"]
        );
    }

    // ── Counts ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_counts_group_by_backend() {
        let repo = make_repo(setup_pool("files_counts").await);

        repo.create(&sample_file("uploads/a/one.pdf"))
            .await
            .unwrap();

        let mut s3 = sample_file("uploads/a/two.pdf");
        s3.backend = FileBackend::S3;
        repo.create(&s3).await.unwrap();
        let mut s3b = sample_file("uploads/a/three.pdf");
        s3b.backend = FileBackend::S3;
        repo.create(&s3b).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 3);
        assert_eq!(repo.count_by_backend(FileBackend::Local).await.unwrap(), 1);
        assert_eq!(repo.count_by_backend(FileBackend::S3).await.unwrap(), 2);
    }

    // ── Update ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_update_changes_metadata_only() {
        let repo = make_repo(setup_pool("files_update").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let updated = repo
            .update(
                &created.id,
                &FileUpdate {
                    original_name: Some("renamed.pdf".to_string()),
                    content_type: None,
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.original_name, "renamed.pdf");
        assert_eq!(updated.content_type, "application/pdf");
        // Location must survive a metadata edit
        assert_eq!(updated.backend, created.backend);
        assert_eq!(updated.container, created.container);
        assert_eq!(updated.key, created.key);
        assert!(updated.updated >= created.updated);
    }

    #[tokio::test]
    async fn test_update_returns_none_for_missing_row() {
        let repo = make_repo(setup_pool("files_update_missing").await);

        let result = repo
            .update(
                "00000000-0000-0000-0000-000000000000",
                &FileUpdate {
                    original_name: Some("x.pdf".to_string()),
                    content_type: None,
                },
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_location_flips_pointer() {
        let repo = make_repo(setup_pool("files_update_location").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let moved = repo
            .update_location(&created.id, FileBackend::S3, "crabbase-uploads")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(moved.backend, FileBackend::S3);
        assert_eq!(moved.container, "crabbase-uploads");
        // Identity and metadata are untouched by a pointer flip
        assert_eq!(moved.id, created.id);
        assert_eq!(moved.key, created.key);
        assert_eq!(moved.size_bytes, created.size_bytes);

        let reread = repo
            .get_by_key("uploads/abc/report.pdf")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reread.backend, FileBackend::S3);
        assert_eq!(reread.container, "crabbase-uploads");
    }

    #[tokio::test]
    async fn test_update_location_by_key_works() {
        let repo = make_repo(setup_pool("files_update_location_key").await);

        repo.create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let moved = repo
            .update_location_by_key(
                "uploads/abc/report.pdf",
                FileBackend::S3,
                "crabbase-uploads",
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(moved.backend, FileBackend::S3);
        assert_eq!(moved.container, "crabbase-uploads");
    }

    #[tokio::test]
    async fn test_update_location_rejects_empty_container() {
        let repo = make_repo(setup_pool("files_update_location_bad").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        assert!(matches!(
            repo.update_location(&created.id, FileBackend::S3, "  ")
                .await
                .unwrap_err(),
            RepositoryError::Validation { .. }
        ));
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_returns_row_so_caller_can_free_bytes() {
        let repo = make_repo(setup_pool("files_delete").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let deleted = repo.delete(&created.id).await.unwrap().unwrap();
        assert_eq!(deleted, created);

        assert!(
            repo.get_by_key("uploads/abc/report.pdf")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(repo.count().await.unwrap(), 0);

        // Deleting again is a no-op, not an error
        assert!(repo.delete(&created.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_by_key() {
        let repo = make_repo(setup_pool("files_delete_by_key").await);

        repo.create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let deleted = repo
            .delete_by_key("uploads/abc/report.pdf")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(deleted.key, "uploads/abc/report.pdf");

        assert!(
            repo.delete_by_key("uploads/abc/report.pdf")
                .await
                .unwrap()
                .is_none()
        );
    }

    // ── Serialisation ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_file_record_serialises_as_camel_case() {
        let repo = make_repo(setup_pool("files_camel_case").await);

        let created = repo
            .create(&sample_file("uploads/abc/report.pdf"))
            .await
            .unwrap();

        let json = serde_json::to_value(&created).unwrap();
        assert_eq!(json["originalName"], "report.pdf");
        assert_eq!(json["contentType"], "application/pdf");
        assert_eq!(json["sizeBytes"], 2048);
        assert_eq!(json["backend"], "local");
        assert!(json.get("original_name").is_none());
    }

    // ── The scenario this table exists for ────────────────────────────────────

    #[tokio::test]
    async fn test_stored_file_survives_storage_config_change() {
        let repo = make_repo(setup_pool("files_config_change").await);

        // File uploaded while local was the active backend.
        let local_file = repo
            .create(&NewFile {
                backend: FileBackend::Local,
                container: "$HOME/crabbase/files/".to_string(),
                ..sample_file("uploads/before/report.pdf")
            })
            .await
            .unwrap();

        // Admin switches the active backend to S3; new uploads go there.
        let s3_file = repo
            .create(&NewFile {
                backend: FileBackend::S3,
                container: "crabbase-uploads".to_string(),
                ..sample_file("uploads/after/report.pdf")
            })
            .await
            .unwrap();

        // Both still resolve to the backend that actually holds them, with no
        // per-request branching on the current config.
        let before = repo.get_by_key(&local_file.key).await.unwrap().unwrap();
        assert_eq!(before.backend, FileBackend::Local);
        assert_eq!(before.container, "$HOME/crabbase/files/");

        let after = repo.get_by_key(&s3_file.key).await.unwrap().unwrap();
        assert_eq!(after.backend, FileBackend::S3);
        assert_eq!(after.container, "crabbase-uploads");
    }
}
