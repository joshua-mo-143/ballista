use anyhow::Result;

use openai::{
    chat::{ChatCompletionBuilder, ChatCompletionMessage},
    embeddings::{Embedding, Embeddings},
};

use crate::files::File;
use crate::{llm::Conversation, EmbeddingsResult, LLMBackend, PromptBackend};

use std::env;

#[derive(Clone)]
pub struct OpenAIBackend;

#[async_trait::async_trait]
impl PromptBackend for OpenAIBackend {
    async fn chat_stream(&self, prompt: &str, contents: &str) -> Result<Conversation> {
        let content = format!("{}\n Context: {}\n Be concise", prompt, contents);

        let stream = ChatCompletionBuilder::default()
            .model("gpt-3.5-turbo")
            .temperature(0.0)
            .user("josh")
            .messages(vec![ChatCompletionMessage {
                role: openai::chat::ChatCompletionMessageRole::User,
                content: Some(content),
                name: Some("josh".to_string()),
                function_call: None,
            }])
            .create_stream()
            .await?;

        Ok(stream)
    }
}

#[async_trait::async_trait]
impl LLMBackend for OpenAIBackend {
    fn new() -> Result<Self> {
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
