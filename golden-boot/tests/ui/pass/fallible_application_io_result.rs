use golden_boot::golden_boot_application;

#[derive(Clone)]
struct AppState;

#[golden_boot_application]
async fn main() -> std::io::Result<AppState> {
    if std::hint::black_box(true) {
        std::process::exit(0);
    }

    Ok(AppState)
}
