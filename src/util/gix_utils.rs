use gix::Repository;
use gix::diff::blob::platform::prepare_diff::Operation;
use gix::object::tree::diff::ChangeDetached;
use gix::prelude::TreeDiffChangeExt;

use crate::error::RigReviewError;

pub trait ChangeDetachedUtils {
    /// Gets the starting contents of the change. If there is none (e.g. new file) then
    /// None is returned
    fn get_initial_contents(
        &self,
        repository: &Repository,
    ) -> Result<Option<Vec<u8>>, RigReviewError>;

    /// Gets the modified contents of the change. If there is not (e.g. deleted file) then
    /// None is returned
    fn get_modified_contents(
        &self,
        repository: &Repository,
    ) -> Result<Option<Vec<u8>>, RigReviewError>;

    /// Gets the diff for the change
    fn get_diff(&self, repository: &Repository) -> Result<String, RigReviewError>;
}

impl ChangeDetachedUtils for ChangeDetached {
    fn get_initial_contents(
        &self,
        repository: &Repository,
    ) -> Result<Option<Vec<u8>>, RigReviewError> {
        // Determine object ID
        let object_id = match self {
            ChangeDetached::Addition { .. } => None,
            ChangeDetached::Deletion { id, .. } => Some(*id),
            ChangeDetached::Modification { previous_id, .. } => Some(*previous_id),
            ChangeDetached::Rewrite { source_id, .. } => Some(*source_id),
        };

        let Some(object_id) = object_id else {
            return Ok(None);
        };

        // Lookup contents
        let object = repository.find_object(object_id)?;
        Ok(Some(object.data.clone()))
    }

    fn get_modified_contents(
        &self,
        repository: &Repository,
    ) -> Result<Option<Vec<u8>>, RigReviewError> {
        // Determine object ID
        let object_id = match self {
            ChangeDetached::Addition { id, .. } => Some(*id),
            ChangeDetached::Deletion { .. } => None,
            ChangeDetached::Modification { id, .. } => Some(*id),
            ChangeDetached::Rewrite { id, .. } => Some(*id),
        };

        let Some(object_id) = object_id else {
            return Ok(None);
        };

        // Lookup contents
        let object = repository.find_object(object_id)?;
        Ok(Some(object.data.clone()))
    }

    fn get_diff(&self, repository: &Repository) -> Result<String, RigReviewError> {
        let path = self.location().to_string();

        let mut cache = repository.diff_resource_cache_for_tree_diff()?;
        let platform = self.attach(repository, repository).diff(&mut cache)?;
        let prep = platform
            .resource_cache
            .prepare_diff()
            .map_err(gix::Error::from)?;

        let body = match prep.operation {
            Operation::InternalDiff { algorithm } => {
                let input = prep.interned_input();
                let diff = gix::diff::blob::diff_with_slider_heuristics(algorithm, &input);
                gix::diff::blob::UnifiedDiff::new(
                    &diff,
                    &input,
                    gix::diff::blob::unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
                    gix::diff::blob::unified_diff::ContextSize::symmetrical(3),
                )
                .consume()?
            }
            Operation::SourceOrDestinationIsBinary => {
                format!("Binary files a/{path} and b/{path} differ\n")
            }
            Operation::ExternalCommand { .. } => {
                return Err(RigReviewError::Io(std::io::Error::other(
                    "external diff drivers are not supported",
                )));
            }
        };

        Ok(format!("--- a/{path}\n+++ b/{path}\n{body}"))
    }
}
