//! Provider- and mode-dispatching translation client facade.

use crate::clients::azure_openai_realtime_client::{
    AzureOpenAIRealtimeClient, AzureOpenAIRealtimeClientError,
};
use crate::clients::baidu_translate_client::{BaiduTranslateClient, BaiduTranslateClientError};
use crate::clients::gemini_live_client::{GeminiLiveClient, GeminiLiveClientError};
use crate::clients::high_quality_client::HighQualityTranslationClient;
use crate::clients::live_translate_client::{LiveTranslateClient, LiveTranslateClientError};
use crate::clients::openai_realtime_client::{OpenAIRealtimeClient, OpenAIRealtimeClientError};
use crate::clients::provider_events::ProviderEventSender;
use crate::clients::tencent_cloud_client::{TencentCloudClient, TencentCloudClientError};
use crate::clients::volcano_engine_client::{VolcanoEngineClient, VolcanoEngineClientError};
use crate::clients::xai_realtime_client::{XAIRealtimeClient, XAIRealtimeClientError};
use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::credentials::{ProviderCredentials, ProviderCredentialsError};
use crate::core::models::TranslationMode;
use crate::core::protocols::qwen_mt::{QwenMTClientError, QwenMTModel};
use crate::core::provider::ProviderKind;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Clone)]
pub enum TranslationClient {
    LowLatency(LiveTranslateClient),
    HighQuality(HighQualityTranslationClient),
    OpenAIRealtime(OpenAIRealtimeClient),
    GeminiLive(GeminiLiveClient),
    AzureOpenAIRealtime(AzureOpenAIRealtimeClient),
    TencentCloud(TencentCloudClient),
    BaiduTranslate(BaiduTranslateClient),
    VolcanoEngine(VolcanoEngineClient),
    XaiRealtime(XAIRealtimeClient),
}

impl TranslationClient {
    pub fn new(
        configuration: &LiveTranslationConfiguration,
        events: ProviderEventSender,
    ) -> Result<Self, TranslationClientError> {
        let credentials = configuration
            .credentials
            .validated_for(configuration.provider)?;
        match configuration.provider {
            ProviderKind::OpenAIRealtime => {
                return OpenAIRealtimeClient::new(
                    direct_api_key(&credentials)?,
                    configuration.target_language,
                    events,
                )
                .map(Self::OpenAIRealtime)
                .map_err(TranslationClientError::OpenAI);
            }
            ProviderKind::GoogleGeminiLive => {
                return GeminiLiveClient::new(
                    direct_api_key(&credentials)?,
                    configuration.target_language,
                    events,
                )
                .map(Self::GeminiLive)
                .map_err(TranslationClientError::Gemini);
            }
            ProviderKind::AzureOpenAIRealtime => {
                let (endpoint, deployment, transcription_deployment, api_key) = credentials
                    .azure_openai()
                    .ok_or(ProviderCredentialsError::ProviderMismatch)?;
                return AzureOpenAIRealtimeClient::new(
                    endpoint,
                    deployment,
                    transcription_deployment,
                    api_key,
                    configuration.target_language,
                    events,
                )
                .map(Self::AzureOpenAIRealtime)
                .map_err(TranslationClientError::AzureOpenAI);
            }
            ProviderKind::XAIRealtime => {
                return XAIRealtimeClient::new(
                    direct_api_key(&credentials)?,
                    configuration.target_language,
                    events,
                )
                .map(Self::XaiRealtime)
                .map_err(TranslationClientError::Xai);
            }
            ProviderKind::VolcanoEngine => {
                return VolcanoEngineClient::new(
                    direct_api_key(&credentials)?,
                    configuration.source_language,
                    configuration.target_language,
                    events,
                )
                .map(Self::VolcanoEngine)
                .map_err(TranslationClientError::VolcanoEngine);
            }
            ProviderKind::TencentCloud => {
                let (app_id, secret_id, secret_key) = credentials
                    .tencent_cloud()
                    .ok_or(ProviderCredentialsError::ProviderMismatch)?;
                return TencentCloudClient::new(
                    app_id,
                    secret_id,
                    secret_key,
                    configuration.source_language,
                    configuration.target_language,
                    events,
                )
                .map(Self::TencentCloud)
                .map_err(TranslationClientError::TencentCloud);
            }
            ProviderKind::BaiduTranslate => {
                let (app_id, app_key) = credentials
                    .baidu_translate()
                    .ok_or(ProviderCredentialsError::ProviderMismatch)?;
                return BaiduTranslateClient::new(
                    app_id,
                    app_key,
                    configuration.source_language,
                    configuration.target_language,
                    events,
                )
                .map(Self::BaiduTranslate)
                .map_err(TranslationClientError::BaiduTranslate);
            }
            ProviderKind::AlibabaCloud => {}
        }
        // Automatic source recognition omits the transcription language on
        // the wire so the recognition service detects the language per
        // utterance (both protocol encoders handle `Automatic` this way,
        // mirroring the original app's RealtimeASRProtocol).
        match configuration.effective_translation_mode() {
            TranslationMode::LowLatency => {
                let client = LiveTranslateClient::new(
                    direct_api_key(&credentials)?,
                    configuration.source_language,
                    configuration.target_language,
                    BTreeMap::new(),
                    events,
                )
                .map_err(TranslationClientError::Live)?;
                Ok(Self::LowLatency(client))
            }
            TranslationMode::HighQuality => {
                let client = HighQualityTranslationClient::new(
                    direct_api_key(&credentials)?,
                    configuration.source_language,
                    configuration.target_language,
                    QwenMTModel::Plus,
                    Duration::from_millis(1_200),
                    Duration::from_millis(4_500),
                    20,
                    events,
                )
                .map_err(TranslationClientError::MT)?;
                Ok(Self::HighQuality(client))
            }
            TranslationMode::Turbo => {
                let client = HighQualityTranslationClient::new(
                    direct_api_key(&credentials)?,
                    configuration.source_language,
                    configuration.target_language,
                    QwenMTModel::Flash,
                    Duration::from_millis(500),
                    Duration::from_millis(2_000),
                    12,
                    events,
                )
                .map_err(TranslationClientError::MT)?;
                Ok(Self::HighQuality(client))
            }
        }
    }

