use golden_boot::{
    ApiResponse,
    IntoResponse,
    PaginationError,
    RequestEntityError,
    Response,
    ResponseEntity,
    StatusCode,
};

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("failed to prepare application data directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to initialize database: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Request(#[from] RequestEntityError),

    #[error(transparent)]
    Pagination(#[from] PaginationError),

    #[error("article {0} was not found")]
    ArticleNotFound(i64),

    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::Request(_) | Self::Pagination(_) => {
                (StatusCode::BAD_REQUEST, 40000)
            }

            Self::ArticleNotFound(_) => {
                (StatusCode::NOT_FOUND, 40400)
            }

            Self::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, 50000)
            }
        };

        ResponseEntity::new(
            status,
            ApiResponse::<()>::error(code, self.to_string()),
        )
            .into_response()
    }
}