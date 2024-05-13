pub mod files;
pub mod github;
pub mod llm;
pub mod qdrant;
pub mod routes;
pub mod state;

use llm::{Conversation, Embeddable, EmbeddingsResult, LLMBackend, PromptBackend};
use std::sync::Arc;

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

        match embeddings {
            EmbeddingsResult::OpenAIEmbeddings(embeddings) => {
                for embedding in embeddings.data {
                    vector_db.upsert_embedding(embedding, file).await?;
                }
            }
            EmbeddingsResult::CandleEmbeddings(embeddings) => {
                for embedding in embeddings {
                    vector_db.upsert_embedding(embedding, file).await?;
                }
            }
            _ => { return Err(anyhow::anyhow!("Embeddings were the wrong enum variant. Embeddings from files to be embedded should be used here.")) }
        }
    }

    Ok(())
}

async fn get_contents<'a, T: LLMBackend + PromptBackend>(
    prompt: &str,
    state: &Arc<AppState<T>>,
) -> Result<Conversation> {
    let embedding = state.llm.embed_sentence(prompt).await?;
    let embedding = match embedding {
        EmbeddingsResult::OpenAIEmbedding(embedding) => embedding.into_vec_f32(),
        EmbeddingsResult::CandleEmbedding(embedding) => embedding.into_iter().next().unwrap(),
        _ => return Err(anyhow::anyhow!("Error!")),
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
