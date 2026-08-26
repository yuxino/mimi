//! Provider-specific credentials kept exclusively in the OS keychain.

use crate::core::provider::ProviderKind;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

const MAXIMUM_CREDENTIAL_FIELD_LENGTH: usize = 1_024;
const MAXIMUM_DEPLOYMENT_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderCredentialsError {
    #[error("Add the connection credentials for {0} in Settings.")]
    Missing(ProviderKind),
    #[error("The saved credentials do not match the selected service.")]
    ProviderMismatch,
    #[error("The Azure OpenAI endpoint must be an official HTTPS resource endpoint.")]
    InvalidAzureEndpoint,
    #[error("The Azure OpenAI deployment name is invalid.")]
    InvalidAzureDeployment,
    #[error("One or more credential fields are invalid.")]
    InvalidField,
    #[error("The saved credentials could not be read.")]
    InvalidStoredValue,
}

/// Write-only IPC payload and keychain representation. The tagged JSON form is
/// used only for providers with multiple fields; historical one-key values
/// remain raw strings in Keychain for rollback compatibility.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ProviderCredentials {
    ApiKey {
        api_key: String,
    },
    AzureOpenAI {
        endpoint: String,
        deployment: String,
        transcription_deployment: String,
        api_key: String,
    },
    TencentCloud {
        app_id: String,
        secret_id: String,
        secret_key: String,
    },
    BaiduTranslate {
        app_id: String,
        app_key: String,
    },
}

impl fmt::Debug for ProviderCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderCredentials")
            .field("kind", &self.kind_label())
            .field("values", &"[REDACTED]")
            .finish()
    }
}

impl ProviderCredentials {
    pub fn api_key(value: impl Into<String>) -> Self {
        Self::ApiKey {
            api_key: value.into(),
        }
    }

    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::ApiKey { .. } => "api_key",
            Self::AzureOpenAI { .. } => "azure_openai",
            Self::TencentCloud { .. } => "tencent_cloud",
            Self::BaiduTranslate { .. } => "baidu_translate",
        }
    }

    pub fn validated_for(&self, provider: ProviderKind) -> Result<Self, ProviderCredentialsError> {
        match (provider, self) {
            (provider, Self::ApiKey { api_key }) if provider.uses_api_key_only() => {
                Ok(Self::ApiKey {
                    api_key: required_field(api_key, provider)?,
                })
            }
            (
                ProviderKind::AzureOpenAIRealtime,
                Self::AzureOpenAI {
                    endpoint,
                    deployment,
                    transcription_deployment,
                    api_key,
                },
            ) => Ok(Self::AzureOpenAI {
                endpoint: validated_azure_endpoint(endpoint)?,
                deployment: validated_deployment(deployment)?,
                transcription_deployment: validated_deployment(transcription_deployment)?,
                api_key: required_field(api_key, provider)?,
            }),
            (
                ProviderKind::TencentCloud,
                Self::TencentCloud {
                    app_id,
                    secret_id,
                    secret_key,
                },
            ) => {
                let app_id = required_field(app_id, provider)?;
                if !app_id.bytes().all(|byte| byte.is_ascii_digit()) || app_id.len() > 32 {
                    return Err(ProviderCredentialsError::InvalidField);
                }
                Ok(Self::TencentCloud {
                    app_id,
                    secret_id: required_field(secret_id, provider)?,
                    secret_key: required_field(secret_key, provider)?,
                })
            }
            (ProviderKind::BaiduTranslate, Self::BaiduTranslate { app_id, app_key }) => {
                Ok(Self::BaiduTranslate {
                    app_id: identifier_field(app_id, provider)?,
                    app_key: required_field(app_key, provider)?,
                })
            }
            _ => Err(ProviderCredentialsError::ProviderMismatch),
        }
    }

    pub fn encode_for_keychain(
        &self,
        provider: ProviderKind,
    ) -> Result<String, ProviderCredentialsError> {
        let credentials = self.validated_for(provider)?;
        match credentials {
            Self::ApiKey { api_key } => Ok(api_key),
            other => serde_json::to_string(&other)
                .map_err(|_| ProviderCredentialsError::InvalidStoredValue),
        }
    }

    pub fn decode_from_keychain(
        provider: ProviderKind,
        value: &str,
    ) -> Result<Self, ProviderCredentialsError> {
        if value.trim().is_empty() {
            return Err(ProviderCredentialsError::Missing(provider));
        }
        if provider.uses_api_key_only() {
            return Self::api_key(value).validated_for(provider);
        }
        serde_json::from_str::<Self>(value)
            .map_err(|_| ProviderCredentialsError::InvalidStoredValue)?
            .validated_for(provider)
    }

    pub fn direct_api_key(&self) -> Option<&str> {
        match self {
            Self::ApiKey { api_key } => Some(api_key),
            Self::AzureOpenAI { api_key, .. } => Some(api_key),
            Self::TencentCloud { .. } | Self::BaiduTranslate { .. } => None,
        }
    }

    pub fn azure_openai(&self) -> Option<(&str, &str, &str, &str)> {
        match self {
            Self::AzureOpenAI {
                endpoint,
                deployment,
                transcription_deployment,
                api_key,
            } => Some((endpoint, deployment, transcription_deployment, api_key)),
            _ => None,
        }
    }

    pub fn tencent_cloud(&self) -> Option<(&str, &str, &str)> {
        match self {
            Self::TencentCloud {
                app_id,
                secret_id,
                secret_key,
            } => Some((app_id, secret_id, secret_key)),
            _ => None,
        }
    }

    pub fn baidu_translate(&self) -> Option<(&str, &str)> {
        match self {
            Self::BaiduTranslate { app_id, app_key } => Some((app_id, app_key)),
            _ => None,
        }
    }
}

