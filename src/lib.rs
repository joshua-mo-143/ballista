pub mod files;
pub mod github;
pub mod open_ai;
pub mod qdrant;
pub mod routes;
pub mod state;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};

use tokio::{net::TcpListener, sync::mpsc::Receiver};

use qdrant::VectorDB;

use routes::prompt::prompt;
use routes::webhooks::handle_github_webhook;

use files::{File, Finder};

use state::AppState;

use openai::chat::ChatCompletionDelta;
use std::sync::Arc;
use tower_http::services::ServeDir;

async fn embed_documentation(
    files: &mut Vec<File>,
    vector_db: &mut VectorDB,
) -> anyhow::Result<()> {
    for file in files {
        let embeddings = open_ai::embed_file(file).await?;
        println!("Embedding: {:?}", file.path);
        for embedding in embeddings.data {
            vector_db.upsert_embedding(embedding, file).await?;
        }
    }

    Ok(())
}

async fn get_contents(
    prompt: &str,
    state: &AppState,
) -> anyhow::Result<Receiver<ChatCompletionDelta>> {
    let embedding = open_ai::embed_sentence(prompt).await?;
    let result = state.db.search(embedding).await?;
    let contents = state
        .files
        .read()
        .await
        .get_contents(&result)
        .ok_or(anyhow::anyhow!("There was a prompt error :("))?;
    open_ai::chat_stream(prompt, contents.as_str()).await
}
