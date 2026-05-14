CREATE TABLE IF NOT EXISTS entries (
    id BLOB PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at_physical_ms INTEGER NOT NULL,
    created_at_logical INTEGER NOT NULL,
    created_on_device BLOB NOT NULL,
    last_modified_at_physical_ms INTEGER NOT NULL,
    last_modified_at_logical INTEGER NOT NULL,
    last_modified_on_device BLOB NOT NULL
) STRICT;
