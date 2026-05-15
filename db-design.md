Now let me get the complete schema details for all system tables:Now let me get the full system collections code to provide a complete schema overview:Now I have all the information I need. Let me compile the complete SQL schema for PocketBase:

## Complete PocketBase SQL Schema

Based on the source code analysis, here's the complete SQL schema for all PocketBase entities:

### **System Tables (Core Infrastructure)**

#### 1. **_params** - Application Settings/Parameters
```sql
CREATE TABLE _params (
    id      TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    value   JSON DEFAULT NULL,
    created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')) NOT NULL,
    updated TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')) NOT NULL
);
```

#### 2. **_collections** - Collection Metadata & Schema Definition
```sql
CREATE TABLE _collections (
    id         TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    system     BOOLEAN DEFAULT FALSE NOT NULL,
    type       TEXT DEFAULT "base" NOT NULL,  -- "base", "auth", or "view"
    name       TEXT UNIQUE NOT NULL,
    fields     JSON DEFAULT "[]" NOT NULL,     -- Array of field definitions
    indexes    JSON DEFAULT "[]" NOT NULL,     -- Array of index definitions
    listRule   TEXT DEFAULT NULL,              -- Filter rule for LIST operations
    viewRule   TEXT DEFAULT NULL,              -- Filter rule for VIEW operations
    createRule TEXT DEFAULT NULL,              -- Filter rule for CREATE operations
    updateRule TEXT DEFAULT NULL,              -- Filter rule for UPDATE operations
    deleteRule TEXT DEFAULT NULL,              -- Filter rule for DELETE operations
    options    JSON DEFAULT "{}" NOT NULL,     -- Type-specific options (auth/view configs)
    created    TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')) NOT NULL,
    updated    TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')) NOT NULL
);

CREATE INDEX idx__collections_type ON _collections (type);
```

#### 3. **_migrations** - Migration History (Aux DB)
```sql
CREATE TABLE _migrations (
    id   TEXT PRIMARY KEY,
    file TEXT UNIQUE NOT NULL,
    applied_at TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')) NOT NULL
);
```

#### 4. **_logs** - Application Logs (Aux DB)
```sql
CREATE TABLE _logs (
    id      TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    level   INTEGER DEFAULT 0 NOT NULL,
    message TEXT DEFAULT "" NOT NULL,
    data    JSON DEFAULT "{}" NOT NULL,
    created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')) NOT NULL
);

CREATE INDEX idx_logs_level ON _logs (level);
CREATE INDEX idx_logs_message ON _logs (message);
CREATE INDEX idx_logs_created_hour ON _logs (strftime('%Y-%m-%d %H:00:00', created));
```

---

### **System Collections (Auto-Created)**

#### 5. **_superusers** - Administrator Accounts
```sql
CREATE TABLE _superusers (
    -- System Fields
    id       TEXT PRIMARY KEY,
    created  TEXT NOT NULL,
    updated  TEXT NOT NULL,
    
    -- Auth Fields (inherited from auth type)
    email       TEXT UNIQUE NOT NULL,
    password    TEXT NOT NULL,
    tokenKey    TEXT NOT NULL,
    emailVisibility BOOLEAN DEFAULT FALSE,
    verified    BOOLEAN DEFAULT FALSE
);

CREATE INDEX idx__superusers_email ON _superusers (email);
CREATE INDEX idx__superusers_tokenKey ON _superusers (tokenKey);
```

#### 6. **_mfas** - Multi-Factor Authentication
```sql
CREATE TABLE _mfas (
    id            TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    collectionRef TEXT NOT NULL,  -- Reference to collection
    recordRef     TEXT NOT NULL,  -- Reference to record in that collection
    method        TEXT NOT NULL,  -- MFA method (password, oauth2, otp)
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL
);

CREATE INDEX idx_mfas_collectionRef_recordRef ON _mfas (collectionRef, recordRef);
```

#### 7. **_otps** - One-Time Passwords
```sql
CREATE TABLE _otps (
    id            TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    collectionRef TEXT NOT NULL,
    recordRef     TEXT NOT NULL,
    password      TEXT NOT NULL,  -- Hashed OTP
    sentTo        TEXT,           -- Email sent to
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL
);

CREATE INDEX idx_otps_collectionRef_recordRef ON _otps (collectionRef, recordRef);
```

