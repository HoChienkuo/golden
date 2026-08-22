CREATE TABLE IF NOT EXISTS articles
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    content    TEXT    NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
    );

INSERT INTO articles (name, content)
VALUES ('GoldenBoot', 'Hello GoldenBoot and SQLite');