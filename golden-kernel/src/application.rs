use crate::{
    ApplicationError,
    routing::{create_router, validate_registered_routes},
};
use axum::Router;
use std::{any::Any, convert::Infallible, error::Error, future::Future, net::SocketAddr};

pub const DEFAULT_PORT: u16 = 8080;

pub fn run<I, S>(port: u16, initializer: I) -> Result<(), ApplicationError>
where
    I: Future<Output = S>,
    S: Any + Clone + Send + Sync + 'static,
{
    run_fallible(port, async move { Ok::<S, Infallible>(initializer.await) })
}

pub fn run_fallible<I, S, E>(port: u16, initializer: I) -> Result<(), ApplicationError>
where
    I: Future<Output = Result<S, E>>,
    S: Any + Clone + Send + Sync + 'static,
    E: Error + Send + Sync + 'static,
{
    validate_registered_routes()?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ApplicationError::Runtime)?;

    runtime.block_on(async move {
        let state = initializer
            .await
            .map_err(|source| ApplicationError::Initialization {
                source: Box::new(source),
            })?;

        let router = create_router(&state)?;
        serve(port, router).await
    })
}

async fn serve(port: u16, router: Router) -> Result<(), ApplicationError> {
    let address = SocketAddr::from(([0, 0, 0, 0], port));

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
