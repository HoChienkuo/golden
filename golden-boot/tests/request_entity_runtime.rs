use golden_boot::__private::axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
    routing::{get, post},
};
use golden_boot::{
    IntoResponse, RequestEntity, RequestEntityError, Response, StatusCode, Validate, header,
};
use serde::Deserialize;
use tower::ServiceExt;

#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error(transparent)]
    Request(#[from] RequestEntityError),
}

impl IntoResponse for TestError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = TestError)]
struct ArticleParts {
    #[path_variable(name = "id")]
    article_id: u64,

    #[request_param(default = 1)]
    page: u32,

    #[request_param(name = "q")]
    search: Option<String>,

    #[request_header(name = header::AUTHORIZATION)]
    authorization: Option<String>,
}

async fn read_article(request: ArticleParts) -> String {
    format!(
        "id={};page={};search={};authorization={}",
        request.article_id,
        request.page,
        request.search.as_deref().unwrap_or("none"),
        request.authorization.as_deref().unwrap_or("none"),
    )
}

#[derive(Debug, Deserialize)]
struct ArticleBody {
    name: String,
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = TestError)]
struct CreateArticle<T> {
    #[path_variable]
    id: u64,

    #[request_body]
    body: T,
}

async fn create_article(request: CreateArticle<ArticleBody>) -> String {
    format!("id={};name={}", request.id, request.body.name)
}

#[derive(Debug, RequestEntity, Validate)]
#[request_entity(rejection = TestError, validate)]
struct PageRequest {
    #[request_param]
    #[validate(range(min = 1, max = 100))]
    size: u32,
}

async fn validate_page(request: PageRequest) -> String {
    request.size.to_string()
}

async fn response_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");

    String::from_utf8(bytes.to_vec()).expect("response body should be UTF-8")
}

#[tokio::test]
async fn extracts_path_query_default_optional_query_and_header() {
    let app = Router::new().route("/articles/{id}", get(read_article));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/articles/42?q=golden")
                .header(header::AUTHORIZATION, "Bearer test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_text(response).await,
        "id=42;page=1;search=golden;authorization=Bearer test",
    );
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = TestError)]
struct RequiredParts {
    #[path_variable]
    id: u64,

    #[request_param(name = "page-size")]
    page_size: u32,

    #[request_header(name = header::HeaderName::from_static("x-request-id"))]
    request_id: u64,
}

async fn required_parts(request: RequiredParts) -> String {
    format!(
        "id={};page-size={};request-id={}",
        request.id, request.page_size, request.request_id,
    )
}

#[tokio::test]
async fn reports_missing_renamed_query_parameter() {
    let app = Router::new().route("/required/{id}", get(required_parts));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/required/1")
                .header("x-request-id", "9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_text(response).await,
        "missing query parameter `page-size`",
    );
}

#[tokio::test]
async fn reports_invalid_path_name_and_value() {
    let app = Router::new().route("/required/{id}", get(required_parts));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/required/not-a-number?page-size=10")
                .header("x-request-id", "9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_text(response).await;
    assert!(body.contains("path variable `id`"));
    assert!(body.contains("value `not-a-number`"));
}

#[tokio::test]
async fn reports_invalid_query_name_and_value() {
    let app = Router::new().route("/required/{id}", get(required_parts));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/required/1?page-size=large")
                .header("x-request-id", "9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_text(response).await;
    assert!(body.contains("query parameter `page-size`"));
    assert!(body.contains("value `large`"));
}

#[tokio::test]
async fn reports_the_actual_missing_header_name() {
    let app = Router::new().route("/required/{id}", get(required_parts));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/required/1?page-size=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_text(response).await,
        "missing request header `x-request-id`",
    );
}

#[tokio::test]
async fn reports_invalid_header_without_echoing_its_value() {
    let app = Router::new().route("/required/{id}", get(required_parts));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/required/1?page-size=10")
                .header("x-request-id", "secret-invalid-value")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_text(response).await;
    assert!(body.contains("request header `x-request-id`"));
    assert!(!body.contains("secret-invalid-value"));
}

#[tokio::test]
async fn extracts_generic_json_body_together_with_path() {
    let app = Router::new().route("/articles/{id}", post(create_article));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/articles/7")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"GoldenBoot"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_text(response).await, "id=7;name=GoldenBoot");
}

#[tokio::test]
async fn converts_invalid_json_to_the_user_rejection() {
    let app = Router::new().route("/articles/{id}", post(create_article));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/articles/7")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("not-json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_text(response)
            .await
            .contains("invalid JSON request body")
    );
}

#[tokio::test]
async fn converts_validation_failure_to_the_user_rejection() {
    let app = Router::new().route("/pages", get(validate_page));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/pages?size=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_text(response)
            .await
            .contains("request validation failed")
    );
}
