//! Preferences (workspace ID, languages, mode, font size, overlay state)
//! stored in `app_config_dir()/preferences.json`, and the DashScope API key
//! stored in the OS keychain (macOS Keychain / Windows Credential Manager)
//! via `keyring`. Ported from `AppSettings.swift` + `KeychainStore.swift`.
//!
//! No plaintext, source-controlled, or environment-variable credential
//! fallback exists.

use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

pub const KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.credentials.v2";
pub const LEGACY_KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.translation";
pub const KEYCHAIN_ACCOUNT: &str = "dashscope-api-key";

pub const FONT_SIZE_RANGE: std::ops::RangeInclusive<f64> = 14.0..=20.0;
pub const DEFAULT_FONT_SIZE: f64 = 18.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub workspace_id: String,
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
    pub font_size: f64,
    pub overlay_locked: bool,
    pub overlay_frame: Option<OverlayFrame>,
    pub frame_layout_version: u64,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            workspace_id: String::new(),
            source_language: SourceLanguage::Automatic,
            target_language: TargetLanguage::SimplifiedChinese,
            translation_mode: TranslationMode::LowLatency,
            font_size: DEFAULT_FONT_SIZE,
            overlay_locked: false,
            overlay_frame: None,
            frame_layout_version: 0,
        }
    }
}

/// Abstraction over the OS secret store so tests can use an in-memory
/// implementation; the production implementation uses `keyring`.
pub trait SecretStore: Send + Sync {
    fn load(&self) -> Result<Option<String>, String>;
    fn save(&self, value: &str) -> Result<(), String>;
}

struct KeyringSecretStore;

impl SecretStore for KeyringSecretStore {
    fn load(&self) -> Result<Option<String>, String> {
        load_from_keyring(KEYCHAIN_SERVICE).map_err(|error| format!("Keychain: {error}"))
    }

    fn save(&self, value: &str) -> Result<(), String> {
        save_to_keyring(KEYCHAIN_SERVICE, value)
    }
}

fn load_from_keyring(service: &str) -> Result<Option<String>, String> {
    match keyring::Entry::new(service, KEYCHAIN_ACCOUNT) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        },
        Err(error) => Err(error.to_string()),
    }
}

fn save_to_keyring(service: &str, value: &str) -> Result<(), String> {
    let entry =
        keyring::Entry::new(service, KEYCHAIN_ACCOUNT).map_err(|error| error.to_string())?;
    entry.set_password(value).map_err(|error| error.to_string())
}

pub struct SettingsStore {
    prefs_path: PathBuf,
    prefs: Mutex<Preferences>,
    secret: Box<dyn SecretStore>,
    is_ui_test: bool,
}

impl SettingsStore {
    /// Loads preferences from `app_config_dir()/preferences.json`.
    pub fn load(app_config_dir: PathBuf, is_ui_test: bool) -> Self {
        let prefs_path = app_config_dir.join("preferences.json");
        let mut prefs = std::fs::read_to_string(&prefs_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Preferences>(&text).ok())
            .unwrap_or_default();
        if is_ui_test {
            prefs.workspace_id = "your-workspace-id".into();
        }
        let font_size = prefs
            .font_size
            .clamp(*FONT_SIZE_RANGE.start(), *FONT_SIZE_RANGE.end());
        let prefs = Preferences { font_size, ..prefs };
        Self {
            prefs_path,
            prefs: Mutex::new(prefs),
            secret: Box::new(KeyringSecretStore),
            is_ui_test,
        }
    }

    #[cfg(test)]
    pub fn in_memory(secret: Box<dyn SecretStore>, is_ui_test: bool) -> Self {
        Self {
            prefs_path: PathBuf::new(),
            prefs: Mutex::new(Preferences::default()),
            secret,
            is_ui_test,
        }
    }

    pub fn is_ui_test(&self) -> bool {
        self.is_ui_test
    }

    pub fn preferences(&self) -> Preferences {
        self.prefs.lock().unwrap().clone()
    }

    pub fn update_preferences(&self, update: impl FnOnce(&mut Preferences)) {
        let mut prefs = self.prefs.lock().unwrap();
        update(&mut prefs);
    }

    /// The stored API key, or `None` when missing. Reading the legacy service
    /// migrates the value to the current one (same behavior as the Swift
    /// `KeychainStore.loadAPIKey`).
    pub fn load_api_key(&self) -> Result<Option<String>, String> {
        if self.is_ui_test {
            return Ok(Some("sk-demo-not-a-real-key".into()));
        }
        if let Some(value) = self.secret.load()? {
            return Ok(Some(value));
        }
        let legacy = load_from_keyring(LEGACY_KEYCHAIN_SERVICE)?;
        if let Some(legacy_value) = legacy {
            self.secret.save(&legacy_value)?;
            return Ok(Some(legacy_value));
        }
        Ok(None)
    }

    pub fn credential_load_error(&self) -> Option<String> {
        self.load_api_key().err()
    }

    pub fn has_api_key(&self) -> bool {
        self.load_api_key()
            .ok()
            .flatten()
            .is_some_and(|key| !key.is_empty())
    }

    /// Validates and saves the workspace ID + API key (key → OS keychain),
    /// then persists preferences. Mirrors `AppSettings.save()`.
    pub fn save_credentials(&self, workspace_id: &str, api_key: &str) -> Result<(), String> {
        let prefs = self.prefs.lock().unwrap().clone();
        let configuration = LiveTranslationConfiguration::new(
            workspace_id,
            api_key,
            prefs.source_language,
            prefs.target_language,
            prefs.translation_mode,
        );
        let validated = configuration
            .validated()
            .map_err(|error| error.to_string())?;

        let effective_mode = validated.effective_translation_mode();
        let mut prefs = self.prefs.lock().unwrap();
        prefs.workspace_id = validated.workspace_id.clone();
        prefs.target_language = validated.target_language;
        prefs.translation_mode = effective_mode;
        drop(prefs);

        if !self.is_ui_test {
            self.secret.save(&validated.api_key)?;
        }
        self.persist();
        Ok(())
    }

    /// The validated configuration used to start a session.
    pub fn configuration(&self) -> Result<LiveTranslationConfiguration, String> {
        let prefs = self.prefs.lock().unwrap().clone();
        let api_key = self.load_api_key()?.unwrap_or_default();
        LiveTranslationConfiguration::new(
            prefs.workspace_id,
            api_key,
            prefs.source_language,
            prefs.target_language,
            prefs.translation_mode,
        )
        .validated()
        .map_err(|error| error.to_string())
    }

    /// Applies the listening-time language adjustments: automatic source
    /// becomes Japanese, Chinese source switches to original subtitles.
    pub fn prepare_for_listening(&self) {
        let mut prefs = self.prefs.lock().unwrap();
        if prefs.source_language == SourceLanguage::Automatic {
            prefs.source_language = SourceLanguage::Japanese;
        }
        if prefs.source_language == SourceLanguage::Chinese {
            prefs.target_language = TargetLanguage::Original;
        }
        drop(prefs);
        self.persist();
    }

    pub fn persist(&self) {
        if self.is_ui_test {
            return;
        }
        let prefs = self.prefs.lock().unwrap();
        if let Ok(text) = serde_json::to_string_pretty(&*prefs) {
            if let Some(parent) = self.prefs_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&self.prefs_path, text);
        }
    }
}
