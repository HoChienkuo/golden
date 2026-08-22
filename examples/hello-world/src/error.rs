use golden_boot::{ApiResponse, IntoResponse, RequestEntityError, Response, ResponseEntity};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error(transparent)]
    Request(#[from] RequestEntityError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        ResponseEntity::bad_request(ApiResponse::<()>::error(40000, self.to_string()))
            .into_response()
    }
}
