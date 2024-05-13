use crate::llm::Embeddable;
use anyhow::Result;

use qdrant_client::prelude::{
    CreateCollection, Distance, Payload, PointStruct, QdrantClient, QdrantClientConfig,
};
use qdrant_client::qdrant::{
    vectors_config::Config, with_payload_selector::SelectorOptions, ScoredPoint, SearchPoints,
    VectorParams, VectorsConfig, WithPayloadSelector,
};
use serde_json::json;
use std::sync::Arc;

use crate::files::File;
use std::env;

static COLLECTION: &str = "brain";

#[derive(Clone)]
pub struct VectorDB {
    client: Arc<QdrantClient>,
    id: u64,
}

impl VectorDB {
    pub fn new() -> Result<Self> {
        let qdrant_url = env::var("QDRANT_URL").unwrap_or_else(|_| {
            println!("No QDRANT_URL env var found! Defaulting to localhost:6334...");
            "http://localhost:6334".to_string()
        });
        let qdrant_api_key = env::var("QDRANT_API_KEY");

        let cfg = QdrantClientConfig::from_url(&qdrant_url).with_api_key(qdrant_api_key);
        let client = QdrantClient::new(Some(cfg))?;

        Ok(Self {
            client: Arc::new(client),
            id: 0,
        })
    }

    pub fn from_qdrant_client(client: QdrantClient) -> Self {
        Self {
            client: Arc::new(client),
            id: 0,
        }
    }

    pub async fn reset_collection(&self) -> Result<()> {
        self.client.delete_collection(COLLECTION).await?;

        self.client
            .create_collection(&CreateCollection {
                collection_name: COLLECTION.to_string(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
                        size: 1536,
                        distance: Distance::Cosine.into(),
                        hnsw_config: None,
                        quantization_config: None,
                        on_disk: None,
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    pub async fn upsert_embedding<T: Embeddable>(
        &mut self,
        embedding: T,
        file: &File,
    ) -> Result<()> {
        let payload: Payload = json!({
            "id": file.path.clone(),
        })
        .try_into()
        .unwrap();

        println!("Embedded: {}", file.path);

        let vec = embedding.into_vec_f32();

        let points = vec![PointStruct::new(self.id, vec, payload)];
        self.client
            .upsert_points(COLLECTION, None, points, None)
            .await?;
        self.id += 1;

        Ok(())
    }

    pub async fn search(&self, embedding: Vec<f32>) -> Result<ScoredPoint> {
        let payload_selector = WithPayloadSelector {
            selector_options: Some(SelectorOptions::Enable(true)),
        };

        let search_points = SearchPoints {
            collection_name: COLLECTION.to_string(),
            vector: embedding,
            limit: 1,
            with_payload: Some(payload_selector),
            ..Default::default()
        };

        let search_result = self.client.search_points(&search_points).await?;
        let result = search_result.result[0].clone();
        Ok(result)
    }
}
