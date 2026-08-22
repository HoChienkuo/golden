use std::{str::FromStr, sync::Arc};

use crate::article::service::ArticleService;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

#[derive(Clone)]
pub struct AppState {
    pub article_service: Arc<ArticleService>,
}

impl AppState {
    pub async fn initialize() -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str("sqlite://data/golden.db")?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self {
            article_service: Arc::new(ArticleService::new(pool)),
        })
    }
}
