pub mod files;
pub mod github;
pub mod llm;
pub mod qdrant;
pub mod routes;
pub mod state;

use llm::{Conversation, EmbeddingsResult, LLMBackend, PromptBackend};

use anyhow::Result;
use files::{File, Finder};
use qdrant::VectorDB;
use state::AppState;

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

async fn get_contents<T: LLMBackend + PromptBackend>(
    prompt: &str,
    state: &AppState<T>,
) -> Result<Conversation> {
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
    state.llm.chat_stream(prompt, contents.as_str()).await
}
