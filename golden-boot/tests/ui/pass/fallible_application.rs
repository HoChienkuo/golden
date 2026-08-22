use golden_boot::golden_boot_application;

#[derive(Clone)]
struct AppState;

#[derive(Debug, thiserror::Error)]
#[error("startup failed")]
struct StartupError;

async fn initialize_dependency() -> Result<(), StartupError> {
    Ok(())
}

#[golden_boot_application]
async fn main() -> Result<AppState, StartupError> {
    if std::hint::black_box(true) {
        std::process::exit(0);
    }

    initialize_dependency().await?;
    Ok(AppState)
}
