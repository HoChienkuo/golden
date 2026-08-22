use crate::article::model::{Article, CreateArticleBody, UpdateArticleBody};
use crate::error::ApiError;
use crate::state::AppState;
use golden_boot::{
    ApiResponse, Page, RequestEntity, ResponseEntity, State, Validate, delete_mapping, get_mapping,
    post_mapping, put_mapping,
};

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = ApiError)]
struct ArticleIdRequest {
    #[path_variable(name = "id")]
    id: i64,
}

#[derive(Debug, RequestEntity, Validate)]
#[request_entity(rejection = ApiError, validate)]
struct ArticlePageRequest {
    #[request_param(default = 1)]
    #[validate(range(min = 1))]
    page: u32,

    #[request_param(default = 10)]
    #[validate(range(min = 1, max = 100))]
    size: u32,
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = ApiError)]
struct BodyRequest<T> {
    #[request_body]
    body: T,
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = ApiError)]
struct ArticleBodyRequest<T> {
    #[path_variable(name = "id")]
    id: i64,

    #[request_body]
    body: T,
}
#[get_mapping("/articles/{id}")]
async fn find_article(
    State(state): State<AppState>,
    request: ArticleIdRequest,
) -> Result<ResponseEntity<ApiResponse<Article>>, ApiError> {
    let article = state
        .article_service
        .find_by_id(request.id)
        .await?
        .ok_or(ApiError::ArticleNotFound(request.id))?;

    Ok(ResponseEntity::ok(ApiResponse::success(article)))
}

#[get_mapping("/articles")]
async fn find_articles(
    State(state): State<AppState>,
    request: ArticlePageRequest,
) -> Result<ResponseEntity<ApiResponse<Page<Article>>>, ApiError> {
    let articles = state
        .article_service
        .find_all(request.page, request.size)
        .await?;

    let total = state.article_service.count().await?;

    let page = Page::new(articles, request.page, request.size, total as u64)?;

    Ok(ResponseEntity::ok(ApiResponse::success(page)))
}

#[post_mapping("/articles")]
async fn create_article(
    State(state): State<AppState>,
    request: BodyRequest<CreateArticleBody>,
) -> Result<ResponseEntity<ApiResponse<Article>>, ApiError> {
    let article = state.article_service.create(request.body).await?;

    Ok(ResponseEntity::created(ApiResponse::success(article)))
}

#[put_mapping("/articles/{id}")]
async fn update_article(
    State(state): State<AppState>,
    request: ArticleBodyRequest<UpdateArticleBody>,
) -> Result<ResponseEntity<ApiResponse<Article>>, ApiError> {
    let article = state
        .article_service
        .update(request.id, request.body)
        .await?
        .ok_or(ApiError::ArticleNotFound(request.id))?;

    Ok(ResponseEntity::ok(ApiResponse::success(article)))
}

#[delete_mapping("/articles/{id}")]
async fn delete_article(
    State(state): State<AppState>,
    request: ArticleIdRequest,
) -> Result<ResponseEntity<ApiResponse<()>>, ApiError> {
    let deleted = state.article_service.delete(request.id).await?;

    if !deleted {
        return Err(ApiError::ArticleNotFound(request.id));
    }

    Ok(ResponseEntity::ok(ApiResponse::success_empty()))
}
