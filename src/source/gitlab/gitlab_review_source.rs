use std::sync::Arc;

use crate::{
    config::NetworkConfig,
    error::RigReviewError,
    sink::gitlab::gitlab_review_sink::GitlabReviewSink,
    source::{base::ReviewSource, gitlab::config::GitlabReviewSourceConfig},
    util::reqwest_utils::load_certificates_as_pem_bundle,
};

use gitlab::{
    AsyncGitlab, CertificateBundle, GitlabBuilder, RestError,
    api::{
        self, ApiError, AsyncQuery,
        projects::{
            merge_requests::{MergeRequest, MergeRequestDiffs},
            repository::files::FileRaw,
        },
    },
};
use regex::Regex;
use serde::Deserialize;
use tokio::sync::{Mutex, OnceCell};
use url::Url;

/// If the scheme used is https or http
enum HttpSecurity {
    Secure,
    Insecure,
}

pub struct GitlabReviewSource {
    mr_url: String,
    project_path: String,
    mr_iid: u64,
    client: Arc<Mutex<AsyncGitlab>>,

    // Cached requests
    changes: OnceCell<Vec<FileDiff>>,
    merge_request_metadata: OnceCell<MergeRequestMetadata>,
}

impl GitlabReviewSource {
    /// Validate a merge request URL against the configured GitLab host and URL format.
    pub fn is_valid_mr_url(mr_url: &str, config: &GitlabReviewSourceConfig) -> bool {
        let Ok(parsed_url) = Url::parse(mr_url) else {
            return false;
        };
        if !parsed_url.username().is_empty()
            || parsed_url.password().is_some()
            || parsed_url.query().is_some()
            || parsed_url.fragment().is_some()
        {
            return false;
        }

        let Ok((_, _, base_path)) = Self::parse_host_and_scheme(config, mr_url) else {
            return false;
        };
        Self::parse_project_path_and_mr_iid(&base_path, mr_url).is_ok()
    }

    /// Constructor
    ///
    /// # Arguments
    /// * `mr_url` - the URL of the merge request
    /// * `config` - the configuration
    pub async fn new(
        mr_url: &str,
        config: &GitlabReviewSourceConfig,
        network_config: &NetworkConfig,
    ) -> Result<Self, RigReviewError> {
        tracing::info!("Source: {mr_url}");

        // Extract the host and scheme from the merge request URL, or config if present
        let (security, host, base_path) = Self::parse_host_and_scheme(config, mr_url)?;

        // Extract the project path and merge request iid
        let (project_path, mr_iid) = Self::parse_project_path_and_mr_iid(&base_path, mr_url)?;

        let mut client_builder = GitlabBuilder::new(host, config.api_token.clone());
        if matches!(security, HttpSecurity::Insecure) {
            client_builder.insecure();
        }
        // Staged until https://gitlab.kitware.com/utils/rust-gitlab/-/work_items/131
        // is fixed
        if !network_config.extra_ca.is_empty() {
            client_builder.cert_custom(CertificateBundle::pem_bundle(
                &load_certificates_as_pem_bundle(network_config)?,
            )?);
        }
        if network_config.ignore_certs {
            client_builder.cert_insecure();
        }

        let client = client_builder.build_async().await?;

        Ok(Self {
            mr_url: mr_url.into(),
            project_path,
            mr_iid,
            client: Arc::new(Mutex::new(client)),
            changes: OnceCell::new(),
            merge_request_metadata: OnceCell::new(),
        })
    }

    /// Obtains the changes in this merge request
    /// The result is cached, so will only be queried once
    async fn get_mr_diff(&self) -> Result<&Vec<FileDiff>, ApiError<RestError>> {
        let client = self.client.lock().await;

        self.changes
            .get_or_try_init(async || {
                let endpoint: MergeRequestDiffs<'_> = MergeRequestDiffs::builder()
                    .project(&self.project_path)
                    .merge_request(self.mr_iid)
                    .build()
                    .unwrap();

                let diffs: Result<Vec<FileDiff>, ApiError<RestError>> =
                    api::paged(endpoint, api::Pagination::All)
                        .query_async(&*client)
                        .await;
                diffs
            })
            .await
    }

