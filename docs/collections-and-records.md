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
| `GET` | `/api/collections/{name}/records?page=&per_page=` | rules apply | List records |
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

## Relations

Define them at creation time; Crabbase creates a real foreign key:

```json
{ "name": "category_id", "data_type": "Relation", "related_to": "categories" }
```

Store the related record's UUID in the field value. The referenced collection must exist before you can relate to it.

## API rules

Each collection stores rule strings — `list_rule`, `view_rule`, `create_rule`, `update_rule`, `delete_rule` — written in a PocketBase-style filter language, compiled to parameterized SQL (no string injection of user values).

Supported syntax:

- Operators: `= != < > <= >= ~` (`~` is SQL `LIKE`)
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
