pub mod files;
pub mod github;
pub mod open_ai;
pub mod qdrant;
pub mod routes;
pub mod state;

use open_ai::LLMBackend;
use tokio::sync::mpsc::Receiver;

use anyhow::Result;
use files::{File, Finder};
use openai::chat::ChatCompletionDelta;
use qdrant::VectorDB;
use state::AppState;

use crate::open_ai::EmbeddingsResult;

async fn embed_documentation<T: LLMBackend>(
    files: &mut Vec<File>,
    vector_db: &mut VectorDB,
    llm: &T,
) -> Result<()> {
    for file in files {
        let embeddings = llm.embed_file(file).await?;
        println!("Embedding: {:?}", file.path);
        let EmbeddingsResult::OpenAIEmbeddings(embeddings) = embeddings else {
            return Err(anyhow::anyhow!("Embeddings were the wrong enum variant :("));
        };
        for embedding in embeddings.data {
            vector_db.upsert_embedding(embedding, file).await?;
        }
    }

    Ok(())
}

async fn get_contents<T: LLMBackend>(
    prompt: &str,
    state: &AppState<T>,
) -> Result<Receiver<ChatCompletionDelta>> {
    let embedding = state.llm.embed_sentence(prompt).await?;
    let EmbeddingsResult::OpenAIEmbedding(embedding) = embedding else {
        return Err(anyhow::anyhow!("Embedding was the wrong enum variant :("));
    };
    let result = state.db.search(embedding).await?;
    let contents = state
        .files
        .read()
        .await
        .get_contents(&result)
        .ok_or(anyhow::anyhow!("There was a prompt error :("))?;
    open_ai::chat_stream(prompt, contents.as_str()).await
}
