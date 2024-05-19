use axum::{routing::post, Router};

use ballista::llm::{open_ai::OpenAIBackend, LLMBackend};
use ballista::routes::prompt::prompt;
use ballista::routes::webhooks::handle_github_webhook;

use ballista::state::AppState;

use ballista::qdrant::VectorDB;
use std::env;

use std::sync::Arc;
use tower_http::services::ServeDir;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_qdrant::Qdrant(
        cloud_url = "{secrets.QDRANT_URL}",
        api_key = "{secrets.QDRANT_API_KEY}"
    )]
    qdrant: qdrant_client::prelude::QdrantClient,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_axum::ShuttleAxum {
    secrets.into_iter().for_each(|x| {
        if x.1 != String::new() {
            env::set_var(x.0, x.1);
        }
    });
    println!("Secrets found!");

    let vector_db = VectorDB::from_qdrant_client(qdrant);
    println!("VectorDB created!");

    let llm_backend = OpenAIBackend::new()?;
    println!("OpenAI backend created!");

    let state = AppState::new(vector_db, llm_backend)?;

    let state = Arc::new(state);

    let cloned_state: Arc<AppState<OpenAIBackend>> = Arc::clone(&state);

    tokio::spawn(async move {
        cloned_state.run_update_queue().await;
    });

    state.notify.notify_one();

    let rtr = Router::new()
        .route("/prompt", post(prompt))
        .route("/webhooks/github", post(handle_github_webhook))
        .nest_service("/", ServeDir::new("static"))
        .with_state(state);

    Ok(rtr.into())
}
