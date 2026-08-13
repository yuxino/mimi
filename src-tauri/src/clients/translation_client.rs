//! Mode-dispatching translation client, ported from
//! `Sources/MimiCore/TranslationClient.swift`.

use crate::clients::high_quality_client::HighQualityTranslationClient;
use crate::clients::live_translate_client::{LiveTranslateClient, LiveTranslateClientError};
use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::models::TranslationMode;
use crate::core::protocols::live_translate::LiveTranslateServerEvent;
use crate::core::protocols::qwen_mt::{QwenMTClientError, QwenMTModel};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum TranslationClient {
    LowLatency(LiveTranslateClient),
    HighQuality(HighQualityTranslationClient),
}

impl TranslationClient {
    pub fn new(
        configuration: &LiveTranslationConfiguration,
        events: mpsc::UnboundedSender<LiveTranslateServerEvent>,
    ) -> Result<Self, QwenMTClientError> {
        match configuration.effective_translation_mode() {
            TranslationMode::LowLatency => {
                let client = LiveTranslateClient::new(
                    &configuration.workspace_id,
                    &configuration.api_key,
                    configuration.source_language,
                    configuration.target_language,
                    BTreeMap::new(),
                    events,
                )
                .map_err(|_| QwenMTClientError::MissingAPIKey)?;
                Ok(Self::LowLatency(client))
            }
            TranslationMode::HighQuality => {
                let client = HighQualityTranslationClient::new(
                    &configuration.workspace_id,
                    &configuration.api_key,
                    configuration.source_language,
                    configuration.target_language,
                    QwenMTModel::Plus,
                    Duration::from_millis(1_200),
                    Duration::from_millis(4_500),
                    20,
                    events,
                )?;
                Ok(Self::HighQuality(client))
            }
            TranslationMode::Turbo => {
                let client = HighQualityTranslationClient::new(
                    &configuration.workspace_id,
                    &configuration.api_key,
                    configuration.source_language,
                    configuration.target_language,
                    QwenMTModel::Flash,
                    Duration::from_millis(500),
                    Duration::from_millis(2_000),
                    12,
                    events,
                )?;
                Ok(Self::HighQuality(client))
            }
        }
    }

    pub async fn connect(&self) -> Result<(), ConnectError> {
        match self {
            Self::LowLatency(client) => client.connect().await.map_err(ConnectError::Live),
            Self::HighQuality(client) => client.connect().await.map_err(ConnectError::MT),
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
        }
    }

    pub async fn ping(&self, timeout: Duration) -> Result<(), ConnectError> {
        match self {
            Self::LowLatency(client) => client.ping(timeout).await.map_err(ConnectError::Live),
            Self::HighQuality(client) => client.ping(timeout).await.map_err(ConnectError::MT),
        }
    }

    pub async fn finish(&self) {
        match self {
            Self::LowLatency(client) => client.finish(Duration::from_secs(1)).await,
            Self::HighQuality(client) => client.finish().await,
        }
    }

    pub async fn disconnect(&self) {
        match self {
            Self::LowLatency(client) => client.disconnect().await,
            Self::HighQuality(client) => client.disconnect().await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("{0}")]
    Live(#[from] LiveTranslateClientError),
    #[error("{0}")]
    MT(#[from] QwenMTClientError),
}