    pub async fn connect(&self) -> Result<(), ConnectError> {
        match self {
            Self::LowLatency(client) => client.connect().await.map_err(ConnectError::Live),
            Self::HighQuality(client) => client.connect().await.map_err(ConnectError::MT),
            Self::OpenAIRealtime(client) => client.connect().await.map_err(ConnectError::OpenAI),
            Self::GeminiLive(client) => client.connect().await.map_err(ConnectError::Gemini),
            Self::AzureOpenAIRealtime(client) => {
                client.connect().await.map_err(ConnectError::AzureOpenAI)
            }
            Self::TencentCloud(client) => {
                client.connect().await.map_err(ConnectError::TencentCloud)
            }
            Self::BaiduTranslate(client) => {
                client.connect().await.map_err(ConnectError::BaiduTranslate)
            }
            Self::VolcanoEngine(client) => {
                client.connect().await.map_err(ConnectError::VolcanoEngine)
            }
            Self::XaiRealtime(client) => client.connect().await.map_err(ConnectError::Xai),
        }
    }

    pub async fn send_audio(&self, pcm_data: &[u8]) -> Result<(), ConnectError> {
        match self {
            Self::LowLatency(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::Live),
            Self::HighQuality(client) => {
                client.send_audio(pcm_data).await.map_err(ConnectError::MT)
            }
            Self::OpenAIRealtime(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::OpenAI),
            Self::GeminiLive(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::Gemini),
            Self::AzureOpenAIRealtime(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::AzureOpenAI),
            Self::TencentCloud(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::TencentCloud),
            Self::BaiduTranslate(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::BaiduTranslate),
            Self::VolcanoEngine(client) => client
                .send_audio(pcm_data)
                .await
                .map_err(ConnectError::VolcanoEngine),
            Self::XaiRealtime(client) => {
                client.send_audio(pcm_data).await.map_err(ConnectError::Xai)
            }
        }
    }

    pub async fn ping(&self, timeout: Duration) -> Result<(), ConnectError> {
        match self {
            Self::LowLatency(client) => client.ping(timeout).await.map_err(ConnectError::Live),
            Self::HighQuality(client) => client.ping(timeout).await.map_err(ConnectError::MT),
            Self::OpenAIRealtime(client) => {
                client.ping(timeout).await.map_err(ConnectError::OpenAI)
            }
            Self::GeminiLive(client) => client.ping(timeout).await.map_err(ConnectError::Gemini),
            Self::AzureOpenAIRealtime(client) => client
                .ping(timeout)
                .await
                .map_err(ConnectError::AzureOpenAI),
            Self::TencentCloud(client) => client
                .ping(timeout)
                .await
                .map_err(ConnectError::TencentCloud),
            Self::BaiduTranslate(client) => client
                .ping(timeout)
                .await
                .map_err(ConnectError::BaiduTranslate),
            Self::VolcanoEngine(client) => client
                .ping(timeout)
                .await
                .map_err(ConnectError::VolcanoEngine),
            Self::XaiRealtime(client) => client.ping(timeout).await.map_err(ConnectError::Xai),
        }
    }

