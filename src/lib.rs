pub mod files;
pub mod github;
pub mod open_ai;
pub mod qdrant;
pub mod routes;
pub mod state;

use tokio::sync::mpsc::Receiver;

use anyhow::Result;
use files::{File, Finder};
use openai::chat::ChatCompletionDelta;
use qdrant::VectorDB;
use state::AppState;

async fn embed_documentation(files: &mut Vec<File>, vector_db: &mut VectorDB) -> Result<()> {
    for file in files {
        let embeddings = open_ai::embed_file(file).await?;
        println!("Embedding: {:?}", file.path);
        for embedding in embeddings.data {
            vector_db.upsert_embedding(embedding, file).await?;
        }
    }

    Ok(())
}

async fn get_contents(prompt: &str, state: &AppState) -> Result<Receiver<ChatCompletionDelta>> {
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
