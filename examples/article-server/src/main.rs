mod article;
mod error;
mod state;

use crate::error::StartupError;
use crate::state::AppState;
use golden_boot::golden_boot_application;

#[golden_boot_application(port = 9090)]
async fn main() -> Result<AppState, StartupError> {
    let data_directory =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data");
    std::fs::create_dir_all(&data_directory)?;

    let state = AppState::initialize().await?;

    Ok(state)
}
