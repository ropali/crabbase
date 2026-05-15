-- Create a new collection table with dynamic columns
-- Placeholders: {name} = collection name, {column_defs} = column definitions
CREATE TABLE IF NOT EXISTS "{name}" (
    id INTEGER PRIMARY KEY,
    {column_defs}
    created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
    updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
);

-- Index template (one per column with index: true)
-- Placeholders: {name} = collection name, {column} = column name
-- CREATE INDEX idx_{name}_{column} ON "{name}" ("{column}");
