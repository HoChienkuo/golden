use golden_boot::__private::{
    axum::body::{Body, to_bytes},
    create_router,
};
use golden_boot::{ApplicationError, Path, State, get_mapping};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

#[derive(Clone)]
struct AppState {
    application_name: &'static str,
    counter: Arc<AtomicUsize>,
}

#[get_mapping("/state")]
async fn state_handler(State(state): State<AppState>) -> &'static str {
    state.application_name
}

#[get_mapping("/counter/increment")]
async fn increment_counter(State(state): State<AppState>) -> String {
    (state.counter.fetch_add(1, Ordering::SeqCst) + 1).to_string()
}

#[get_mapping("/counter/value")]
async fn read_counter(State(state): State<AppState>) -> String {
    state.counter.load(Ordering::SeqCst).to_string()
}

#[get_mapping("/state/{id}")]
async fn state_with_path(State(state): State<AppState>, Path(id): Path<u64>) -> String {
    format!("{}-{id}", state.application_name)
}

#[get_mapping("/health")]
async fn stateless_handler() -> &'static str {
    "ok"
}

fn app_state() -> AppState {
    AppState {
        application_name: "golden",
        counter: Arc::new(AtomicUsize::new(0)),
    }
}

async fn get(router: &golden_boot::__private::axum::Router, uri: &str) -> (u16, Vec<u8>) {
    let response = router
        .clone()
        .oneshot(
            golden_boot::Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status().as_u16();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();

    (status, body)
}

#[tokio::test]
async fn automatically_registers_a_handler_with_application_state() {
    let router = create_router(&app_state()).unwrap();

    assert_eq!(get(&router, "/state").await, (200, b"golden".to_vec()));
    assert_eq!(get(&router, "/health").await, (200, b"ok".to_vec()));
    assert_eq!(
        get(&router, "/state/42").await,
        (200, b"golden-42".to_vec()),
    );
    assert_eq!(
        get(&router, "/counter/increment").await,
        (200, b"1".to_vec()),
    );
    assert_eq!(get(&router, "/counter/value").await, (200, b"1".to_vec()),);
}

#[test]
fn rejects_an_application_state_with_the_wrong_type() {
    let error = create_router(&()).unwrap_err();

    assert!(matches!(error, ApplicationError::StateTypeMismatch { .. }));
    assert!(error.to_string().contains("AppState"));
}
