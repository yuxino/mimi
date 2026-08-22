//! Provider- and mode-dispatching translation client facade.

use crate::clients::high_quality_client::HighQualityTranslationClient;
use crate::clients::live_translate_client::{LiveTranslateClient, LiveTranslateClientError};
use crate::clients::openai_realtime_client::{OpenAIRealtimeClient, OpenAIRealtimeClientError};
use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::models::TranslationMode;
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::protocols::qwen_mt::{QwenMTClientError, QwenMTModel};
use crate::core::provider::ProviderKind;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum TranslationClient {
    LowLatency(LiveTranslateClient),
    HighQuality(HighQualityTranslationClient),
    OpenAIRealtime(OpenAIRealtimeClient),
}

impl TranslationClient {
    pub fn new(
        configuration: &LiveTranslationConfiguration,
        events: mpsc::UnboundedSender<LiveTranslateServerEvent>,
    ) -> Result<Self, TranslationClientError> {
        if configuration.provider == ProviderKind::OpenAIRealtime {
            return OpenAIRealtimeClient::new(
                &configuration.api_key,
                configuration.target_language,
                events,
            )
            .map(Self::OpenAIRealtime)
            .map_err(TranslationClientError::OpenAI);
        }
        // Automatic source recognition omits the transcription language on
        // the wire so the recognition service detects the language per
        // utterance (both protocol encoders handle `Automatic` this way,
        // mirroring the original app's RealtimeASRProtocol).
        match configuration.effective_translation_mode() {
            TranslationMode::LowLatency => {
                let client = LiveTranslateClient::new(
                    &configuration.api_key,
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
                    &configuration.api_key,
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
                    &configuration.api_key,
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
        }
    }

    pub async fn ping(&self, timeout: Duration) -> Result<(), ConnectError> {
        match self {
            Self::LowLatency(client) => client.ping(timeout).await.map_err(ConnectError::Live),
            Self::HighQuality(client) => client.ping(timeout).await.map_err(ConnectError::MT),
            Self::OpenAIRealtime(client) => {
                client.ping(timeout).await.map_err(ConnectError::OpenAI)
            }
        }
    }

    pub async fn finish(&self) {
        match self {
            Self::LowLatency(client) => client.finish(Duration::from_secs(1)).await,
            Self::HighQuality(client) => client.finish().await,
            Self::OpenAIRealtime(client) => client.finish(Duration::from_secs(2)).await,
        }
    }

    pub async fn disconnect(&self) {
        match self {
            Self::LowLatency(client) => client.disconnect().await,
            Self::HighQuality(client) => client.disconnect().await,
            Self::OpenAIRealtime(client) => client.disconnect().await,
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
}

#[derive(Debug, thiserror::Error)]
pub enum TranslationClientError {
    #[error("{0}")]
    Live(#[from] LiveTranslateClientError),
    #[error("{0}")]
    MT(#[from] QwenMTClientError),
    #[error("{0}")]
    OpenAI(#[from] OpenAIRealtimeClientError),
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let (events, _receiver) = mpsc::unbounded_channel();
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
        let (events, _receiver) = mpsc::unbounded_channel();
        assert!(matches!(
            TranslationClient::new(&configuration, events).unwrap(),
            TranslationClient::LowLatency(_)
        ));
    }
}
