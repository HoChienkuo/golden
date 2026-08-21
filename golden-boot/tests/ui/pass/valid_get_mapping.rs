use golden_boot::{
    get_mapping,
    Json,
    Path,
    Query,
};

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

fn main() {}