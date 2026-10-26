CREATE TABLE IF NOT EXISTS messages (
    id           VARCHAR(64)  NOT NULL,
    thread_id    VARCHAR(191) NOT NULL,
    run_id       VARCHAR(64)  NOT NULL,
    seq          BIGINT       NOT NULL,
    role         VARCHAR(16)  NOT NULL,
    content      JSON         NOT NULL,
    tool_calls   JSON         NULL,
    tool_call_id VARCHAR(64)  NULL,
    created_at   BIGINT       NOT NULL,
    is_context   TINYINT(1)   NOT NULL DEFAULT 1,
    is_summary   TINYINT(1)   NOT NULL DEFAULT 0,
    summary_of   JSON         NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_messages_thread_seq (thread_id, seq),
    KEY idx_messages_thread_ctx (thread_id, is_context, created_at, seq)
);
