CREATE TABLE IF NOT EXISTS store_items (
    namespace  TEXT    NOT NULL,
    key        TEXT    NOT NULL,
    value      JSONB   NOT NULL,
    created_at BIGINT  NOT NULL,
    updated_at BIGINT  NOT NULL,
    PRIMARY KEY (namespace, key)
);

CREATE INDEX IF NOT EXISTS idx_store_items_ns_updated
    ON store_items (namespace, updated_at);
