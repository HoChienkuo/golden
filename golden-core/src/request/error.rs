/// An error produced while extracting a request entity.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RequestEntityError {
    #[error("failed to extract path variables: {message}")]
    PathExtraction { message: String },

    #[error("missing path variable `{name}`")]
    MissingPath { name: &'static str },

    #[error("invalid path variable `{name}` with value `{value}`: {message}")]
    InvalidPath {
        name: &'static str,
        value: String,
        message: String,
    },

    #[error("failed to extract query parameters: {message}")]
    QueryExtraction { message: String },

    #[error("missing query parameter `{name}`")]
    MissingQuery { name: &'static str },

    #[error("invalid query parameter `{name}` with value `{value}`: {message}")]
    InvalidQuery {
        name: &'static str,
        value: String,
        message: String,
    },

    #[error("missing request header `{name}`")]
    MissingHeader { name: String },

    #[error("invalid request header `{name}`: {message}")]
    InvalidHeader { name: String, message: String },

    #[error("invalid JSON request body: {message}")]
    InvalidBody { message: String },

    #[error("request validation failed: {message}")]
    Validation { message: String },
}
