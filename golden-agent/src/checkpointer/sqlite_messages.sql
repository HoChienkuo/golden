CREATE TABLE IF NOT EXISTS messages (
    id           TEXT    PRIMARY KEY,
    thread_id    TEXT    NOT NULL,
    run_id       TEXT    NOT NULL,
    seq          INTEGER NOT NULL,
    role         TEXT    NOT NULL,
    content      TEXT    NOT NULL,
    tool_calls   TEXT,
    tool_call_id TEXT,
    created_at   INTEGER NOT NULL,
    is_context   INTEGER NOT NULL DEFAULT 1,
    is_summary   INTEGER NOT NULL DEFAULT 0,
    summary_of   TEXT,
    UNIQUE (thread_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_messages_thread_ctx
    ON messages (thread_id, is_context, created_at, seq);
