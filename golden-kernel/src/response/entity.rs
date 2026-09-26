use crate::response::error::ResponseEntityError;
use axum::{
    Json,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// An HTTP response containing a status code, headers, and an optional body.
#[derive(Debug)]
pub struct ResponseEntity<T = ()> {
    status: StatusCode,
    headers: HeaderMap,
    body: Option<T>,
}

/// A builder for constructing a [`ResponseEntity`].
#[derive(Debug)]
pub struct ResponseEntityBuilder {
    status: StatusCode,
    headers: HeaderMap,
}

impl<T> ResponseEntity<T> {
    /// Creates a response with `200 OK`.
    pub fn ok(body: T) -> Self {
        Self::new(StatusCode::OK, body)
    }

    /// Creates a response with `201 Created`.
    pub fn created(body: T) -> Self {
        Self::new(StatusCode::CREATED, body)
    }

    /// Creates a response with `202 Accepted`.
    pub fn accepted(body: T) -> Self {
        Self::new(StatusCode::ACCEPTED, body)
    }

    /// Creates a response with `400 Bad Request`.
    pub fn bad_request(body: T) -> Self {
        Self::new(StatusCode::BAD_REQUEST, body)
    }

    /// Creates a response with `404 Not Found`.
    pub fn not_found(body: T) -> Self {
        Self::new(StatusCode::NOT_FOUND, body)
    }

    /// Creates a response with `500 Internal Server Error`.
    pub fn internal_server_error(body: T) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, body)
    }

    /// Creates a response containing the given status and body.
    pub fn new(status: StatusCode, body: T) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Some(body),
        }
    }

    /// Adds a prevalidated header.
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Parses and adds a header.
    pub fn try_with_header(
        mut self,
        name: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Self, ResponseEntityError> {
        let name = HeaderName::from_bytes(name.as_ref().as_bytes())?;

        let value = HeaderValue::from_str(value.as_ref())?;

        self.headers.insert(name, value);

        Ok(self)
    }

    /// Adds all headers from the provided map.
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Returns the response status.
    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    /// Returns the response headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns a mutable reference to the response headers.
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Returns a reference to the response body.
    pub fn body_ref(&self) -> Option<&T> {
        self.body.as_ref()
    }

    /// Consumes the response and returns its body.
    pub fn into_body(self) -> Option<T> {
        self.body
    }
}

impl ResponseEntity<()> {
    /// Starts building a response with the given status.
    pub fn status(status: StatusCode) -> ResponseEntityBuilder {
        ResponseEntityBuilder {
            status,
            headers: HeaderMap::new(),
        }
    }

    /// Creates a response with `204 No Content`.
    pub fn no_content() -> Self {
        Self {
            status: StatusCode::NO_CONTENT,
            headers: HeaderMap::new(),
            body: None,
        }
    }
}

impl ResponseEntityBuilder {
    /// Adds a prevalidated header.
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Parses and adds a header.
    pub fn try_with_header(
        mut self,
        name: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Self, ResponseEntityError> {
        let name = HeaderName::from_bytes(name.as_ref().as_bytes())?;

        let value = HeaderValue::from_str(value.as_ref())?;

        self.headers.insert(name, value);

        Ok(self)
    }

    /// Adds all headers from the provided map.
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Completes the response with a body.
    pub fn body<T>(self, body: T) -> ResponseEntity<T> {
        ResponseEntity {
            status: self.status,
            headers: self.headers,
            body: Some(body),
        }
    }

    /// Completes the response without a body.
    pub fn empty(self) -> ResponseEntity<()> {
        ResponseEntity {
            status: self.status,
            headers: self.headers,
            body: None,
        }
    }
}

impl<T> IntoResponse for ResponseEntity<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut response = match self.body {
            Some(body) => Json(body).into_response(),
            None => ().into_response(),
        };

        *response.status_mut() = self.status;
        response.headers_mut().extend(self.headers);

        response
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::{HeaderName, HeaderValue, StatusCode},
        response::IntoResponse,
    };

    use serde::Serialize;

    use super::ResponseEntity;

    #[derive(Serialize)]
    struct TestBody {
        message: &'static str,
    }

    #[tokio::test]
    async fn creates_ok_json_response() {
        let response = ResponseEntity::ok(TestBody { message: "hello" }).into_response();

        assert_eq!(response.status(), StatusCode::OK,);

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json",
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        assert_eq!(body, r#"{"message":"hello"}"#,);
    }

    #[tokio::test]
    async fn creates_no_content_response() {
        let response = ResponseEntity::no_content().into_response();

        assert_eq!(response.status(), StatusCode::NO_CONTENT,);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn adds_headers() {
        let response = ResponseEntity::ok(TestBody { message: "hello" })
            .with_header(
                HeaderName::from_static("x-test"),
                HeaderValue::from_static("value"),
            )
            .into_response();

        assert_eq!(response.headers().get("x-test").unwrap(), "value",);
    }

    #[test]
    fn rejects_invalid_header_value() {
        let result = ResponseEntity::ok(TestBody { message: "hello" })
            .try_with_header("x-test", "invalid\r\nvalue");

        assert!(result.is_err());
    }
}
