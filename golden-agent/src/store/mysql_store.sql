CREATE TABLE IF NOT EXISTS store_items (
    namespace  VARCHAR(512) NOT NULL,
    `key`      VARCHAR(191) NOT NULL,
    value      JSON         NOT NULL,
    created_at BIGINT       NOT NULL,
    updated_at BIGINT       NOT NULL,
    PRIMARY KEY (namespace(255), `key`),
    KEY idx_store_items_ns_updated (namespace(255), updated_at)
);