    /// Get merge request metadata
    /// The result is cached so will only be queried once
    async fn get_mr_metadata(&self) -> Result<&MergeRequestMetadata, ApiError<RestError>> {
        let client = self.client.lock().await;

        self.merge_request_metadata
            .get_or_try_init(async || {
                let endpoint = MergeRequest::builder()
                    .project(&self.project_path)
                    .merge_request(self.mr_iid)
                    .build()
                    .unwrap();

                let metadata: Result<MergeRequestMetadata, ApiError<RestError>> =
                    endpoint.query_async(&*client).await;
                metadata
            })
            .await
    }

    /// Given a merge request URL, extract the host (and base path), and security level
    ///
    /// # Arguments
    /// * `config` - the configuration
    /// * `mr_url` - the URL of the merge request
    fn parse_host_and_scheme(
        config: &GitlabReviewSourceConfig,
        mr_url: &str,
    ) -> Result<(HttpSecurity, String, String), RigReviewError> {
        let mr_url = Url::parse(mr_url)?;

        // If the configuration specifies the host, simply use this. If it doesn't
        // then use the merge request URL
        let configured_host = if let Some(host) = &config.host {
            Some(Url::parse(host)?)
        } else {
            None
        };
        let url = configured_host.as_ref().unwrap_or(&mr_url);

        // Determine if http or https should be used
        let secure = match url.scheme() {
            "https" => Ok(HttpSecurity::Secure),
            "http" => Ok(HttpSecurity::Insecure),
            scheme => Err(RigReviewError::InvalidUrl(format!(
                "Unknown scheme '{scheme}'"
            ))),
        }?;

        // Extract the host, port and base path (if there is a configured host - defaults to empty)
        let host = url
            .host_str()
            .ok_or(RigReviewError::InvalidUrl("No host".into()))?
            .to_owned();
        let port = url
            .port()
            .map(|port| format!(":{port}"))
            .unwrap_or_default();
        let base_path = configured_host
            .as_ref()
            .map(|url| url.path().trim_end_matches('/').to_owned())
            .unwrap_or_default();

        // Verify that the merge request is on the comfigured host
        if configured_host.is_some()
            && (mr_url.host_str() != Some(host.as_str()) || mr_url.port() != url.port())
        {
            return Err(RigReviewError::InvalidUrl(format!(
                "url {mr_url} must be on host {url}"
            )));
        }

        // GitlabBuilder appends `/api/v4/` to this value, so retain any configured
        // reverse-proxy path prefix as well as a non-default port.
        Ok((secure, format!("{host}{port}{base_path}"), base_path))
    }

    /// Extracts the project path and merge request IID
    fn parse_project_path_and_mr_iid(
        base_path: &str,
        mr_url: &str,
    ) -> Result<(String, u64), RigReviewError> {
        let url = Url::parse(mr_url)?;
        let path = url.path();
        let relative_path = path
            .strip_prefix(base_path)
            .filter(|path| base_path.is_empty() || path.starts_with('/'))
            .ok_or(RigReviewError::MergeRequestUrlStructureIncorrect(
                path.to_owned(),
            ))?;
        let relative_path = relative_path.trim_start_matches('/');

        // The project is extracted from the mr_url which must be relative to the base URL
        // This should have produced group/subgroup/project/-/merge_requests/{id}. Let's extract that
        let re = Regex::new(r"^(?P<project>.+)/-/merge_requests/(?P<iid>\d+)/?$").unwrap();
        let caps =
            re.captures(relative_path)
                .ok_or(RigReviewError::MergeRequestUrlStructureIncorrect(
                    relative_path.to_owned(),
                ))?;

        let project = &caps["project"];
        let iid: u64 = caps["iid"].parse().unwrap();
        Ok((project.to_owned(), iid))
    }

    /// Creates a corresponding sink
    pub fn create_sink(&self) -> GitlabReviewSink {
        GitlabReviewSink::new(
            self.mr_url.clone(),
            self.project_path.clone(),
            self.mr_iid,
            self.client.clone(),
        )
    }
}

/// Changes in a merge request
/// https://docs.gitlab.com/api/merge_requests/#list-merge-request-diffs
#[derive(Debug, Deserialize)]
struct FileDiff {
    old_path: String,
    new_path: String,
    diff: String,
}

#[derive(Debug, Deserialize)]
struct MergeRequestDiffRefs {
    base_sha: String,
    head_sha: String,
}

/// Metadata about a merge request. See
/// https://docs.gitlab.com/api/merge_requests/#retrieve-a-merge-request
#[derive(Debug, Deserialize)]
struct MergeRequestMetadata {
    diff_refs: MergeRequestDiffRefs,
}

