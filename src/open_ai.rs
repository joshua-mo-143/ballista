use anyhow::Result;

use openai::{
    chat::{ChatCompletionBuilder, ChatCompletionDelta, ChatCompletionMessage},
    embeddings::{Embedding, Embeddings},
};
use tokio::sync::mpsc::Receiver;

use crate::files::File;
use std::env;
type Conversation = Receiver<ChatCompletionDelta>;

pub trait Embeddable {
    fn into_vec_f32(self) -> Vec<f32>;
}

impl Embeddable for Embedding {
    fn into_vec_f32(self) -> Vec<f32> {
        self.vec.iter().map(|&x| x as f32).collect()
    }
}

pub enum EmbeddingsResult {
    OpenAIEmbeddings(Embeddings),
    OpenAIEmbedding(Embedding),
}

#[async_trait::async_trait]
pub trait LLMBackend {
    fn new() -> Result<Self>
    where
        Self: Sized;
    async fn embed_file(&self, file: &File) -> Result<EmbeddingsResult>;
    async fn embed_sentence(&self, prompt: &str) -> Result<EmbeddingsResult>;
}

pub struct OpenAIBackend;

#[async_trait::async_trait]
impl LLMBackend for OpenAIBackend {
    fn new() -> Result<Self>
    where
        Self: Sized,
    {
        // This sets OPENAI_API_KEY as API_KEY for use with all openai crate functions
        let openai_key = env::var("OPENAI_KEY").unwrap();
        openai::set_key(openai_key);
        Ok(Self)
    }

    async fn embed_file(&self, file: &File) -> Result<EmbeddingsResult> {
        let sentence_as_str: Vec<&str> = file.sentences.iter().map(|s| s.as_str()).collect();
        println!("Embedding: {:?}", file.path);
        let embeddings = Embeddings::create("text-embedding-ada-002", sentence_as_str, "josh")
            .await
            .inspect_err(|x| println!("Failed to embed: {x:?}"))?;

        Ok(EmbeddingsResult::OpenAIEmbeddings(embeddings))
    }

    async fn embed_sentence(&self, prompt: &str) -> Result<EmbeddingsResult> {
        let embedding = Embedding::create("text-embedding-ada-002", prompt, "josh").await?;

        Ok(EmbeddingsResult::OpenAIEmbedding(embedding))
    }
}

pub async fn chat_stream(prompt: &str, contents: &str) -> Result<Conversation> {
    let content = format!("{}\n Context: {}\n Be concise", prompt, contents);

    Ok(ChatCompletionBuilder::default()
        .model("gpt-3.5-turbo")
        .temperature(0.0)
        .user("shuttle")
        .messages(vec![ChatCompletionMessage {
            role: openai::chat::ChatCompletionMessageRole::User,
            content: Some(content),
            name: Some("shuttle".to_string()),
            function_call: None,
        }])
        .create_stream()
        .await?)
}
