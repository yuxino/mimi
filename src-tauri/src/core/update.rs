//! Pure version comparison for manual application update checks.

use semver::Version;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAvailability {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VersionComparisonError {
    #[error("current application version is not valid SemVer")]
    InvalidCurrentVersion,
    #[error("latest release tag is not valid SemVer")]
    InvalidReleaseTag,
}

/// Compares the running version with a GitHub release tag. GitHub tags may
/// carry a conventional leading `v`; display versions are returned normalized.
pub fn compare_release_versions(
    current_version: &str,
    release_tag: &str,
) -> Result<UpdateAvailability, VersionComparisonError> {
    let current = Version::parse(current_version.trim())
        .map_err(|_| VersionComparisonError::InvalidCurrentVersion)?;
    let trimmed_tag = release_tag.trim();
    let normalized_tag = trimmed_tag
        .strip_prefix('v')
        .or_else(|| trimmed_tag.strip_prefix('V'))
        .unwrap_or(trimmed_tag);
    let latest =
        Version::parse(normalized_tag).map_err(|_| VersionComparisonError::InvalidReleaseTag)?;

    Ok(UpdateAvailability {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        update_available: latest > current,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_release_is_available_and_leading_v_is_normalized() {
        let result = compare_release_versions("1.3.1", "v1.4.0").unwrap();

        assert_eq!(result.current_version, "1.3.1");
        assert_eq!(result.latest_version, "1.4.0");
        assert!(result.update_available);
    }

    #[test]
    fn equal_or_older_release_is_not_reported_as_an_update() {
        assert!(
            !compare_release_versions("1.3.1", "1.3.1")
                .unwrap()
                .update_available
        );
        assert!(
            !compare_release_versions("1.3.1", "v1.3.0")
                .unwrap()
                .update_available
        );
    }

    #[test]
    fn stable_release_supersedes_a_prerelease() {
        assert!(
            compare_release_versions("1.4.0-beta.2", "V1.4.0")
                .unwrap()
                .update_available
        );
        assert!(
            !compare_release_versions("1.4.0", "v1.4.0-beta.2")
                .unwrap()
                .update_available
        );
    }

    #[test]
    fn malformed_versions_are_rejected() {
        assert_eq!(
            compare_release_versions("development", "v1.4.0"),
            Err(VersionComparisonError::InvalidCurrentVersion)
        );
        assert_eq!(
            compare_release_versions("1.3.1", "latest"),
            Err(VersionComparisonError::InvalidReleaseTag)
        );
    }
}
