CREATE TABLE IF NOT EXISTS messages (
    id           TEXT    PRIMARY KEY,
    thread_id    TEXT    NOT NULL,
    run_id       TEXT    NOT NULL,
    seq          INTEGER NOT NULL,
    role         TEXT    NOT NULL,
    content      TEXT    NOT NULL,
    tool_calls   TEXT,
    tool_call_id TEXT,
    reasoning    TEXT,
    reasoning_signature TEXT,
    created_at   INTEGER NOT NULL,
    is_context   INTEGER NOT NULL DEFAULT 1,
    UNIQUE (thread_id, seq)
);

ALTER TABLE messages ADD COLUMN reasoning TEXT;

ALTER TABLE messages ADD COLUMN reasoning_signature TEXT;

CREATE INDEX IF NOT EXISTS idx_messages_thread_ctx
    ON messages (thread_id, is_context, created_at, seq);
