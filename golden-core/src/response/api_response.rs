use serde::Serialize;

/// A standard API response containing a business code,
/// message, and optional data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiResponse<T = ()> {
    code: i32,
    msg: String,
    data: Option<T>,
}

impl<T> ApiResponse<T> {
    /// Creates an API response.
    pub fn new(code: i32, msg: impl Into<String>, data: Option<T>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data,
        }
    }

    /// Creates a successful response.
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            msg: "success".to_owned(),
            data: Some(data),
        }
    }

    /// Creates a successful response with a custom message.
    pub fn success_with_message(msg: impl Into<String>, data: T) -> Self {
        Self {
            code: 0,
            msg: msg.into(),
            data: Some(data),
        }
    }

    /// Creates an error response without data.
    pub fn error(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: None,
        }
    }

    /// Returns the business response code.
    pub fn code(&self) -> i32 {
        self.code
    }

    /// Returns the response message.
    pub fn msg(&self) -> &str {
        &self.msg
    }

    /// Returns the response data.
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Returns mutable response data.
    pub fn data_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    /// Consumes the response and returns its data.
    pub fn into_data(self) -> Option<T> {
        self.data
    }

    /// Maps the contained data while preserving code and message.
    pub fn map<U>(self, mapper: impl FnOnce(T) -> U) -> ApiResponse<U> {
        ApiResponse {
            code: self.code,
            msg: self.msg,
            data: self.data.map(mapper),
        }
    }
}

impl ApiResponse<()> {
    /// Creates a successful response without data.
    pub fn success_empty() -> Self {
        Self {
            code: 0,
            msg: "success".to_owned(),
            data: None,
        }
    }
}
