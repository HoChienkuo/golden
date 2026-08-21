use golden_boot::{
    ApiResponse, Page, Path, Query, ResponseEntity, get_mapping,
    golden_boot_application,
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ArticleQuery {
    page: u32,
    size: u32,
}

#[derive(Serialize)]
struct Article {
    id: u64,
    name: String,
}

#[get_mapping("/articles/{id}")]
async fn get_article(Path(id): Path<u64>) -> ResponseEntity<ApiResponse<Article>> {
    ResponseEntity::ok(ApiResponse::success(Article {
        id,
        name: String::new(),
    }))
}

#[get_mapping("/articles")]
async fn list_articles(
    Query(query): Query<ArticleQuery>,
) -> ResponseEntity<ApiResponse<Page<Article>>> {
    let articles = vec![
        Article {
            id: 1,
            name: "First".to_owned(),
        },
        Article {
            id: 2,
            name: "Second".to_owned(),
        },
    ];

    let page = match Page::new(articles, query.page, query.size, 101) {
        Ok(page) => page,

        Err(error) => {
            return ResponseEntity::bad_request(ApiResponse::<Page<Article>>::error(
                40001,
                error.to_string(),
            ));
        }
    };

    ResponseEntity::ok(ApiResponse::success(page))
}

#[golden_boot_application(port = 8080)]
async fn main() {
    println!("Hello, world!");
}
