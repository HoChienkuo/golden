use golden_boot::{Path, Query, ResponseEntity, get_mapping, golden_boot_application,
};

use serde::{Deserialize, Serialize};

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
) -> ResponseEntity<Article> {
    ResponseEntity::ok(Article {
        id,
        page: query.page,
    })
}

#[golden_boot_application(port = 8080)]
async fn main() {
    println!("Hello, world!");
}
