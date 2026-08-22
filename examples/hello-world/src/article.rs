use golden_boot::{
    ApiResponse, RequestEntity, ResponseEntity, State, get_mapping, header, post_mapping,
};

use crate::{error::ApiError, state::AppState};

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = ApiError)]
struct GetArticleRequest {
    #[path_variable(name = "id")]
    article_id: u64,

    #[request_param(name = "page", default = 1)]
    page_number: u32,

    #[request_param(default = 10)]
    size: u32,

    #[request_header(name = header::AUTHORIZATION)]
    authorization: Option<String>,
}

#[get_mapping("/articles/{id}")]
async fn get_article(
    State(state): State<AppState>,
    request: GetArticleRequest,
) -> ResponseEntity<ApiResponse<String>> {
    let article = state.article_service.find(request.article_id).await;

    ResponseEntity::ok(ApiResponse::success(format!(
        "{article}, page={}, size={}, authorization={}",
        request.page_number,
        request.size,
        request.authorization.as_deref().unwrap_or("<none>"),
    )))
}

#[derive(Debug, serde::Deserialize)]
struct CreateArticleBody {
    name: String,
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = ApiError)]
struct CreateArticleRequest<T> {
    #[path_variable(name = "id")]
    article_id: u64,

    #[request_body]
    body: T,
}

#[post_mapping("/articles/{id}")]
async fn create_article(
    request: CreateArticleRequest<CreateArticleBody>,
) -> ResponseEntity<ApiResponse<String>> {
    ResponseEntity::ok(ApiResponse::success(format!(
        "created article {}: {}",
        request.article_id, request.body.name,
    )))
}
