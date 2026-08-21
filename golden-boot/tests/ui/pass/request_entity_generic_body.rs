use golden_boot::{IntoResponse, RequestEntity, RequestEntityError, Response};
use serde::Deserialize;

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

#[derive(Deserialize)]
struct Body {
    name: String,
}

#[derive(RequestEntity)]
#[request_entity(rejection = ApiError)]
struct CreateRequest<T> {
    #[path_variable]
    id: u64,

    #[request_body]
    body: T,
}

fn assert_request_entity<T: golden_boot::RequestEntity>() {}

fn main() {
    assert_request_entity::<CreateRequest<Body>>();
}
