# Files & uploads

Crabbase has built-in file storage. A `File` column stores a reference to an
uploaded object; the bytes live on local disk or in S3, and downloads are served
through Crabbase so the collection's `view_rule` is always enforced.

Typical workflow:

1. Define one or more `File` columns on a collection.
2. Upload the bytes with a multipart request.
3. Save the returned file `id` in a record.
4. Render or download the file from its URL, adding a file token when the
   collection is not public.

The admin dashboard supports this end to end: create a `File` column, then use
the upload control in the create/edit record drawer.

---

## Quick start

```sh
# 1. Create a collection with a public cover image column
curl -X POST http://localhost:8989/api/collections \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "articles",
    "collection_type": "base",
    "columns": [
      { "name": "title", "data_type": "PlainText", "required": true },
      { "name": "cover", "data_type": "File", "max_size": 5242880, "mime_types": ["image/*"] }
    ],
    "list_rule": "",
    "view_rule": "",
    "create_rule": ""
  }'

# 2. Upload a file (public because create_rule is "")
curl -X POST http://localhost:8989/api/collections/articles/records/upload \
  -F 'cover=@cover.png;type=image/png'

# Response:
# {"files":[{"id":"e97669ba-...","key":"uploads/.../cover.png", ...}]}

# 3. Attach the returned id to a record
curl -X POST http://localhost:8989/api/collections/articles/records \
  -H 'Content-Type: application/json' \
  -d '{"data":{"title":"Hello","cover":"e97669ba-..."}}'

# 4. Fetch the file (public view_rule)
curl http://localhost:8989/api/files/e97669ba-...
```

