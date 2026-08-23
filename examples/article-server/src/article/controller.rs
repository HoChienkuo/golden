use crate::article::model::{Article, CreateArticleBody, UpdateArticleBody};
use crate::error::ApiError;
use crate::state::AppState;
use golden_boot::{
    ApiResponse, Body, Event, IntoResponse, KeepAlive, Multipart, Page, RequestEntity, Response,
    ResponseEntity, Sse, State, StatusCode, Validate, delete_mapping, get_mapping, header,
    post_mapping, put_mapping,
};
use std::convert::Infallible;
use std::fs;
use std::time::Duration;
use tokio_stream::{StreamExt, wrappers::IntervalStream};

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

#[get_mapping("/file/exmaple")]
async fn download() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            r#"attachment; filename="example.txt""#,
        )
        .body(Body::from("Hello GoldenBoot!"))
        .expect("static response headers are valid")
}

#[post_mapping("/upload")]
async fn upload(mut multipart: Multipart) -> Response {
    while let Some(field) = multipart.next_field().await.unwrap() {
        // 保存文件
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let data = field.bytes().await.unwrap();
        fs::write(format!("./uploads/{}", file_name), data).unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .body("Upload success".into())
        .expect("static response headers are valid")
}

#[get_mapping("/sse/events")]
async fn events() -> impl IntoResponse {
    let interval = tokio::time::interval(Duration::from_secs(1));

    let mut index = 0_u64;

    let stream = IntervalStream::new(interval).map(move |_| {
        let current = index;
        index += 1;

        Ok::<Event, Infallible>(
            Event::default()
                .event("article")
                .id(current.to_string())
                .data(format!("event {current}")),
        )
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
