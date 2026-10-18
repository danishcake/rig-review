use std::{path::PathBuf, sync::Arc};

use gix::Repository;
use gix::object::tree::diff::ChangeDetached;
use gix::utils::AsBStr;
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{
    error::RigReviewError, source::base::ReviewSource, util::gix_utils::ChangeDetachedUtils,
};

/// A review source that is backed by a local git repository
pub struct LocalGitReviewSource {
    repository: Arc<Mutex<Repository>>,
    changes: Vec<ChangeDetached>,
}

impl LocalGitReviewSource {
    pub fn new(
        folder: PathBuf,
        base_commit: &str,
        modified_commit: &str,
    ) -> Result<Self, RigReviewError> {
        tracing::info!(
            "Source: {} {base_commit}..{modified_commit}",
            folder.display()
        );

        let repository = gix::open(folder)?;

        // Extract the changes
        let changes = {
            // Get the two commits
            let base_commit = repository
                .rev_parse_single(base_commit)?
                .object()?
                .try_into_commit()?;
            let modified_commit = repository
                .rev_parse_single(modified_commit)?
                .object()?
                .try_into_commit()?;

            let base_tree = base_commit.tree()?;
            let modified_tree = modified_commit.tree()?;
            repository.diff_tree_to_tree(Some(&base_tree), Some(&modified_tree), None)?
        };

        // Remove directories
        let changes = changes
            .into_iter()
            .filter(|change| !change.entry_mode().is_tree())
            .collect();

        Ok(LocalGitReviewSource {
            repository: Arc::new(Mutex::new(repository)),
            changes,
        })
    }

    /// Finds the change corresponding to a file
    fn find_change(&self, file: &str) -> Result<&ChangeDetached, RigReviewError> {
        self.changes
            .iter()
            .find(|change| change.location() == file.as_bstr())
            .ok_or_else(|| RigReviewError::FileNotChanged(file.to_owned()))
    }
}

#[async_trait::async_trait]
impl ReviewSource for LocalGitReviewSource {
    async fn changed_files(&self) -> Result<Vec<String>, RigReviewError> {
        let changes = self.changes.clone();

        spawn_blocking(move || {
            // Find all changed files
            Ok(changes
                .iter()
                .map(|change| change.location().to_string())
                .collect())
        })
        .await
        .expect("failed to collect changed files")
    }

    async fn changes(&self, file: &str) -> Result<String, RigReviewError> {
        let change = self.find_change(file)?.clone();
        let repository = self.repository.clone().lock_owned().await;

        spawn_blocking(move || change.get_diff(&repository))
            .await
            .expect("failed to compute file diff")
    }

    async fn original_file(&self, file: &str) -> Result<String, RigReviewError> {
        let change = self.find_change(file)?.clone();
        let repository = self.repository.clone().lock_owned().await;

        spawn_blocking(move || {
            let contents = change.get_initial_contents(&repository)?;
            Ok(contents
                .map(|data| String::from_utf8_lossy(&data).into_owned())
                .unwrap_or(String::new()))
        })
        .await
        .expect("failed to read original file")
    }

    async fn modified_file(&self, file: &str) -> Result<String, RigReviewError> {
        let change = self.find_change(file)?.clone();
        let repository = self.repository.clone().lock_owned().await;

        spawn_blocking(move || {
            let contents = change.get_modified_contents(&repository)?;
            Ok(contents
                .map(|data| String::from_utf8_lossy(&data).into_owned())
                .unwrap_or(String::new()))
        })
        .await
        .expect("failed to read modified file")
    }
}
