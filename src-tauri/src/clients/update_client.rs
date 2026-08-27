//! Read-only client for the latest public mimi GitHub release tag.

use futures_util::StreamExt;
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

const LATEST_RELEASE_API_URL: &str = "https://api.github.com/repos/yuxino/mimi/releases/latest";
const GITHUB_API_VERSION: &str = "2022-11-28";
const MAX_RELEASE_RESPONSE_BYTES: usize = 1024 * 1024;
pub const RELEASES_LATEST_URL: &str = "https://github.com/yuxino/mimi/releases/latest";

#[derive(Debug, Error)]
pub enum UpdateClientError {
    #[error("client_build_failed")]
    ClientBuild(#[source] reqwest::Error),
    #[error("request_failed")]
    Request(#[source] reqwest::Error),
    #[error("unexpected_http_status_{0}")]
    HttpStatus(u16),
    #[error("invalid_response")]
    InvalidResponse(#[source] serde_json::Error),
    #[error("response_too_large")]
    ResponseTooLarge,
}

impl UpdateClientError {
    pub fn diagnostic_label(&self) -> &'static str {
        match self {
            Self::ClientBuild(_) => "client_build_failed",
            Self::Request(_) => "request_failed",
            Self::HttpStatus(_) => "http_status",
            Self::InvalidResponse(_) => "invalid_response",
            Self::ResponseTooLarge => "response_too_large",
        }
    }

    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::HttpStatus(status) => Some(*status),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Fetches only public release metadata. The client is created per manual
/// check so there is no background task or long-lived network state.
pub async fn latest_release_tag() -> Result<String, UpdateClientError> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent(format!("mimi/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(UpdateClientError::ClientBuild)?;
    let response = client
        .get(LATEST_RELEASE_API_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .send()
        .await
        .map_err(UpdateClientError::Request)?;

    if !response.status().is_success() {
        return Err(UpdateClientError::HttpStatus(response.status().as_u16()));
    }

    let body = read_bounded_body(response).await?;
    parse_latest_release(&body)
}

async fn read_bounded_body(response: reqwest::Response) -> Result<Vec<u8>, UpdateClientError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RELEASE_RESPONSE_BYTES as u64)
    {
        return Err(UpdateClientError::ResponseTooLarge);
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(UpdateClientError::Request)?;
        if body.len().saturating_add(chunk.len()) > MAX_RELEASE_RESPONSE_BYTES {
            return Err(UpdateClientError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn parse_latest_release(body: &[u8]) -> Result<String, UpdateClientError> {
    let release: GitHubRelease =
        serde_json::from_slice(body).map_err(UpdateClientError::InvalidResponse)?;
    Ok(release.tag_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_latest_release_tag_without_retaining_release_content() {
        let body = br#"{
          "tag_name": "v1.4.0",
          "body": "release notes are intentionally ignored",
          "assets": [{"name": "mimi.dmg"}]
        }"#;

        assert_eq!(parse_latest_release(body).unwrap(), "v1.4.0");
    }

    #[test]
    fn rejects_a_response_without_a_release_tag() {
        assert!(matches!(
            parse_latest_release(br#"{"name":"mimi"}"#),
            Err(UpdateClientError::InvalidResponse(_))
        ));
    }

    #[test]
    fn response_size_error_has_a_content_free_diagnostic_label() {
        let error = UpdateClientError::ResponseTooLarge;
        assert_eq!(error.diagnostic_label(), "response_too_large");
        assert_eq!(error.status_code(), None);
    }
}
