use golden_boot::{Json, Path, Query, get_mapping, golden_boot_application, post_mapping};

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Deserialize)]
struct ArticleQuery {
    page: Option<u32>,
}

#[derive(Serialize)]
struct Article {
    id: u64,
    page: Option<u32>,
}

#[get_mapping("/articles/{id}")]
async fn get_article(
    Path(id): Path<u64>,
    Query(query): Query<ArticleQuery>,
) -> Json<Article> {
    Json(Article {
        id,
        page: query.page,
    })
}

#[get_mapping("/articles")]
async fn list_articles() {}

#[post_mapping("/articles")]
async fn create_article() {}

#[golden_boot_application(port = 8080)]
async fn main() {
    println!("Hello, world!");
}
