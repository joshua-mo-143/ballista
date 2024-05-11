use anyhow::Result;
use axum::{routing::post, Router};

use ballista::routes::prompt::prompt;
use ballista::routes::webhooks::handle_github_webhook;

use ballista::open_ai;
use ballista::state::{AppState, AppStateBuilder};
use tokio::net::TcpListener;

use std::sync::Arc;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<()> {
    open_ai::setup().expect("Set up OpenAI key");

    let state = AppStateBuilder::new().build()?;
    let state = Arc::new(state);

    let cloned_state: Arc<AppState> = Arc::clone(&state);

    tokio::spawn(async move {
        cloned_state.run_update_queue().await;
    });

    state.update().await?;

    let rtr = Router::new()
        .route("/prompt", post(prompt))
        .route("/webhooks/github", post(handle_github_webhook))
        .nest_service("/", ServeDir::new("static"))
        .with_state(state);

    let tcp = TcpListener::bind("0.0.0.0:8000").await.unwrap();

    println!("Starting up server...");
    axum::serve(tcp, rtr).await.unwrap();

    Ok(())
}
