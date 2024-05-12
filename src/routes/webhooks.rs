use axum::{
    body::to_bytes,
    extract::{FromRequest, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use octocrab::models::{
    orgs::Organization,
    webhook_events::{EventInstallation, WebhookEventPayload, WebhookEventType},
    Author, Repository,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{open_ai::LLMBackend, state::AppState};

#[derive(Deserialize)]
pub struct WebhookEvent {
    pub sender: Option<Author>,
    pub repository: Option<Repository>,
    pub organization: Option<Organization>,
    pub installation: Option<EventInstallation>,
    pub kind: WebhookEventType,
    pub specific: WebhookEventPayload,
}

pub struct GithubEvent(WebhookEvent);

#[axum::async_trait]
impl<T: LLMBackend> FromRequest<Arc<AppState<T>>> for GithubEvent {
    type Rejection = Response;
    async fn from_request(
        req: Request,
        _state: &Arc<AppState<T>>,
    ) -> Result<Self, Self::Rejection> {
        let Some(_event_type) = req.headers().get("X-Github-Event") else {
            return Err((StatusCode::BAD_REQUEST, "Incorrect headers").into_response());
        };

        let Ok(body) = to_bytes(req.into_body(), usize::MAX).await else {
            return Err(
                (StatusCode::BAD_REQUEST, "Failed to convert body to Vec<u8>").into_response(),
            );
        };

        let Ok(event) = serde_json::from_slice::<WebhookEvent>(&body) else {
            return Err((StatusCode::BAD_REQUEST, "Failed to deserialize body :(").into_response());
        };

        Ok(Self(event))
    }
}

pub async fn handle_github_webhook<T: LLMBackend>(
    State(state): State<Arc<AppState<T>>>,
    GithubEvent(evt): GithubEvent,
) -> impl IntoResponse {
    match evt.kind {
        WebhookEventType::Push => {}
        _ => return StatusCode::BAD_REQUEST,
    }

    let WebhookEventPayload::Push(_evt) = evt.specific else {
        return StatusCode::BAD_REQUEST;
    };

    //    let Some(commit) = evt.commits.iter().next() else {
    //        return StatusCode::BAD_REQUEST;
    //    };

    state.notify.notify_one();

    StatusCode::OK
}
