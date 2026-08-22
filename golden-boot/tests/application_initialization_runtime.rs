use golden_boot::{__private::run_fallible, ApplicationError};

#[derive(Debug, thiserror::Error)]
#[error("database is unavailable")]
struct StartupError;

#[test]
fn preserves_the_application_initialization_error() {
    let error = run_fallible(8080, async { Err::<(), _>(StartupError) }).unwrap_err();

    match error {
        ApplicationError::Initialization { source } => {
            assert!(source.is::<StartupError>());
            assert_eq!(source.to_string(), "database is unavailable");
        }
        other => panic!("expected initialization error, got {other}"),
    }
}
