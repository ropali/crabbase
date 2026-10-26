# Collections & records

Collections are the heart of Crabbase. A collection is a real PostgreSQL table that you define at runtime — and the moment it exists, a full REST CRUD surface is generated for it at `/api/collections/{name}/records`. This is how you build a backend without writing any API code.

## The easy way: the admin dashboard

Everything in this document can be done from the admin dashboard (`crabbase admin`, default `http://localhost:9898`) — no curl required:

| Task | Where in the UI |
|---|---|
| Create a collection (name, type, fields, indexes) | **Create Collection** drawer in the sidebar |
| Add/remove/retype fields after creation | Collection's **Edit Schema** drawer |
| Set API rules | Same edit drawer (rule fields per operation) |
| Browse / create / edit / delete records | Collection's data page |
| Delete all records | Data page → truncate |
| Delete a collection | Edit drawer |

The dashboard talks to the exact same REST endpoints documented below — use whichever fits your workflow (clicks for day-to-day work, API for automation/CI).

Collection management requires a **superuser** login (dashboard) or superuser JWT (API). Record access is controlled per operation by API rules: with no rule set, only superusers are allowed. Set list/view to `""` (public) after creating a collection if you want unauthenticated reads.

## Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/collections?page=1&per_page=20` | superuser | List collections |
| `POST` | `/api/collections` | superuser | Create collection (creates the table) |
| `GET` | `/api/collections/{name}` | superuser | Get one collection |
| `PATCH` | `/api/collections/{name}` | superuser | Update schema / rename / set rules |
| `DELETE` | `/api/collections/{name}` | superuser | Delete collection **and drop its table** |
| `POST` | `/api/collections/{name}/truncate` | superuser | Delete all records |
| `GET` | `/api/collections/{name}/records?page=1&per_page=20&filter=&expand=` | rules apply | List records (with pagination, dynamic filtering, & relation expansion) |
| `POST` | `/api/collections/{name}/records` | rules apply* | Create record |
| `GET` | `/api/collections/{name}/records/{id}?expand=` | rules apply* | Get record (with optional relation expansion; `view_rule` enforced) |
| `PATCH` | `/api/collections/{name}/records/{id}` | rules apply* | Update record |
| `DELETE` | `/api/collections/{name}/records/{id}` | rules apply* | Delete record |

