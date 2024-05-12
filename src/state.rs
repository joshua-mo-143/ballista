use anyhow::Result;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::sync::Notify;

use crate::{embed_documentation, files::load_files_from_dir, github::Octo, qdrant::VectorDB};

use crate::files::File;

use crate::open_ai::LLMBackend;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState<T: LLMBackend> {
    pub files: Arc<RwLock<Vec<File>>>,
    pub notify: Arc<Notify>,
    pub db: VectorDB,
    pub octo: Octo,
    pub llm: T,
}

impl<T: LLMBackend> AppState<T> {
    pub fn new(db: VectorDB, octo: Octo, llm: T) -> Result<Self> {
        Ok(Self {
            files: Arc::new(RwLock::new(Vec::new())),
            notify: Arc::new(Notify::new()),
            db,
            octo,
            llm,
        })
    }

    pub async fn update(&self) -> Result<()> {
        let temp_dir = tempdir()?;
        self.octo.download_repo(&temp_dir).await?;

        let mut files =
            load_files_from_dir(temp_dir.path().to_path_buf(), "md", &PathBuf::from(""))?;

        let mut db = VectorDB::new()?;

        db.reset_collection().await?;
        embed_documentation(&mut files, &mut db, &self.llm).await?;

        let mut lock = self.files.write().await;
        *lock = files;

        Ok(())
    }

    pub async fn run_update_queue(&self) {
        loop {
            self.notify.notified().await;

            let _ = self
                .update()
                .await
                .inspect_err(|x| println!("Error while updating application state: {x}"));
        }
    }
}

pub struct AppStateBuilder<T: LLMBackend> {
    pub db: Option<VectorDB>,
    pub octo: Option<Octo>,
    pub llm: Option<T>,
}

impl<T: LLMBackend> Default for AppStateBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: LLMBackend> AppStateBuilder<T> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn with_qdrant_client(mut self, db: VectorDB) -> Self {
        self.db = Some(db);

        self
    }

    pub fn with_octo(mut self, octo: Octo) -> Self {
        self.octo = Some(octo);

        self
    }

    pub fn with_llm(mut self, llm: T) -> Self {
        self.llm = Some(llm);

        self
    }

    pub fn build(self) -> Result<AppState<T>> {
        let db = match self.db {
            Some(db) => db,
            None => VectorDB::new()?,
        };

        let octo = match self.octo {
            Some(db) => db,
            None => Octo::new()?,
        };

        let llm = match self.llm {
            Some(llm) => llm,
            None => return Err(anyhow::anyhow!("Couldn't find an LLM")),
        };

        AppState::new(db, octo, llm)
    }
}