    pub async fn finish(&self) {
        match self {
            Self::LowLatency(client) => client.finish(Duration::from_secs(1)).await,
            Self::HighQuality(client) => client.finish().await,
            Self::OpenAIRealtime(client) => client.finish(Duration::from_secs(2)).await,
            Self::GeminiLive(client) => client.finish(Duration::from_secs(2)).await,
            Self::AzureOpenAIRealtime(client) => client.finish(Duration::from_secs(2)).await,
            Self::TencentCloud(client) => client.finish(Duration::from_secs(2)).await,
            Self::BaiduTranslate(client) => client.finish(Duration::from_secs(2)).await,
            Self::VolcanoEngine(client) => client.finish(Duration::from_secs(2)).await,
            Self::XaiRealtime(client) => client.finish(Duration::from_secs(2)).await,
        }
    }

    pub async fn disconnect(&self) {
        match self {
            Self::LowLatency(client) => client.disconnect().await,
            Self::HighQuality(client) => client.disconnect().await,
            Self::OpenAIRealtime(client) => client.disconnect().await,
            Self::GeminiLive(client) => client.disconnect().await,
            Self::AzureOpenAIRealtime(client) => client.disconnect().await,
            Self::TencentCloud(client) => client.disconnect().await,
            Self::BaiduTranslate(client) => client.disconnect().await,
            Self::VolcanoEngine(client) => client.disconnect().await,
            Self::XaiRealtime(client) => client.disconnect().await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("{0}")]
    Live(#[from] LiveTranslateClientError),
    #[error("{0}")]
    MT(#[from] QwenMTClientError),
    #[error("{0}")]
    OpenAI(#[from] OpenAIRealtimeClientError),
    #[error("{0}")]
    Gemini(#[from] GeminiLiveClientError),
    #[error("{0}")]
    AzureOpenAI(#[from] AzureOpenAIRealtimeClientError),
    #[error("{0}")]
    TencentCloud(#[from] TencentCloudClientError),
    #[error("{0}")]
    BaiduTranslate(#[from] BaiduTranslateClientError),
    #[error("{0}")]
    VolcanoEngine(#[from] VolcanoEngineClientError),
    #[error("{0}")]
    Xai(#[from] XAIRealtimeClientError),
}

impl ConnectError {
    /// Content-free provider label for diagnostics. The wrapped error remains
    /// available for user-facing status, but is never written verbatim to
    /// pipeline logs.
    pub fn diagnostic_label(&self) -> &'static str {
        match self {
            Self::Live(_) => "provider.live_translate",
            Self::MT(_) => "provider.alibaba_high_quality",
            Self::OpenAI(_) => "provider.openai_realtime",
            Self::Gemini(_) => "provider.google_gemini_live",
            Self::AzureOpenAI(_) => "provider.azure_openai_realtime",
            Self::TencentCloud(_) => "provider.tencent_cloud",
            Self::BaiduTranslate(_) => "provider.baidu_translate",
            Self::VolcanoEngine(_) => "provider.volcano_engine",
            Self::Xai(_) => "provider.xai_realtime",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TranslationClientError {
    #[error("{0}")]
    Live(#[from] LiveTranslateClientError),
    #[error("{0}")]
    MT(#[from] QwenMTClientError),
    #[error("{0}")]
    OpenAI(#[from] OpenAIRealtimeClientError),
    #[error("{0}")]
    Gemini(#[from] GeminiLiveClientError),
    #[error("{0}")]
    AzureOpenAI(#[from] AzureOpenAIRealtimeClientError),
    #[error("{0}")]
    TencentCloud(#[from] TencentCloudClientError),
    #[error("{0}")]
    BaiduTranslate(#[from] BaiduTranslateClientError),
    #[error("{0}")]
    VolcanoEngine(#[from] VolcanoEngineClientError),
    #[error("{0}")]
    Xai(#[from] XAIRealtimeClientError),
    #[error("{0}")]
    Credentials(#[from] ProviderCredentialsError),
}

impl TranslationClientError {
    pub fn diagnostic_label(&self) -> &'static str {
        match self {
            Self::Live(_) => "configuration.live_translate",
            Self::MT(_) => "configuration.alibaba_high_quality",
            Self::OpenAI(_) => "configuration.openai_realtime",
            Self::Gemini(_) => "configuration.google_gemini_live",
            Self::AzureOpenAI(_) => "configuration.azure_openai_realtime",
            Self::TencentCloud(_) => "configuration.tencent_cloud",
            Self::BaiduTranslate(_) => "configuration.baidu_translate",
            Self::VolcanoEngine(_) => "configuration.volcano_engine",
            Self::Xai(_) => "configuration.xai_realtime",
            Self::Credentials(_) => "configuration.provider_credentials",
        }
    }
}

fn direct_api_key(credentials: &ProviderCredentials) -> Result<&str, ProviderCredentialsError> {
    credentials
        .direct_api_key()
        .ok_or(ProviderCredentialsError::ProviderMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::provider_events::provider_event_channel;
    use crate::core::models::{SourceLanguage, TargetLanguage};

    #[test]
    fn provider_factory_selects_openai_realtime() {
        let configuration = LiveTranslationConfiguration::for_provider(
            ProviderKind::OpenAIRealtime,
            "sk-test-not-real",
            SourceLanguage::Automatic,
            TargetLanguage::English,
            TranslationMode::Turbo,
        );
        let (events, _receiver) = provider_event_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::OpenAIRealtime(_)
        ));
    }

    #[test]
    fn provider_factory_selects_alibaba() {
        let configuration = LiveTranslationConfiguration::for_provider(
            ProviderKind::AlibabaCloud,
            "sk-test-not-real",
            SourceLanguage::Automatic,
            TargetLanguage::SimplifiedChinese,
            TranslationMode::LowLatency,
        );
        let (events, _receiver) = provider_event_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::LowLatency(_)
        ));
    }

    #[test]
    fn provider_factory_selects_google_and_xai_realtime_adapters() {
        for (provider, expected) in [
            (ProviderKind::GoogleGeminiLive, "gemini"),
            (ProviderKind::XAIRealtime, "xai"),
        ] {
            let configuration = LiveTranslationConfiguration::for_provider(
                provider,
                "sk-test-not-real",
                SourceLanguage::Automatic,
                TargetLanguage::English,
                TranslationMode::Turbo,
            );
            let (events, _receiver) = provider_event_channel();
            let client = TranslationClient::new(&configuration, events).unwrap();
            assert!(matches!(
                (expected, client),
                ("gemini", TranslationClient::GeminiLive(_))
                    | ("xai", TranslationClient::XaiRealtime(_))
            ));
        }
    }

    #[test]
    fn provider_factory_selects_azure_with_structured_credentials() {
        let configuration = LiveTranslationConfiguration::with_credentials(
            ProviderKind::AzureOpenAIRealtime,
            ProviderCredentials::AzureOpenAI {
                endpoint: "https://mimi.openai.azure.com".into(),
                deployment: "translate".into(),
                transcription_deployment: "transcribe".into(),
                api_key: "azure-test-not-real".into(),
            },
            SourceLanguage::Automatic,
            TargetLanguage::Japanese,
            TranslationMode::Turbo,
        );
        let (events, _receiver) = provider_event_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::AzureOpenAIRealtime(_)
        ));
    }

    #[test]
    fn provider_factory_selects_volcano_engine() {
        let configuration = LiveTranslationConfiguration::for_provider(
            ProviderKind::VolcanoEngine,
            "volcano-test-not-real",
            SourceLanguage::English,
            TargetLanguage::Japanese,
            TranslationMode::Turbo,
        );
        let (events, _receiver) = provider_event_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::VolcanoEngine(_)
        ));
    }

    #[test]
    fn provider_factory_selects_tencent_with_structured_credentials() {
        let configuration = LiveTranslationConfiguration::with_credentials(
            ProviderKind::TencentCloud,
            ProviderCredentials::TencentCloud {
                app_id: "1250000000".into(),
                secret_id: "AKIDtest".into(),
                secret_key: "tencent-test-not-real".into(),
            },
            SourceLanguage::Japanese,
            TargetLanguage::SimplifiedChinese,
            TranslationMode::Turbo,
        );
        let (events, _receiver) = provider_event_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::TencentCloud(_)
        ));
    }

    #[test]
    fn provider_factory_selects_baidu_with_structured_credentials() {
        let configuration = LiveTranslationConfiguration::with_credentials(
            ProviderKind::BaiduTranslate,
            ProviderCredentials::BaiduTranslate {
                app_id: "baidu-app-id".into(),
                app_key: "baidu-test-not-real".into(),
            },
            SourceLanguage::English,
            TargetLanguage::Japanese,
            TranslationMode::Turbo,
        );
        let (events, _receiver) = provider_event_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::BaiduTranslate(_)
        ));
    }
}
