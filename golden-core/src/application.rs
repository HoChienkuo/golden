use crate::ApplicationError;
use axum::Router;
use axum::routing::get;
use std::net::SocketAddr;

pub const DEFAULT_PORT: u16 = 8080;

pub fn run<I>(
    port: u16,
    initializer: I,
) -> Result<(), ApplicationError>
where
    I: Future<Output = ()>,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ApplicationError::Runtime)?;

    runtime.block_on(async move {
        initializer.await;
        serve(port).await
    })
}

async fn serve(port: u16) -> Result<(), ApplicationError> {
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let router = create_router();

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(ApplicationError::Bind)?;

    println!("GoldenBoot started");
    println!("Listening on http://{address}");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(ApplicationError::Serve)
}

fn create_router() -> Router {
    Router::new().route("/", get(index))
}

async fn index() -> &'static str {
    "Hello from GoldenBoot!"
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        shutdown_signal_unix().await;
    }

    #[cfg(not(unix))]
    {
        shutdown_signal_ctrl_c().await;
    }
}

async fn shutdown_signal_ctrl_c() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("failed to listen for Ctrl+C: {error}");
    }
}

#[cfg(unix)]
async fn shutdown_signal_unix() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = match signal(SignalKind::terminate()) {
        Ok(signal) => signal,
        Err(error) => {
            eprintln!("failed to listen for SIGTERM: {error}");
            shutdown_signal_ctrl_c().await;
            return;
        }
    };

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                eprintln!("failed to listen for Ctrl+C: {error}");
            }
        }

        _ = terminate.recv() => {}
    }
}