#[async_trait::async_trait]
impl ReviewSource for GitlabReviewSource {
    async fn changed_files(&self) -> Result<Vec<String>, RigReviewError> {
        let diffs = self.get_mr_diff().await?;
        Ok(diffs.iter().map(|diff| diff.new_path.clone()).collect())
    }

    async fn changes(&self, file: &str) -> Result<String, RigReviewError> {
        let mr_diff = self.get_mr_diff().await?;

        let file_diff =
            mr_diff
                .iter()
                .find(|f| f.new_path == file)
                .ok_or(RigReviewError::FileNotChanged(format!(
                    "{} not found in diff",
                    file
                )))?;

        Ok(file_diff.diff.clone())
    }

    async fn original_file(&self, file: &str) -> Result<String, RigReviewError> {
        // Determine the old path
        let mr_diff = self.get_mr_diff().await?;
        let old_path = mr_diff
            .iter()
            .find(|f| f.new_path == file)
            .ok_or(RigReviewError::FileNotChanged(format!(
                "{} not found in diff",
                file
            )))?
            .old_path
            .clone();

        // Determine the old commit hash
        let mr_metadata = self.get_mr_metadata().await?;
        let base_sha = mr_metadata.diff_refs.base_sha.clone();

        // Retrieve that file from that commit
        let endpoint = FileRaw::builder()
            .project(&self.project_path)
            .ref_(base_sha)
            .file_path(old_path)
            .build()?;

        let client = self.client.lock().await;
        let data: Vec<u8> = endpoint.query_async(&*client).await?;

        Ok(String::from_utf8(data)?)
    }

    async fn modified_file(&self, file: &str) -> Result<String, RigReviewError> {
        // Determine the modified commit hash
        let mr_metadata = self.get_mr_metadata().await?;
        let head_sha = mr_metadata.diff_refs.head_sha.clone();

        // Retrieve that file from that commit
        let endpoint = FileRaw::builder()
            .project(&self.project_path)
            .ref_(head_sha)
            .file_path(file)
            .build()?;

        let client = self.client.lock().await;
        let data: Vec<u8> = endpoint.query_async(&*client).await?;

        Ok(String::from_utf8(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(host: Option<&str>) -> GitlabReviewSourceConfig {
        GitlabReviewSourceConfig {
            host: host.map(str::to_owned),
            api_token: String::new(),
        }
    }

    #[test]
    fn extracts_project_without_leading_slash() {
        let (security, host, base_path) = GitlabReviewSource::parse_host_and_scheme(
            &config(None),
            "https://gitlab.example.com/group/subgroup/project/-/merge_requests/42",
        )
        .unwrap();

        assert!(matches!(security, HttpSecurity::Secure));
        assert_eq!(host, "gitlab.example.com");
        assert_eq!(base_path, "");
        assert_eq!(
            GitlabReviewSource::parse_project_path_and_mr_iid(
                &base_path,
                "https://gitlab.example.com/group/subgroup/project/-/merge_requests/42",
            )
            .unwrap(),
            ("group/subgroup/project".to_owned(), 42)
        );
    }

    #[test]
    fn retains_configured_port_and_reverse_proxy_path() {
        let (security, host, base_path) = GitlabReviewSource::parse_host_and_scheme(
            &config(Some("https://gitlab.example.com:8443/gitlab/")),
            "https://gitlab.example.com:8443/gitlab/group/project/-/merge_requests/7",
        )
        .unwrap();

        assert!(matches!(security, HttpSecurity::Secure));
        assert_eq!(host, "gitlab.example.com:8443/gitlab");
        assert_eq!(base_path, "/gitlab");
        assert_eq!(
            GitlabReviewSource::parse_project_path_and_mr_iid(
                &base_path,
                "https://gitlab.example.com:8443/gitlab/group/project/-/merge_requests/7",
            )
            .unwrap(),
            ("group/project".to_owned(), 7)
        );
    }

    #[test]
    fn validates_mr_urls_against_the_configured_host() {
        let config = config(Some("https://gitlab.example.com"));

        assert!(GitlabReviewSource::is_valid_mr_url(
            "https://gitlab.example.com/group/project/-/merge_requests/42",
            &config,
        ));
        assert!(!GitlabReviewSource::is_valid_mr_url(
            "https://attacker.example/group/project/-/merge_requests/42",
            &config,
        ));
        assert!(!GitlabReviewSource::is_valid_mr_url(
            "https://user:password@gitlab.example.com/group/project/-/merge_requests/42",
            &config,
        ));
    }
}
