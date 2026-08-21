use golden_boot::{
    ApiResponse, IntoResponse, RequestEntity, RequestEntityError, Response, ResponseEntity,
    get_mapping, golden_boot_application, header, post_mapping,
};

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error(transparent)]
    Request(#[from] RequestEntityError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        ResponseEntity::bad_request(ApiResponse::<()>::error(40000, self.to_string()))
            .into_response()
    }
}

#[derive(Debug, RequestEntity)]
#[request_entity(rejection = ApiError)]
struct GetArticleRequest {
    #[path_variable]
    id: u64,

    #[request_param(default = 1)]
    page: u32,

    #[request_param(default = 10)]
    size: u32,

    #[request_header(
        name = header::AUTHORIZATION
    )]
    authorization: Option<String>,
}

#[get_mapping("/articles/{id}")]
async fn get_article(request: GetArticleRequest) -> ResponseEntity<ApiResponse<String>> {
    ResponseEntity::ok(ApiResponse::success(format!(
        "id={}, page={}, size={}, authorization={}",
        request.id,
        request.page,
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
    #[path_variable]
    id: u64,

    #[request_body]
    body: T,
}

#[post_mapping("/articles/{id}")]
async fn create_article(
    request: CreateArticleRequest<CreateArticleBody>,
) -> ResponseEntity<ApiResponse<String>> {
    ResponseEntity::ok(ApiResponse::success(format!(
        "created article {}: {}",
        request.id, request.body.name,
    )))
}

#[golden_boot_application]
async fn main() {}
