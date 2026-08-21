/// An error produced while extracting a request entity.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RequestEntityError {
    #[error("missing path variable `{name}`")]
    MissingPath {
        name: &'static str,
    },

    #[error("invalid path variable `{name}`: {message}")]
    InvalidPath {
        name: &'static str,
        message: String,
    },

    #[error("missing query parameter `{name}`")]
    MissingQuery {
        name: &'static str,
    },

    #[error("invalid query parameter `{name}`: {message}")]
    InvalidQuery {
        name: &'static str,
        message: String,
    },

    #[error("missing request header `{name}`")]
    MissingHeader {
        name: &'static str,
    },

    #[error("invalid request header `{name}`: {message}")]
    InvalidHeader {
        name: &'static str,
        message: String,
    },

    #[error("invalid JSON request body: {message}")]
    InvalidBody {
        message: String,
    },

    #[error("request validation failed: {message}")]
    Validation {
        message: String,
    },
}