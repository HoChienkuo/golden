use golden_boot::golden_boot_application;

struct AppState;

#[golden_boot_application]
async fn main() -> AppState {
    AppState
}
