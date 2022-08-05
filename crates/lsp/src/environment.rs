use std::{path::{Path, PathBuf}, time::Duration};

use async_trait::async_trait;
use futures::Future;
use lsp_types::Url;

pub mod native;

#[async_trait(?Send)]
pub trait Environment: Clone + Send + Sync + 'static {
    fn spawn<F>(&self, fut: F)
    where
        F: Future + Send + 'static,
        F::Output: Send;

    fn spawn_local<F>(&self, fut: F)
    where
        F: Future + 'static;

    fn env_var(&self, name: &str) -> Option<String>;

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, anyhow::Error>;

    fn url_to_file_path(&self, url: &Url) -> Option<PathBuf>;

    /// Absolute current working dir.
    fn cwd(&self) -> Option<PathBuf>;

    fn glob_files(&self, glob: &str) -> Result<Vec<PathBuf>, anyhow::Error>;

    fn is_absolute(&self, path: &Path) -> bool;

    fn discover_rhai_config(&self, root: &Path) -> Option<PathBuf>;

    fn is_dir(&self, root: &Path) -> bool;

    async fn sleep(&self, duration: Duration);
}
