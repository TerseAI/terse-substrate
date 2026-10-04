use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;

#[async_trait]
pub trait Credentials: Send + Sync {
    async fn token(&self) -> Result<String>;
}

/// Reads the current token for every RPC, including projected Kubernetes tokens.
pub struct FileCredentials {
    path: PathBuf,
}

impl FileCredentials {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl Credentials for FileCredentials {
    async fn token(&self) -> Result<String> {
        tokio::fs::read_to_string(&self.path)
            .await
            .context("read Substrate bearer token")
    }
}
