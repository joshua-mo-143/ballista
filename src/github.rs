use anyhow::Result;
use flate2::read::GzDecoder;
use http_body_util::BodyExt;
use octocrab::{models::repos::RepoCommit, repos::RepoHandler, Octocrab, OctocrabBuilder};

use std::env;
use std::io::{Cursor, Read};
use tempfile::TempDir;

use tokio_tar::Archive;

#[derive(Clone)]
pub struct Octo {
    crab: Octocrab,
    user: String,
    repo: String,
}

impl Octo {
    pub fn new() -> Result<Self> {
        let pat = env::var("GITHUB_PERSONAL_ACCESS_TOKEN")?;
        let crab = OctocrabBuilder::new().personal_token(pat).build()?;

        let user = env::var("GITHUB_USERNAME")?;
        let repo = env::var("GITHUB_REPO")?;

        Ok(Self { crab, user, repo })
    }

    pub fn get_repo(&self) -> Result<RepoHandler<'_>> {
        Ok(self.crab.repos(&self.user, &self.repo))
    }

    pub async fn get_latest_commit_from_repo(&self) -> Result<Option<RepoCommit>> {
        let repo = self.get_repo()?;

        let res = repo.list_commits().send().await?;

        Ok(res.items.into_iter().next())
    }

    pub async fn download_repo(&self, dir: &TempDir) -> Result<()> {
        let repo = self.get_repo()?;
        let Some(commit) = self.get_latest_commit_from_repo().await? else {
            return Err(anyhow::anyhow!("Could not find a commit from the repo :("));
        };

        let tarball = repo.download_tarball(commit.sha).await?;

        let meme = tarball.into_body().collect().await?.to_bytes();

        let mut gzip = GzDecoder::new(Cursor::new(meme));
        let mut decompressed_bytes = Vec::new();
        gzip.read_to_end(&mut decompressed_bytes)?;

        let mut ar = Archive::new(Cursor::new(decompressed_bytes));

        ar.unpack(dir.path()).await?;

        Ok(())
    }
}