fn required_field(value: &str, provider: ProviderKind) -> Result<String, ProviderCredentialsError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ProviderCredentialsError::Missing(provider));
    }
    if value.chars().count() > MAXIMUM_CREDENTIAL_FIELD_LENGTH
        || value.chars().any(char::is_control)
    {
        return Err(ProviderCredentialsError::InvalidField);
    }
    Ok(value.to_string())
}

fn identifier_field(
    value: &str,
    provider: ProviderKind,
) -> Result<String, ProviderCredentialsError> {
    let value = required_field(value, provider)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ProviderCredentialsError::InvalidField);
    }
    Ok(value)
}

fn validated_deployment(value: &str) -> Result<String, ProviderCredentialsError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAXIMUM_DEPLOYMENT_LENGTH
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ProviderCredentialsError::InvalidAzureDeployment);
    }
    Ok(value.to_string())
}

fn validated_azure_endpoint(value: &str) -> Result<String, ProviderCredentialsError> {
    let value = value.trim().trim_end_matches('/');
    let endpoint =
        url::Url::parse(value).map_err(|_| ProviderCredentialsError::InvalidAzureEndpoint)?;
    let host = endpoint
        .host_str()
        .ok_or(ProviderCredentialsError::InvalidAzureEndpoint)?
        .to_ascii_lowercase();
    let official_host = [".openai.azure.com", ".openai.azure.cn", ".openai.azure.us"]
        .iter()
        .any(|suffix| host.ends_with(suffix) && host.len() > suffix.len());
    if endpoint.scheme() != "https"
        || !official_host
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.port().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || !matches!(endpoint.path(), "" | "/")
    {
        return Err(ProviderCredentialsError::InvalidAzureEndpoint);
    }
    Ok(format!("https://{host}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_single_api_keys_remain_raw_and_readable() {
        let credentials = ProviderCredentials::decode_from_keychain(
            ProviderKind::OpenAIRealtime,
            "  sk-legacy  ",
        )
        .unwrap();
        assert_eq!(
            credentials
                .encode_for_keychain(ProviderKind::OpenAIRealtime)
                .unwrap(),
            "sk-legacy"
        );
    }

    #[test]
    fn structured_credentials_round_trip_without_debug_disclosure() {
        let secret = "tencent-secret-value";
        let credentials = ProviderCredentials::TencentCloud {
            app_id: "123456".into(),
            secret_id: "AKIDexample".into(),
            secret_key: secret.into(),
        };
        let encoded = credentials
            .encode_for_keychain(ProviderKind::TencentCloud)
            .unwrap();
        assert_eq!(
            ProviderCredentials::decode_from_keychain(ProviderKind::TencentCloud, &encoded)
                .unwrap(),
            credentials
        );
        assert!(!format!("{credentials:?}").contains(secret));
    }

    #[test]
    fn frontend_camel_case_ipc_shapes_deserialize_for_every_credential_kind() {
        let cases = [
            (
                r#"{"kind":"apiKey","apiKey":"sk-test"}"#,
                ProviderCredentials::api_key("sk-test"),
            ),
            (
                r#"{"kind":"azureOpenAI","endpoint":"https://mimi.openai.azure.com","deployment":"translate","transcriptionDeployment":"transcribe","apiKey":"azure-secret"}"#,
                ProviderCredentials::AzureOpenAI {
                    endpoint: "https://mimi.openai.azure.com".into(),
                    deployment: "translate".into(),
                    transcription_deployment: "transcribe".into(),
                    api_key: "azure-secret".into(),
                },
            ),
            (
                r#"{"kind":"tencentCloud","appId":"123456","secretId":"AKIDexample","secretKey":"tencent-secret"}"#,
                ProviderCredentials::TencentCloud {
                    app_id: "123456".into(),
                    secret_id: "AKIDexample".into(),
                    secret_key: "tencent-secret".into(),
                },
            ),
            (
                r#"{"kind":"baiduTranslate","appId":"app_123","appKey":"baidu-secret"}"#,
                ProviderCredentials::BaiduTranslate {
                    app_id: "app_123".into(),
                    app_key: "baidu-secret".into(),
                },
            ),
        ];

        for (json, expected) in cases {
            assert_eq!(
                serde_json::from_str::<ProviderCredentials>(json).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn credential_ipc_shape_rejects_snake_case_and_unknown_fields() {
        for json in [
            r#"{"kind":"apiKey","api_key":"sk-test"}"#,
            r#"{"kind":"azureOpenAI","endpoint":"https://mimi.openai.azure.com","deployment":"translate","transcription_deployment":"transcribe","apiKey":"secret"}"#,
            r#"{"kind":"tencentCloud","appId":"123456","secretId":"AKIDexample","secretKey":"secret","extra":"not-allowed"}"#,
            r#"{"kind":"baiduTranslate","appId":"app_123","app_key":"secret"}"#,
        ] {
            assert!(serde_json::from_str::<ProviderCredentials>(json).is_err());
        }
    }

    #[test]
    fn providers_reject_the_wrong_credential_shape() {
        assert_eq!(
            ProviderCredentials::api_key("sk-test")
                .validated_for(ProviderKind::TencentCloud)
                .unwrap_err(),
            ProviderCredentialsError::ProviderMismatch
        );
    }

    #[test]
    fn azure_accepts_only_official_resource_endpoints() {
        let valid = ProviderCredentials::AzureOpenAI {
            endpoint: " https://mimi-test.openai.azure.com/ ".into(),
            deployment: " translate-prod ".into(),
            transcription_deployment: " transcribe-prod ".into(),
            api_key: " azure-secret ".into(),
        }
        .validated_for(ProviderKind::AzureOpenAIRealtime)
        .unwrap();
        assert_eq!(
            valid.azure_openai(),
            Some((
                "https://mimi-test.openai.azure.com",
                "translate-prod",
                "transcribe-prod",
                "azure-secret"
            ))
        );

        for endpoint in [
            "http://mimi.openai.azure.com",
            "https://example.com",
            "https://mimi.openai.azure.com/path",
            "https://mimi.openai.azure.com?secret=value",
            "https://user@mimi.openai.azure.com",
        ] {
            assert_eq!(
                ProviderCredentials::AzureOpenAI {
                    endpoint: endpoint.into(),
                    deployment: "translate".into(),
                    transcription_deployment: "transcribe".into(),
                    api_key: "secret".into(),
                }
                .validated_for(ProviderKind::AzureOpenAIRealtime)
                .unwrap_err(),
                ProviderCredentialsError::InvalidAzureEndpoint
            );
        }
    }

    #[test]
    fn every_required_value_is_trimmed_and_empty_values_fail_closed() {
        let baidu = ProviderCredentials::BaiduTranslate {
            app_id: " app_123 ".into(),
            app_key: " secret ".into(),
        }
        .validated_for(ProviderKind::BaiduTranslate)
        .unwrap();
        assert_eq!(baidu.baidu_translate(), Some(("app_123", "secret")));

        assert!(matches!(
            ProviderCredentials::TencentCloud {
                app_id: "123".into(),
                secret_id: "".into(),
                secret_key: "secret".into(),
            }
            .validated_for(ProviderKind::TencentCloud),
            Err(ProviderCredentialsError::Missing(
                ProviderKind::TencentCloud
            ))
        ));
    }
}