#### 8. **_externalAuths** - OAuth2 / External Auth Integrations
```sql
CREATE TABLE _externalAuths (
    id            TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    collectionRef TEXT NOT NULL,
    recordRef     TEXT NOT NULL,
    provider      TEXT NOT NULL,  -- Provider name (google, github, etc.)
    providerId    TEXT NOT NULL,  -- External provider's user ID
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_externalAuths_record_provider 
    ON _externalAuths (collectionRef, recordRef, provider);
CREATE UNIQUE INDEX idx_externalAuths_collection_provider 
    ON _externalAuths (collectionRef, provider, providerId);
```

#### 9. **_authOrigins** - Device/Browser Fingerprints for Auth
```sql
CREATE TABLE _authOrigins (
    id            TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))) NOT NULL,
    collectionRef TEXT NOT NULL,
    recordRef     TEXT NOT NULL,
    fingerprint   TEXT NOT NULL,  -- Device fingerprint
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_authOrigins_unique_pairs 
    ON _authOrigins (collectionRef, recordRef, fingerprint);
```

#### 10. **users** - Default User Collection (Example Auth Collection)
```sql
CREATE TABLE users (
    -- System Fields
    id       TEXT PRIMARY KEY,
    created  TEXT NOT NULL,
    updated  TEXT NOT NULL,
    
    -- Auth Fields
    email           TEXT UNIQUE NOT NULL,
    password        TEXT NOT NULL,
    tokenKey        TEXT NOT NULL,
    emailVisibility BOOLEAN DEFAULT FALSE,
    verified        BOOLEAN DEFAULT FALSE,
    
    -- Custom Fields
    name            TEXT,
    avatar          TEXT  -- File reference
);

CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_users_tokenKey ON users (tokenKey);
```

---

### **Dynamic User Collections**

When you create a new collection (e.g., "posts", "comments"), PocketBase creates a table like:

#### Example: **posts** Collection
```sql
CREATE TABLE posts (
    id       TEXT PRIMARY KEY,  -- Auto-generated 15-char lowercase alphanumeric
    created  TEXT NOT NULL,      -- Autodate field
    updated  TEXT NOT NULL,      -- Autodate field
    
    -- Your custom fields based on collection schema
    title       TEXT,           -- TextField
    content     TEXT,           -- TextField  
    published   BOOLEAN,        -- BoolField
    views       INTEGER,        -- NumberField
    tags        JSON,           -- RelationField / JSONField
    author      TEXT,           -- RelationField (FK to users)
    metadata    JSON            -- JSONField
);

-- Any indexes defined in the collection schema
CREATE INDEX idx_posts_author ON posts (author);
```

---

### **Schema Characteristics**

| Aspect | Details |
|--------|---------|
| **Database** | SQLite |
| **Primary Keys** | Auto-generated 15-char random strings (lowercase alphanumeric) |
| **Timestamps** | ISO 8601 format with timezone (strftime '%Y-%m-%d %H:%M:%fZ') |
| **Field Storage** | Typed columns (TEXT, INTEGER, REAL, JSON, BOOLEAN) |
| **JSON Fields** | Stored as JSON type, not as strings |
| **Record ID Format** | `^[a-z0-9]{15}$` |
| **Default Database** | `pb_data/data.db` |
| **Aux Database** | `pb_data/auxiliary.db` (for logs & migrations) |

---

### **Key Design Patterns**

1. **Collection Metadata vs. Record Data**: Collection definitions live in `_collections`, actual records live in separate typed tables
2. **Soft Cascade**: Foreign keys use application-level logic (via filter rules)
3. **Field Definitions**: Stored as JSON array in `_collections.fields` for flexibility
4. **Access Control**: Rules (`listRule`, `viewRule`, etc.) stored as text expressions, evaluated at query time
5. **Default Timestamps**: Every non-view collection gets `created` and `updated` autodate fields

This schema design allows PocketBase to be truly "1 file" while maintaining full schema flexibility!