\* Record endpoints enforce collection rules: `list_rule` on list, `view_rule` on single-record fetch and relation expansion, `create_rule` on create/upload, and `delete_rule` on delete. `update_rule` rejects non-admins when it is null, but expressions are not applied to rows yet (an empty string allows all non-admins). See [API rules](#api-rules).

## Creating a collection

```sh
curl -X POST http://localhost:8989/api/collections \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "products",
    "collection_type": "base",
    "columns": [
      { "name": "title",   "data_type": "PlainText", "index": true,  "required": true },
      { "name": "body",    "data_type": "RichText" },
      { "name": "price",   "data_type": "Number",    "min": 0 },
      { "name": "active",  "data_type": "Bool" },
      { "name": "email",   "data_type": "Email" },
      { "name": "url",     "data_type": "Url" },
      { "name": "starts_at","data_type": "Datetime" },
      { "name": "published_at", "data_type": "autodate" },
      { "name": "meta",    "data_type": "Json" },
      { "name": "status",  "data_type": "Select" },
      { "name": "category_id", "data_type": "Relation", "related_to": "categories" }
    ],
    "list_rule": "",
    "view_rule": "",
    "create_rule": ""
  }'
```

The `Relation` field requires the `categories` collection to exist first; create it before running this example, or drop that field.

### Field types (`data_type`)

| Type | Postgres type | Notes |
|---|---|---|
| `PlainText` | `TEXT` | Default type |
| `RichText` | `TEXT` | Long-form content; rich editor in admin UI |
| `Number` | `BIGINT` | Integers |
| `Bool` | `BOOLEAN` | |
| `Email` | `TEXT` | |
| `Url` | `TEXT` | |
| `Datetime` | `TIMESTAMPTZ` | Returned as RFC-3339 strings |
| `AutoDatetime` | `TIMESTAMPTZ DEFAULT now()` | Set automatically on insert. Pass as the legacy string `"autodate"` (or `"autodatetime"`) — the plain variant name is not accepted as a string |
| `File` | `TEXT` | Stores the uploaded record ID or storage key from `/api/collections/{name}/records/upload`. See [Files & uploads](files-and-uploads.md) |
| `Relation` | `UUID` + FK | Requires `"related_to": "<other-collection-name>"`; target must exist first |
| `Select` | `TEXT` | |
| `Json` | `TEXT` | JSON viewer in admin UI |
| `GeoPoint` | `TEXT` | Placeholder — not a real geometry type yet |

Legacy aliases are accepted for compatibility: `text`, `richtext`/`editor`, `int`/`integer`, `bool`/`boolean`, `date`, `autodate`, etc. (case-insensitive).

### Column options

| Option | Default | Description |
|---|---|---|
| `index` | `false` | Create a btree index on the column |
| `hidden` | `false` | Mark field hidden |
| `required` | `false` | Marked as required |
| `min` / `max` | — | Size constraints; for `File`, the number of files (see [Files & uploads](files-and-uploads.md#defining-file-columns)) |
| `pattern` | — | Validation pattern; for `File`, matched against the uploaded file name |
| `max_size` / `mime_types` | — | `File` only: per-file byte limit and allowed content types |
| `related_to` | — | Target collection name (for `Relation`) |

### Built-in columns

Every table automatically gets:

- `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
- `created TIMESTAMPTZ`
- `updated TIMESTAMPTZ`

You cannot create columns named `id`, `created`, or `updated`. Collection names must be valid identifiers (`[A-Za-z_][A-Za-z0-9_]*`), and each collection gets its own random JWT secret generated into `options.authToken`.

### Collection types

- `base` — regular data collection
- `auth` — records can log in; see [Authentication](authentication.md)
- `view` — reserved

## Updating schema (live migration)

`PATCH /api/collections/{name}` performs an online schema change: renames the table if you pass `name`, adds/drops/retypes columns per the new `columns` list, rebuilds indexes, and updates API rules.

```sh
curl -X PATCH http://localhost:8989/api/collections/products \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{ "columns": [ { "name": "title", "data_type": "PlainText" }, { "name": "sku", "data_type": "PlainText", "index": true } ] }'
```

> Schema changes are applied immediately against live data. Dropping a column discards its data.

## Working with records

Records are addressed through the auto-generated endpoints. Payloads use a `data` envelope; `id`, `created`, and `updated` are managed by the system and returned on every record.

```sh
# Create
curl -X POST http://localhost:8989/api/collections/products/records \
  -H 'Content-Type: application/json' \
  -d '{"data": {"title": "Widget", "price": 999, "active": true}}'

# Response
{
  "id": "5f0c...",
  "data": {"title": "Widget", "price": 999, "active": true},
  "created": "2026-08-25T10:00:00Z",
  "updated": "2026-08-25T10:00:00Z"
}

# List (pagination)
curl 'http://localhost:8989/api/collections/products/records?page=1&per_page=20'
# -> { "items": [...], "total": 42, "page": 1, "per_page": 20 }

# List with dynamic filter
curl -G 'http://localhost:8989/api/collections/products/records' \
  --data-urlencode "filter=active=true && price < 1500"

# List with expanded relations (no N+1 queries)
curl 'http://localhost:8989/api/collections/books/records?expand=author'

# Get one
curl http://localhost:8989/api/collections/products/records/<id>

# Get one with expanded relations
curl 'http://localhost:8989/api/collections/books/records/<id>?expand=author'

# Update (admin token, unless update_rule allows the caller)
curl -X PATCH http://localhost:8989/api/collections/products/records/<id> \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"data": {"price": 1299}}'

# Delete (admin token, unless delete_rule allows the caller)
curl -X DELETE http://localhost:8989/api/collections/products/records/<id> \
  -H "Authorization: Bearer $TOKEN"
```

Pagination defaults: `page=1`, `per_page=20` (clamped to 1–100).

## File Storage & Attachments

Collections support `File` field types for storing file references (images, documents, archives). The file management workflow is integrated with collections. For the full reference — protected downloads, file tokens, storage backends, and client examples — see [Files & uploads](files-and-uploads.md).

### 1. Uploading Files for a Collection
Upload one or multiple files using multipart form data to `/api/collections/{name}/records/upload` (the only upload endpoint; `POST /api/collections/{name}/files` does not exist):
```sh
# Single file upload
curl -X POST http://localhost:8989/api/collections/articles/records/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F 'file=@photo.png;type=image/png'

# Multiple files at one go
curl -X POST http://localhost:8989/api/collections/articles/records/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F 'files=@photo1.png;type=image/png' \
  -F 'files=@photo2.png;type=image/png'
```

- **Access Rules**: Governed by the collection's `create_rule`. Callers without create permission receive `403 Forbidden`.
- **Target Collection (`{name}`)**: Must match an existing collection in the database (e.g. `articles`, `users`, `posts`). If the collection does not exist, a `404 Not Found` is returned.
- **Collection Linkage**: Uploads record the `collection_name` in the `_files` metadata table, allowing multiple files to be tracked and retrieved per collection.
- **Field-aware uploads**: When a multipart part is named after a `File` column of the collection (e.g. `-F 'cover=@photo.png'`), the column's `max_size` (bytes), `mime_types` (supports wildcards like `image/*` or `*/*`), and `pattern` (regex matched against the original file name, e.g. `\.(png|jpe?g)$`) are enforced. A single-file column (`max` unset or `max: 1`) rejects more than one part for the same field name; a multi-file column (`max > 1` or `min > 1`) accepts up to `max` parts. Parts that do not match a column name are stored without field-level constraints.
- **Request size limit**: multipart bodies are capped at 100 MB per request.
- **Response**: the `files` array mirrors the stored `_files` metadata rows:
  ```json
  {
    "files": [
      {
        "id": "e97669ba-...",
        "key": "uploads/.../photo1.png",
        "backend": "local",
        "container": "$HOME/crabbase/files/",
        "originalName": "photo1.png",
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
  Use the returned `id` (or `key`) when linking files to records.

### 2. Creating or Updating Records with Files
Use the returned file ID or storage key when creating or updating records:
```sh
# Single file
curl -X POST http://localhost:8989/api/collections/articles/records \
  -H 'Content-Type: application/json' \
  -d '{"data": {"title": "My Article", "cover": "e97669ba-..."}}'

# Multiple files (the "photos" field must declare "max": >1, e.g. "max": 10)
curl -X POST http://localhost:8989/api/collections/articles/records \
  -H 'Content-Type: application/json' \
  -d '{"data": {"title": "Gallery Post", "photos": ["e97669ba-...", "f1284abc-..."]}}'
```

Single-file columns (`max` unset or `max: 1`) accept only a string (an array is rejected with `400`), while multi-file columns (`max > 1` or `min > 1`) require an array of file IDs/keys and enforce `min`/`max` item counts on save. In API responses, each stored ID/key is replaced by its `_files` record (`id`, `key`, `originalName`, `contentType`, `sizeBytes`, …); multi-file fields return an array of such records. IDs/keys without a metadata row are left as-is, and removing an item on update deletes the corresponding file.

### 3. Automatic Cleanup
- **Update**: If a file field is replaced with a new file or cleared, the old file record and its storage bytes are automatically purged from the backend.
- **Delete**: When a record is deleted, all files attached to its file fields are automatically removed from disk/S3 and the `_files` metadata table.

### 4. Listing Files in a Collection
Retrieve all files belonging to a collection:
```sh
curl http://localhost:8989/api/collections/articles/files
```
Governed by the collection's `list_rule`.

### 5. Downloading / Viewing Collection Files
```sh
curl http://localhost:8989/api/collections/articles/files/<file_id>
```
Governed by the collection's `view_rule`. Files can also be fetched from
`/api/files/<file_id>`, where the file's own collection supplies the rule. For
non-public collections, exchange a Bearer token for a short-lived file token at
`POST /api/files/token` and append `?token=...`; see
[Protected files](files-and-uploads.md#protected-files). Add `?download=1` to
force an attachment.

## Dynamic record filtering (`?filter`)

Crabbase supports client-side dynamic record filtering on `GET /api/collections/{name}/records` with full PocketBase compatibility. Clients can filter records by passing an expression in the `filter` query parameter. The expression is parsed into a safe AST and compiled to parameterized PostgreSQL SQL.

### Quick Examples

```sh
# Exact equality
curl -G 'http://localhost:8989/api/collections/posts/records' \
  --data-urlencode "filter=status='published'"

# Case-insensitive substring search (ILIKE '%pocket%')
curl -G 'http://localhost:8989/api/collections/products/records' \
  --data-urlencode "filter=title ~ 'pocket'"

# Negated substring search
curl -G 'http://localhost:8989/api/collections/users/records' \
  --data-urlencode "filter=email !~ 'spam'"

# Numeric and date range comparisons
curl -G 'http://localhost:8989/api/collections/orders/records' \
  --data-urlencode "filter=(total >= 100 && created > '2024-01-01 00:00:00Z')"

# Logical grouping with AND (&&), OR (||), and parentheses
curl -G 'http://localhost:8989/api/collections/tickets/records' \
  --data-urlencode "filter=(priority='urgent' || priority='high') && status!='resolved'"

# NULL and NOT NULL checks
curl -G 'http://localhost:8989/api/collections/tasks/records' \
  --data-urlencode "filter=completed_at = null"
curl -G 'http://localhost:8989/api/collections/tasks/records' \
  --data-urlencode "filter=assigned_to != null"

# Dynamic macro context (match current authenticated user)
curl -G 'http://localhost:8989/api/collections/posts/records' \
  -H "Authorization: Bearer $USER_TOKEN" \
  --data-urlencode "filter=author = @request.auth.id"
```

### Supported Operators

| Operator | Meaning | Example Filter | Generated PostgreSQL SQL |
|---|---|---|---|
| `=` | Equal | `status = 'active'` | `("status" = $1)` |
| `!=` | Not equal | `status != 'draft'` | `("status" != $1)` |
| `>` | Greater than | `created > '2024-01-01'` | `("created" > $1)` |
| `>=` | Greater or equal | `views >= 100` | `("views" >= 100)` |
| `<` | Less than | `price < 50` | `("price" < 50)` |
| `<=` | Less or equal | `age <= 18` | `("age" <= 18)` |
| `~` | Like / Contains (case-insensitive) | `title ~ 'rust'` | `("title" ILIKE ('%' \|\| $1 \|\| '%'))` |
| `!~` | Not like / Not contains | `email !~ 'spam'` | `("email" IS NULL OR "email" NOT ILIKE ('%' \|\| $1 \|\| '%'))` |
| `&&` / `&` | Logical AND | `status = 'active' && views > 0` | `(("status" = $1) AND ("views" > 0))` |
| `\|\|` / `\|` | Logical OR | `role = 'admin' \|\| role = 'editor'` | `(("role" = $1) OR ("role" = $2))` |
| `(...)` | Parentheses grouping | `(a = 1 \|\| b = 2) && c = 3` | `((("a" = 1) OR ("b" = 2)) AND ("c" = 3))` |
| `= null` | IS NULL check | `deleted_at = null` | `("deleted_at" IS NULL)` |
| `!= null` | IS NOT NULL check | `avatar != null` | `("avatar" IS NOT NULL)` |

### Literal Types & Escaping

- **Strings**: `'single-quoted'` or `"double-quoted"` (supports `\'` and `\"` escaping).
- **Numbers**: Integers (`42`), decimals (`3.14`), and signed values (`-5`, `+10`).
- **Booleans**: `true`, `false`, `TRUE`, `FALSE` (compiled to SQL `TRUE` / `FALSE`).
- **Null**: `null`, `NULL` (translated to `IS NULL` / `IS NOT NULL`).
- **Dates & Times**: RFC-3339 strings, e.g. `'2024-01-01'` or `'2024-01-01T12:00:00Z'`.

### Context Macros

- `@request.auth.<field>` — Dynamically resolved from the authenticated record's JWT claims (e.g. `@request.auth.id`, `@request.auth.email`).
- `@request.query.<param>` — Dynamically resolved from URL query parameters (e.g. `@request.query.search`).

### Security & Access Control Interaction

The client filter works alongside the collection's access control (`list_rule`):
- **Non-Admin (Public / Authenticated)**: If the collection has a `list_rule`, Crabbase automatically combines them safely as `({list_rule}) && ({client_filter})`. The client cannot bypass or weaken the server security rules. If `list_rule` is `None` (Admin-only), non-admin requests receive `403 Forbidden`.
- **Superuser (Admin)**: Superusers bypass the collection's `list_rule`, and the client `filter` expression is applied directly.

### Client Library / JavaScript Usage

```javascript
// Fetch filtered records with JavaScript / TypeScript fetch
async function fetchProducts({ search = '', minPrice = 0, page = 1 } = {}) {
  const filters = [];
  if (search) filters.push(`title ~ '${search.replace(/'/g, "\\'")}'`);
  if (minPrice > 0) filters.push(`price >= ${minPrice}`);

  const params = new URLSearchParams({
    page: page.toString(),
    per_page: '20',
  });

  if (filters.length > 0) {
    params.set('filter', filters.join(' && '));
  }

  const res = await fetch(`http://localhost:8989/api/collections/products/records?${params.toString()}`);
  return res.json();
}
```

### Searching & Filtering in the Admin Dashboard

In the Crabbase Admin Dashboard (`crabbase admin`):
1. Navigate to any collection.
2. In the top search bar (`Filter records (e.g. status = 'active' || title ~ 'john')...`), type any valid filter query.
3. The UI automatically debounces input by 350ms (or applies instantly when pressing `Enter`).
4. When a filter is active, clicking the `✕` icon clears the filter and restores all records.
5. If a filter expression has syntax errors, an inline error notification is displayed with the exact compiler error.

## Relations

Define them at creation time; Crabbase creates a real foreign key:

```json
{ "name": "category_id", "data_type": "Relation", "related_to": "categories" }
```

Store the related record's UUID in the field value. The referenced collection must exist before you can relate to it.

To fetch related records inline in the same API request instead of making separate queries, use the `?expand` query parameter (see [Expanding relations (`?expand`)](#expanding-relations-expand) below).

## Expanding relations (`?expand`)

Crabbase supports expanding relation fields inline on both list (`GET /api/collections/{name}/records`) and single-record (`GET /api/collections/{name}/records/{id}`) endpoints, matching PocketBase's `?expand` API contract.

Normally, relation fields store only the foreign-key UUID of the related record in `record.data.<field_name>`. Passing `?expand` resolves those foreign keys and embeds the full referenced record(s) inside a top-level `expand` map on each record in a single HTTP request.

### Quick Examples

```sh
# Single relation expansion on list endpoint
curl 'http://localhost:8989/api/collections/books/records?expand=author'

# Multiple relation expansions (comma-separated)
curl 'http://localhost:8989/api/collections/articles/records?expand=author,category'

# Expanding a single record by ID
curl 'http://localhost:8989/api/collections/books/records/a988d3e2-8d7b-410a-b32d-209a896d8e21?expand=author'

# Combining ?filter, pagination, and ?expand
curl -G 'http://localhost:8989/api/collections/books/records' \
  --data-urlencode "filter=published_year >= 1950" \
  --data-urlencode "expand=author,category" \
  -d "page=1" -d "per_page=20"
```

### Response Payload Structure

When `?expand` is specified, each expanded relation is embedded into a top-level `expand` object on the record, keyed by the relation field name:

```json
{
  "id": "a988d3e2-8d7b-410a-b32d-209a896d8e21",
  "data": {
    "title": "1984",
    "author": "7d91d79a-1153-4cf0-9ec8-6cf455bb3f91",
    "isbn": "9780451524935"
  },
  "expand": {
    "author": {
      "id": "7d91d79a-1153-4cf0-9ec8-6cf455bb3f91",
      "data": {
        "name": "George Orwell",
        "bio": "English novelist, essayist, journalist, and critic."
      },
      "created": "2026-09-09T14:30:00Z",
      "updated": "2026-09-09T14:30:00Z"
    }
  },
  "created": "2026-09-09T14:30:00Z",
  "updated": "2026-09-09T14:30:00Z"
}
```

> **Zero Payload Bloat:** When `?expand` is omitted or no relations are resolved, the `"expand"` field is completely omitted from the JSON payload (`Option::None`), keeping standard query responses lightweight.

### Security, Authorization & Privacy Invariants

Expanding relations across collection boundaries can easily introduce Insecure Direct Object References (IDOR) or leak hidden columns if not strictly authorized. Crabbase enforces the following security invariants at the database query level:

1. **Target View Rule Enforcement**:
   Every expand query compiles and executes the target collection's `view_rule` using the requester's authentication context (`SqlContext`):
   - **Admin / Superuser**: Bypasses `view_rule`.
   - **Non-Admin / Public**:
     - If the target collection has `view_rule = null` (admin-only), non-admins cannot expand this collection. The relation is silently omitted from `record.expand`.
     - If the target collection has a `view_rule` expression (e.g. `public = true || owner = @request.auth.id`), the rule is compiled to SQL and appended:
       ```sql
       SELECT * FROM "authors" WHERE id = ANY($1) AND (<target_view_rule>)
       ```
   - **Silent Degradation**: If an unauthenticated or non-admin user is not authorized to view a specific target record, PostgreSQL filters that record out. It is omitted from `record.expand` without failing the parent request, matching PocketBase's security model.

2. **Automatic Redaction of `hidden` Fields**:
   Before attaching target records into `record.expand`, all fields in the target collection defined with `"hidden": true` are stripped from `record.data`. Internal fields (such as email visibility flags, verification tokens, or private notes) are never leaked via relation expansion.

### Performance & Database Architecture

Crabbase implements relation expansion with connection-pool friendliness and zero $N+1$ query overhead:

| Feature | Implementation | Benefit |
|---|---|---|
| **$N+1$ Query Elimination** | Uses PostgreSQL `WHERE id = ANY($1::uuid[])` batch queries | Executes **$1 + M$ queries** for $M$ relations instead of $1 + (N \times M)$ queries. |
| **Foreign Key Deduplication** | Collects unique foreign keys across the page using `HashSet<Uuid>` | Eliminates redundant database lookups when multiple parent records share the same target (e.g. 50 articles by 2 authors queries only 2 UUIDs). |
| **Connection Pool Safety** | Executes relation batch queries sequentially | Holds at most 1 connection at a time, avoiding pool exhaustion on servers with small pools (`max_connections = 10`). |
| **Zero-Cost Short-Circuit** | Checks `unique_uuids.is_empty()` before dispatch | If all records have `null` or empty foreign keys for a relation, 0 database queries are executed. |
| **Prepared Statement Plan Caching** | Uses static `WHERE id = ANY($1)` syntax | Allows PostgreSQL to reuse prepared execution plans and index scans, unlike dynamic `IN ($1, $2, ...)`. |
| **In-Place Mutation** | Modifies records in-place (`&mut [Record]`) | Zero memory re-allocation or record cloning overhead. |

### Client Integration Examples

#### TypeScript / JavaScript Fetch

```typescript
interface Author {
  id: string;
  data: {
    name: string;
    bio?: string;
  };
  created: string;
  updated: string;
}

interface Book {
  id: string;
  data: {
    title: string;
    author: string; // Foreign key UUID
    isbn?: string;
  };
  expand?: {
    author?: Author;
  };
  created: string;
  updated: string;
}

// Fetch books with expanded author relation
async function fetchBooks(): Promise<Book[]> {
  const params = new URLSearchParams({
    page: '1',
    per_page: '20',
    expand: 'author',
  });

  const res = await fetch(`http://localhost:8989/api/collections/books/records?${params}`);
  if (!res.ok) {
    throw new Error(`Failed to fetch books: ${res.statusText}`);
  }

  const json = await res.json();
  return json.items;
}

// Example usage:
const books = await fetchBooks();
for (const book of books) {
  const authorName = book.expand?.author?.data.name ?? 'Unknown Author';
  console.log(`"${book.data.title}" by ${authorName}`);
}
```

#### Python (Requests / HTTPX)

```python
import requests

# Fetch single book with author expanded
response = requests.get(
    "http://localhost:8989/api/collections/books/records/a988d3e2-8d7b-410a-b32d-209a896d8e21",
    params={"expand": "author"}
)
response.raise_for_status()
book = response.json()

title = book["data"]["title"]
author_name = book.get("expand", {}).get("author", {}).get("data", {}).get("name", "Unknown")
print(f"Book: {title}, Author: {author_name}")
```

## API rules

Each collection stores rule strings — `list_rule`, `view_rule`, `create_rule`, `update_rule`, `delete_rule` — written in a PocketBase-style filter language, compiled to parameterized SQL (no string injection of user values).

Supported syntax:

- Operators: `= != < > <= >= ~ !~` (`~` is SQL `LIKE` / `ILIKE`, `!~` is `NOT LIKE`)
- Boolean combinators: `&&` and `||`, grouping with parentheses
- Literals: quoted strings, numbers, `true`/`false`, `null`
- Context variables:
  - `@request.auth.<field>` — a field from the authenticated record's row (e.g. `@request.auth.id`)
  - `@request.query.<param>` — a query-string parameter from the current request

Examples:

```
owner = @request.auth.id
status = 'active'
(owner = @request.auth.id) && status = 'active'
title ~ @request.query.search
```

Set rules when creating or updating a collection:

```sh
curl -X PATCH http://localhost:8989/api/collections/tasks \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{ "list_rule": "owner = @request.auth.id && done = false" }'
```

> **Rule Enforcement Status:** `list_rule` (list), `view_rule` (single-record fetch and `?expand` targets), `create_rule` (create/upload), and `delete_rule` (delete) are enforced. `update_rule` rejects non-admins when it is null, but expressions are not applied to update rows yet (an empty string allows all non-admins).
