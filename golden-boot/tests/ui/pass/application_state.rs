use golden_boot::{State, get_mapping, golden_boot_application};

#[derive(Clone)]
struct AppState {
    name: &'static str,
}

#[get_mapping("/state")]
async fn state_handler(State(state): State<AppState>) -> &'static str {
    state.name
}

#[golden_boot_application]
async fn main() -> AppState {
    if std::hint::black_box(true) {
        std::process::exit(0);
    }

    AppState { name: "golden" }
}
