use crate::article::model::{Article, CreateArticleBody, UpdateArticleBody};
use sqlx::SqlitePool;

pub struct ArticleService {
    pool: SqlitePool,
}

impl ArticleService {
    pub fn new(pool: SqlitePool) -> ArticleService {
        Self { pool }
    }

    pub async fn create(&self, body: CreateArticleBody) -> Result<Article, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
            INSERT INTO articles (name, content)
            VALUES (?, ?)
            RETURNING id, name, content, created_at
            "#,
        )
        .bind(body.name)
        .bind(body.content)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(
        &self,
        id: i64,
        body: UpdateArticleBody,
    ) -> Result<Option<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
            UPDATE articles
            SET name = ?, content = ?
            WHERE id = ?
            RETURNING id, name, content, created_at
            "#,
        )
        .bind(body.name)
        .bind(body.content)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM articles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<Article>, sqlx::Error> {
        sqlx::query_as::<_, Article>(
            r#"
                SELECT id, name, content, created_at
                FROM articles
                WHERE id = ?
                "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_all(&self, page: u32, size: u32) -> Result<Vec<Article>, sqlx::Error> {
        let offset = i64::from(page.saturating_sub(1)) * i64::from(size);

        sqlx::query_as::<_, Article>(
            r#"
                SELECT id, name, content, created_at
                FROM articles
                ORDER BY id DESC
                LIMIT ? OFFSET ?
                "#,
        )
        .bind(i64::from(size))
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM articles")
            .fetch_one(&self.pool)
            .await
    }
}
