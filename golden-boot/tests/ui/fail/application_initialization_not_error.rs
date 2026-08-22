use golden_boot::golden_boot_application;

#[derive(Clone)]
struct AppState;

struct StartupError;

#[golden_boot_application]
async fn main() -> Result<AppState, StartupError> {
    Err(StartupError)
}