If `view_rule` is not empty, step 4 also needs authenticated access: a Bearer
token for server-side requests, or a file token for browser media tags. See
[Protected files](#protected-files).

---

## Defining file columns

Add a column with `"data_type": "File"` when creating or updating a collection:

```json
{
  "name": "articles",
  "collection_type": "base",
  "columns": [
    { "name": "title", "data_type": "PlainText", "required": true },
    {
      "name": "cover",
      "data_type": "File",
      "max_size": 5242880,
      "mime_types": ["image/png", "image/jpeg", "image/webp"]
    },
    {
      "name": "attachments",
      "data_type": "File",
      "min": 1,
      "max": 10
    }
  ]
}
```

`cover` accepts one file. `attachments` accepts an array of 1–10 files because
`max > 1` (or `min > 1`) marks a file column as multi-file.

### File column options

| Option | Description |
|---|---|
| `required` | The record must contain a file reference. |
| `min` / `max` | Number of files for this field, not character length. `max > 1` or `min > 1` makes the column multi-file. |
| `max_size` | Maximum size in **bytes** per file. |
| `mime_types` | Allowed content types. Supports exact values and prefixes such as `image/*`; `*` or `*/*` allows anything. |
| `pattern` | Regular expression matched against the uploaded file name, e.g. `\.(png\|jpe?g)$`. |
| `index`, `hidden` | Normal collection-column options; not file-specific. |

Constraints are applied at **upload time** only when the multipart part name
matches a file column. `min` is checked when the record is created or updated,
because a single upload request cannot know how many files the finished record
will contain.

There is no storage quota. The per-column `max_size` and the request size limit
(100 MB) are the controls.

---

## Uploading files

| Method | Path |
|---|---|
| `POST` | `/api/collections/{name}/records/upload` |
| `POST` | `/api/collection/{name}/record/upload` (legacy alias) |

The request must be `multipart/form-data`. Authentication is governed by the
collection's `create_rule`:

| `create_rule` | Who can upload |
|---|---|
| `null` (not set) | Superusers only |
| `""` (empty string) | Anyone, no authentication required |
| expression | Callers that satisfy the rule; send a Bearer token |

See [API rules](collections-and-records.md#api-rules) for the rule language and
[Authentication](authentication.md) for obtaining a Bearer token.

### Basic upload

```sh
curl -X POST http://localhost:8989/api/collections/articles/records/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F 'cover=@photo.png;type=image/png'
```

```json
{
  "files": [
    {
      "id": "e97669ba-2a3c-4c47-9d0e-1f2a3b4c5d6e",
      "key": "uploads/4f8c.../photo.png",
      "backend": "local",
      "container": "./uploads",
      "originalName": "photo.png",
      "contentType": "image/png",
      "sizeBytes": 10240,
      "checksum": "9f86d081884c7d65...",
      "collectionName": "articles",
      "created": "2026-10-18T15:42:32.888198Z",
      "updated": "2026-10-18T15:42:32.888198Z"
    }
  ]
}
```

Use the returned `id` when linking the file to a record. The `key` also works as
a reference, but `id` is the canonical choice.

### Field names matter

Name each multipart part after a `File` column to apply that column's
constraints:

```sh
# cover's max_size, mime_types, and pattern are enforced
-F 'cover=@photo.png;type=image/png'

# unknown part name: stored, but no column constraints are applied
-F 'file=@photo.png;type=image/png'
```

- **Single-file column** (`max` unset, or `max: 1`): one part with that name.
- **Multi-file column** (`max > 1` or `min > 1`): up to `max` parts; if `max` is
  unset, the number of parts is unlimited.
- **Multiple columns in one request**: the response is a flat `files` array and
  does not identify which part or column each file came from. Upload one column
  per request, or preserve the multipart order yourself.

Uploading does **not** attach files to a record. It only stores the bytes and
returns metadata; saving the record is a separate step.

### Multiple files

```sh
curl -X POST http://localhost:8989/api/collections/articles/records/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F 'attachments=@one.pdf;type=application/pdf' \
  -F 'attachments=@two.pdf;type=application/pdf'
```

The response contains one entry per part, in order.

### Upload limits

- The entire request body is capped at **100 MB**, including all parts.
- A part named after a file column must satisfy that column's `max_size`,
  `mime_types`, and `pattern`; otherwise the request fails with `400`.
- A part with no file name is stored as `unnamed`.
- File names are sanitized to a single path component before storage.

---

## Attaching files to records

Single-file columns take a string:

```sh
curl -X POST http://localhost:8989/api/collections/articles/records \
  -H 'Content-Type: application/json' \
  -d '{"data":{"title":"My Article","cover":"e97669ba-..."}}'
```

Multi-file columns take an array:

```sh
curl -X POST http://localhost:8989/api/collections/articles/records \
  -H 'Content-Type: application/json' \
  -d '{"data":{"title":"Gallery","attachments":["e97669ba-...","f1284abc-..."]}}'
```

When a record is read back, file references are replaced with their metadata
objects (`id`, `key`, `originalName`, `contentType`, `sizeBytes`, ...):

```json
{
  "id": "record-id",
  "data": {
    "title": "My Article",
    "cover": {
      "id": "e97669ba-...",
      "key": "uploads/.../photo.png",
      "originalName": "photo.png",
      "contentType": "image/png",
      "sizeBytes": 10240
    }
  }
}
```

A multi-file field returns an array of such objects.

### Lifecycle and cleanup

- Replacing a file, removing an item from a multi-file array, or clearing the
  field deletes the old bytes and its `_files` metadata row.
- Deleting a record deletes every file referenced by its file fields.
- Uploads that were never attached to a record are **not** cleaned up
  automatically. They remain available in the collection's file list until an
  admin deletes them.
- Truncating a collection deletes records without running per-record file
  cleanup. Delete records individually if you need their files removed.
- Deleting a stored file directly is an admin operation:
  `DELETE /api/files/{id_or_key}`.

---

## Listing and downloading files

### Download URLs

A file can be fetched by `id` or storage `key` from two equivalent places:

| URL | Behavior |
|---|---|
| `GET /api/files/{id_or_key}` | Uses the file's own collection for access checks. |
| `GET /api/collections/{name}/files/{id_or_key}` | Also requires the file to belong to `{name}`; otherwise `404`. |

Downloads default to `Content-Disposition: inline`. Add `?download=1` to force
an attachment (useful for "Save as..." links). Accepted truthy values are `1`,
`true`, `TRUE`, and `yes`.

```sh
# Inline
curl http://localhost:8989/api/files/e97669ba-...

# Force download
curl 'http://localhost:8989/api/files/e97669ba-...?download=1'
```

The response keeps the stored content type. Files are served whole — Range
requests, thumbnails, and on-the-fly transformations are not supported.

### Listing a collection's files

```sh
curl http://localhost:8989/api/collections/articles/files
```

Governed by the collection's `list_rule`. The response includes every file
uploaded for the collection, including files not yet attached to a record:

```json
{
  "items": [
    {
      "id": "e97669ba-...",
      "key": "uploads/.../photo.png",
      "name": "photo.png",
      "size": 10240,
      "contentType": "image/png",
      "collectionName": "articles",
      "created": "2026-10-18T15:42:32Z",
      "updated": "2026-10-18T15:42:32Z",
      "path": "/api/collections/articles/files/e97669ba-..."
    }
  ],
  "totalItems": 1
}
```

`path` is ready to use as a download URL. This endpoint is not paginated.

---

## Protected files

File access is decided by the owning collection's `view_rule`:

| `view_rule` | Download access |
|---|---|
| `""` (empty string) | Public; no token needed. |
| `null` (not set) | Superusers only. |
| expression | Evaluated using the caller's authenticated user. |

Because downloads are checked against the **collection** (there is no per-file
ACL), write rules that only depend on the authenticated user. A rule such as
`@request.auth.id != ""` works; a record-dependent rule such as
`owner = @request.auth.id` cannot be evaluated for a file download and will
deny non-admins.

A normal `Authorization: Bearer` header also works for downloads. File tokens
exist for browser elements that cannot set headers.

### File tokens

Browsers cannot attach an `Authorization` header to `<img>`, `<video>`, or
`<a download>` requests. For protected files, exchange the normal Bearer token
for a short-lived **file token** and pass it in the query string.

```sh
curl -X POST http://localhost:8989/api/files/token \
  -H "Authorization: Bearer $TOKEN"
```

```json
{
  "token": "eyJhbGciOi...",
  "expiresIn": 180
}
```

Then append it to any file URL:

```sh
curl "http://localhost:8989/api/files/e97669ba-...?token=$FILE_TOKEN"

# Combine with download
curl "http://localhost:8989/api/files/e97669ba-...?download=1&token=$FILE_TOKEN"
```

The same token works on both download URL forms and for every file the user is
allowed to view. It is an identity token, not a per-file capability.

- Default lifetime: **180 seconds**.
- Reuse it until it expires; request a new one when a download returns `401`.
- Changing the user's password or token key invalidates outstanding file tokens.
- A file token only authenticates file downloads. It cannot call records,
  settings, or admin endpoints.
- An expired or invalid token returns `401`; a valid token whose user fails the
  collection `view_rule` returns `403`.
- For public files, do not send a stale `Authorization` header — an invalid
  header returns `401` even when the file itself is public.

### JavaScript example

```ts
let cached: { token: string; expiresAt: number } | null = null;

async function getFileToken(authToken: string): Promise<string> {
  if (cached && Date.now() < cached.expiresAt) return cached.token;

  const res = await fetch("/api/files/token", {
    method: "POST",
    headers: { Authorization: `Bearer ${authToken}` },
  });
  if (!res.ok) throw new Error(`file token failed: ${res.status}`);

  const body = await res.json();
  cached = {
    token: body.token,
    // refresh a little early
    expiresAt: Date.now() + (body.expiresIn - 15) * 1000,
  };
  return cached.token;
}

function fileUrl(path: string, token?: string): string {
  if (!token) return path;
  const separator = path.includes("?") ? "&" : "?";
  return `${path}${separator}token=${encodeURIComponent(token)}`;
}

// <img src={fileUrl(`/api/files/${file.id}`, await getFileToken(authToken))} />
```

The admin dashboard performs this exchange automatically for file previews and
download links.

---

## Storage configuration

New uploads go to the storage configured in **Settings → File storage** in the
admin dashboard, or through the admin-only settings API.

| Method | Path |
|---|---|
| `GET` | `/api/settings/storage` |
| `POST` | `/api/settings/storage` |

Both require a superuser Bearer token. `GET` returns `404` until storage
settings have been saved; when unset, Crabbase defaults to local storage under
`./uploads`.

### Local disk

```sh
curl -X POST http://localhost:8989/api/settings/storage \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "storageType": "local",
    "local": { "basePath": "$HOME/crabbase/files/" }
  }'
```

`basePath` is relative to the server's working directory unless absolute.
A leading `$HOME` is expanded at runtime. The directory must be writable by the
Crabbase process. (`local_path` is also accepted as an alias for `basePath`.)

### S3-compatible storage

```sh
curl -X POST http://localhost:8989/api/settings/storage \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "storageType": "s3",
    "local": { "basePath": "./uploads" },
    "s3": {
      "bucket": "my-crabbase-files",
      "region": "us-east-1",
      "accessKey": "AKIA...",
      "secret": "...",
      "endpoint": "",
      "forcePathStyle": false
    }
  }'
```

| Key | Description |
|---|---|
| `storageType` | `"local"` or `"s3"`. Defaults to local. |
| `local.basePath` | Local upload directory. |
| `s3.bucket` | Existing bucket name. The bucket is not created automatically. |
| `s3.region` | AWS region. |
| `s3.accessKey` / `s3.secret` | Credentials with PutObject, GetObject, and DeleteObject permission. |
| `s3.endpoint` | Optional custom endpoint for MinIO, Cloudflare R2, etc. Empty means AWS. |
| `s3.forcePathStyle` | Enable for MinIO and other path-style S3 services. |

Existing files are not migrated when the storage setting changes. Each file's
`_files` row records its backend and container, so old files stay readable as
long as the corresponding local path or S3 credentials are still available.
Uploads made after the change use the new backend.

Credentials are stored in the settings table; protect admin access accordingly.

---

## Limits and caveats

- Upload requests are capped at **100 MB**.
- Per-file size limits come from `max_size` on the column, in bytes.
- There is no per-file permission model; access is collection-wide via
  `view_rule`.
- No image resizing, thumbnails, transcoding, Range requests, or resumable
  uploads.
- File URLs are always proxied through Crabbase; S3 objects are not exposed
  directly.
- Unattached uploads and files left behind by collection truncation are not
  garbage-collected automatically.
- The collection file list is unpaginated and includes unattached files.

---

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `401` on a file URL | Expired or invalid file token; or a stale `Authorization` header on a public request. |
| `403` on upload | `create_rule` is `null` (admin only) or the caller does not satisfy it. |
| `403` on download | No token/Bearer sent for a protected file, `view_rule` is `null`, the caller does not satisfy it, or the rule depends on record fields. |
| `404` on a collection file URL | The file ID/key does not exist or belongs to a different collection. |
| `400: No file was uploaded` | The multipart request contained no parts. |
| `400: field 'x' accepts a single file` | More than one part was sent for a single-file column. |
| `400: unsupported MIME type` | The part's `Content-Type` does not match `mime_types`. |
| `400: exceeds maximum allowed size` | The file is larger than the column's `max_size`. |
| `400: must be an array of file keys or ids` | A multi-file column was given a string, or vice versa. |
| `500` on upload/download | Storage is misconfigured, S3 credentials are missing, or the local directory is not writable. |
| Upload succeeds but the record is unchanged | Uploading only stores bytes; attach the returned `id` with a create/update request. |
| Image shows `403` in a browser | Protected collection; append a file token to the `src` URL. |
