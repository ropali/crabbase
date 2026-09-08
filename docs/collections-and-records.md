# Collections & records

Collections are the heart of Crabbase. A collection is a real PostgreSQL table that you define at runtime — and the moment it exists, a full REST CRUD surface is generated for it at `/api/collections/{name}/records`. This is how you build a backend without writing any API code.

## The easy way: the admin dashboard

Everything in this document can be done from the admin dashboard (`crabbase admin`, default `http://localhost:8181`) — no curl required:

| Task | Where in the UI |
|---|---|
| Create a collection (name, type, fields, indexes) | **Create Collection** drawer in the sidebar |
| Add/remove/retype fields after creation | Collection's **Edit Schema** drawer |
| Set API rules | Same edit drawer (rule fields per operation) |
| Browse / create / edit / delete records | Collection's data page |
| Delete all records | Data page → truncate |
| Delete a collection | Edit drawer |

The dashboard talks to the exact same REST endpoints documented below — use whichever fits your workflow (clicks for day-to-day work, API for automation/CI).

Collection management requires a **superuser** login (dashboard) or superuser JWT (API). Record endpoints are public by default unless restricted with an API rule.

## Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/collections?page=1&per_page=20` | superuser | List collections |
| `POST` | `/api/collections` | superuser | Create collection (creates the table) |
| `GET` | `/api/collections/{name}` | superuser | Get one collection |
| `PATCH` | `/api/collections/{name}` | superuser | Update schema / rename / set rules |
| `DELETE` | `/api/collections/{name}` | superuser | Delete collection **and drop its table** |
| `POST` | `/api/collections/{name}/truncate` | superuser | Delete all records |
| `GET` | `/api/collections/{name}/records?page=1&per_page=20&filter=` | rules apply | List records (with pagination & dynamic filtering) |
| `POST` | `/api/collections/{name}/records` | open* | Create record |
| `GET` | `/api/collections/{name}/records/{id}` | open* | Get record |
| `PATCH` | `/api/collections/{name}/records/{id}` | open* | Update record |
| `DELETE` | `/api/collections/{name}/records/{id}` | open* | Delete record |

\* Record endpoints currently have no built-in permission check other than the collection's `list_rule`, which is enforced on list. See [API rules](#api-rules).

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
    ]
  }'
```

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
| `File` | `TEXT` | Stores a path/name string only — **no upload endpoint yet** |
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
| `min` / `max` | — | Size constraints |
| `pattern` | — | Validation pattern |
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

# Get one
curl http://localhost:8989/api/collections/products/records/<id>

# Update
curl -X PATCH http://localhost:8989/api/collections/products/records/<id> \
  -H 'Content-Type: application/json' \
  -d '{"data": {"price": 1299}}'

# Delete
curl -X DELETE http://localhost:8989/api/collections/products/records/<id>
```

Pagination defaults: `page=1`, `per_page=20` (clamped to 1–100).

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

> **Current limitation:** only `list_rule` is enforced today. The other rules are stored but not yet evaluated — treat record read/create/update/delete on non-superuser collections as open until this lands.
