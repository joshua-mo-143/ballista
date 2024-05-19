use anyhow::Result;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::sync::Notify;

use crate::{embed_documentation, files::load_files_from_dir, github::Octo, qdrant::VectorDB};

use crate::files::File;

use crate::llm::{LLMBackend, PromptBackend};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState<T: LLMBackend + PromptBackend> {
    pub files: Arc<RwLock<Vec<File>>>,
    pub notify: Arc<Notify>,
    pub db: VectorDB,
    pub octo: Octo,
    pub llm: T,
}

impl<T: LLMBackend + PromptBackend + Send + Sync> AppState<T> {
    pub fn new(db: VectorDB, llm: T) -> Result<Self> {
        Ok(Self {
            files: Arc::new(RwLock::new(Vec::new())),
            notify: Arc::new(Notify::new()),
            db,
            octo: Octo::new()?,
            llm,
        })
    }

    pub async fn update(&self) -> Result<()> {
        let temp_dir = tempdir()?;
        let path = self.octo.download_repo(&temp_dir).await?;

        let mut files = load_files_from_dir(temp_dir.path().to_path_buf(), "md", &path)?;

        let mut db = VectorDB::new()?;

        db.reset_collection().await?;
        embed_documentation(&mut files, &mut db, &self.llm).await?;

        let mut lock = self.files.write().await;
        *lock = files;

        println!("All files have been embedded!");

        Ok(())
    }

    pub async fn run_update_queue(&self) {
        loop {
            self.notify.notified().await;

            let _ = self
                .update()
                .await
                .inspect_err(|x| println!("Error while updating application state: {x}"))
                .unwrap();
        }
    }
}
