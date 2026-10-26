CREATE TABLE IF NOT EXISTS messages (
    id           TEXT        PRIMARY KEY,
    thread_id    TEXT        NOT NULL,
    run_id       TEXT        NOT NULL,
    seq          BIGINT      NOT NULL,
    role         TEXT        NOT NULL,
    content      JSONB       NOT NULL,
    tool_calls   JSONB,
    tool_call_id TEXT,
    created_at   BIGINT      NOT NULL,
    is_context   BOOLEAN     NOT NULL DEFAULT TRUE,
    is_summary   BOOLEAN     NOT NULL DEFAULT FALSE,
    summary_of   JSONB,
    UNIQUE (thread_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_messages_thread_ctx
    ON messages (thread_id, is_context, created_at, seq);
