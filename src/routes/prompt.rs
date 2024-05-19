use axum::{extract::State, response::IntoResponse, Json};
use openai::chat::ChatCompletionDelta;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

use futures::StreamExt;
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    llm::{LLMBackend, PromptBackend},
    state::AppState,
};

use futures::stream::Stream;
#[derive(Deserialize)]
pub struct Prompt {
    prompt: String,
}

fn chat_completion_stream(
    chat_completion: Receiver<ChatCompletionDelta>,
) -> impl Stream<Item = String> {
    ReceiverStream::new(chat_completion)
        .map(|completion| completion.choices)
        .map(|choices| {
            choices
                .into_iter()
                .map(|choice| choice.delta.content.unwrap_or("\n".to_string()))
                .collect()
        })
}

fn error_stream() -> impl Stream<Item = String> {
    futures::stream::once(async move { "Error with your prompt".to_string() })
}

pub async fn prompt<T: LLMBackend + PromptBackend>(
    State(app_state): State<Arc<AppState<T>>>,
    Json(Prompt { prompt }): Json<Prompt>,
) -> impl IntoResponse {
    let chat_completion = crate::get_contents(&prompt, &app_state).await;

    match chat_completion {
        Ok(chat) => axum_streams::StreamBodyAs::text(chat_completion_stream(chat)),
        Err(e) => {
            println!("Something went wrong while prompting: {e}");
            axum_streams::StreamBodyAs::text(error_stream())
        }
    }
}
