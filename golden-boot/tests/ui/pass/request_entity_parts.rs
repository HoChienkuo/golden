use golden_boot::{IntoResponse, RequestEntity, RequestEntityError, Response, header};

struct ApiError;

impl From<RequestEntityError> for ApiError {
    fn from(_: RequestEntityError) -> Self {
        Self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        ().into_response()
    }
}

#[derive(RequestEntity)]
#[request_entity(rejection = ApiError)]
struct ArticleRequest {
    #[path_variable]
    id: u64,

    #[request_param(default = 1)]
    page: u32,

    #[request_header(name = header::AUTHORIZATION)]
    authorization: Option<String>,
}

fn main() {}
