pub mod candle;
pub mod open_ai;

use crate::files::File;
use anyhow::Result;

use openai::chat::ChatCompletionDelta;
use openai::embeddings::{Embedding, Embeddings};
use tokio::sync::mpsc::Receiver;

pub type Conversation = Receiver<ChatCompletionDelta>;

pub trait Embeddable {
    fn into_vec_f32(self) -> Vec<f32>;
}

impl Embeddable for Embedding {
    fn into_vec_f32(self) -> Vec<f32> {
        self.vec.iter().map(|&x| x as f32).collect()
    }
}

impl Embeddable for Vec<f32> {
    fn into_vec_f32(self) -> Vec<f32> {
        self
    }
}

pub enum EmbeddingsResult {
    OpenAIEmbeddings(Embeddings),
    OpenAIEmbedding(Embedding),
    CandleEmbeddings(Vec<Vec<f32>>),
    CandleEmbedding(Vec<Vec<f32>>),
}

#[async_trait::async_trait]
pub trait LLMBackend {
    fn new() -> Result<Self>
    where
        Self: Sized;
    async fn embed_file(&self, file: &File) -> Result<EmbeddingsResult>;
    async fn embed_sentence(&self, prompt: &str) -> Result<EmbeddingsResult>;
}

#[async_trait::async_trait]
pub trait PromptBackend {
    async fn chat_stream(&self, prompt: &str, contents: &str) -> Result<Conversation>;
}
