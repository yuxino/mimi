//! Non-secret preferences, service-profile metadata, and OS-backed credentials.
//!
//! General preferences and the profile catalog are separate JSON documents.
//! API keys never enter either document: each profile/provider pair owns one
//! account in the operating-system credential store.

use crate::core::configuration::LiveTranslationConfiguration;
use crate::core::credentials::ProviderCredentials;
use crate::core::models::{SourceLanguage, TargetLanguage, TranslationMode};
use crate::core::provider::{
    ProviderKind, ProviderPreferences, ServiceProfile, DEFAULT_ALIBABA_PROFILE_ID,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[cfg(unix)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const PROFILE_KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.credentials.profiles";
pub const DEVELOPMENT_APPLICATION_IDENTIFIER: &str = "app.yuxino.mimi.dev";
const DEVELOPMENT_PROFILE_KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.dev.credentials.profiles";
pub const LEGACY_KEYCHAIN_SERVICE_V3: &str = "app.yuxino.mimi.credentials.v3";
pub const LEGACY_KEYCHAIN_SERVICE_V2: &str = "app.yuxino.mimi.credentials.v2";
pub const LEGACY_KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.translation";
pub const LEGACY_KEYCHAIN_ACCOUNT: &str = "dashscope-api-key";

const PROFILE_CATALOG_FILE: &str = "service-profiles.json";
const PROFILE_CATALOG_SCHEMA_VERSION: u32 = 1;
const LEGACY_MIGRATION_TOMBSTONE_ACCOUNT: &str = "migration:legacy-alibaba:v1";
const LEGACY_MIGRATION_TOMBSTONE_VALUE: &str = "complete";
const PROFILE_CATALOG_UNAVAILABLE: &str = "Service profile settings are unavailable.";
const CREDENTIAL_STORE_UNAVAILABLE: &str = "The system credential store is unavailable.";
const PREFERENCES_UNAVAILABLE: &str = "Settings could not be saved.";
const PROFILE_NOT_FOUND: &str = "The service profile does not exist.";
const LAST_PROFILE: &str = "At least one service profile is required.";

pub const MAXIMUM_PROFILE_COUNT: usize = 20;
pub const FONT_SIZE_RANGE: std::ops::RangeInclusive<f64> = 14.0..=20.0;
pub const DEFAULT_FONT_SIZE: f64 = 18.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubtitleAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OverlayFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub source_language: SourceLanguage,
    pub target_language: TargetLanguage,
    pub translation_mode: TranslationMode,
    pub font_size: f64,
    pub subtitle_alignment: SubtitleAlignment,
    pub subtitle_blends_with_background: bool,
    pub overlay_locked: bool,
    pub overlay_frame: Option<OverlayFrame>,
    pub frame_layout_version: u64,
    /// UI language override. `None` and `Some("system")` both follow the
    /// operating-system language.
    pub ui_language: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            source_language: SourceLanguage::Automatic,
            target_language: TargetLanguage::SimplifiedChinese,
            translation_mode: TranslationMode::LowLatency,
            font_size: DEFAULT_FONT_SIZE,
            subtitle_alignment: SubtitleAlignment::Center,
            subtitle_blends_with_background: false,
            overlay_locked: false,
            overlay_frame: None,
            frame_layout_version: 0,
            ui_language: None,
        }
    }
}

/// Public, deliberately coarse credential state. OS/keyring error details are
/// never serialized to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialState {
    Present,
    Missing,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStoreError {
    Unavailable,
}

/// Small keyed abstraction over the OS credential store. Tests provide an
/// in-memory implementation; production uses `keyring`.
pub trait SecretStore: Send + Sync {
    fn load(&self, service: &str, account: &str) -> Result<Option<String>, SecretStoreError>;
    fn save(&self, service: &str, account: &str, value: &str) -> Result<(), SecretStoreError>;
    fn delete(&self, service: &str, account: &str) -> Result<(), SecretStoreError>;
}

struct KeyringSecretStore;

impl SecretStore for KeyringSecretStore {
    fn load(&self, service: &str, account: &str) -> Result<Option<String>, SecretStoreError> {
        let entry =
            keyring::Entry::new(service, account).map_err(|_| SecretStoreError::Unavailable)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(SecretStoreError::Unavailable),
        }
    }

    fn save(&self, service: &str, account: &str, value: &str) -> Result<(), SecretStoreError> {
        keyring::Entry::new(service, account)
            .map_err(|_| SecretStoreError::Unavailable)?
            .set_password(value)
            .map_err(|_| SecretStoreError::Unavailable)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), SecretStoreError> {
        let entry =
            keyring::Entry::new(service, account).map_err(|_| SecretStoreError::Unavailable)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretStoreError::Unavailable),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileCatalog {
    schema_version: u32,
    active_profile_id: String,
    profiles: Vec<ServiceProfile>,
}

impl Default for ProfileCatalog {
    fn default() -> Self {
        Self {
            schema_version: PROFILE_CATALOG_SCHEMA_VERSION,
            active_profile_id: DEFAULT_ALIBABA_PROFILE_ID.to_string(),
            profiles: vec![ServiceProfile::alibaba_default()],
        }
    }
}

impl ProfileCatalog {
    fn validated(self) -> Result<Self, ()> {
        if self.schema_version != PROFILE_CATALOG_SCHEMA_VERSION
            || self.profiles.is_empty()
            || self.profiles.len() > MAXIMUM_PROFILE_COUNT
        {
            return Err(());
        }

        let mut ids = HashSet::with_capacity(self.profiles.len());
        let mut profiles = Vec::with_capacity(self.profiles.len());
        for profile in self.profiles {
            let validated = profile.validated().map_err(|_| ())?;
            if !ids.insert(validated.id.clone()) {
                return Err(());
            }
            profiles.push(validated);
        }
        if !ids.contains(&self.active_profile_id) {
            return Err(());
        }

        Ok(Self {
            schema_version: self.schema_version,
            active_profile_id: self.active_profile_id,
            profiles,
        })
    }
}

type SecretCacheKey = (String, String);

pub struct SettingsStore {
    prefs_path: PathBuf,
    prefs: Mutex<Preferences>,
    catalog_path: PathBuf,
    catalog: Mutex<ProfileCatalog>,
    /// Invalid/unknown catalog data is never overwritten. The runtime keeps a
    /// safe default only so the rest of the app can remain operational.
    catalog_write_blocked: bool,
    secret: Box<dyn SecretStore>,
    profile_keychain_service: &'static str,
    migrate_legacy_alibaba: bool,
    /// Every first read is cached, including unavailable results. This keeps
    /// concurrent windows from triggering repeated OS authorization prompts.
    /// Explicit saves/deletes replace the cached state, and restart retries it.
    secret_cache: Mutex<HashMap<SecretCacheKey, Result<Option<String>, SecretStoreError>>>,
    is_ui_test: bool,
}

impl SettingsStore {
    /// Loads preferences and `service-profiles.json` from the app config
    /// directory. A missing catalog is created atomically with Alibaba as the
    /// active default; malformed data is preserved and all profile writes are
    /// blocked until it is repaired.
    pub fn load(app_config_dir: PathBuf, is_ui_test: bool, application_identifier: &str) -> Self {
        let is_development = application_identifier == DEVELOPMENT_APPLICATION_IDENTIFIER;
        let profile_keychain_service = if is_development {
            DEVELOPMENT_PROFILE_KEYCHAIN_SERVICE
        } else {
            PROFILE_KEYCHAIN_SERVICE
        };
        Self::load_with_secret(
            app_config_dir,
            is_ui_test,
            Box::new(KeyringSecretStore),
            profile_keychain_service,
            !is_development,
        )
    }

    fn load_with_secret(
        app_config_dir: PathBuf,
        is_ui_test: bool,
        secret: Box<dyn SecretStore>,
        profile_keychain_service: &'static str,
        migrate_legacy_alibaba: bool,
    ) -> Self {
        let prefs_path = app_config_dir.join("preferences.json");
        let prefs = std::fs::read_to_string(&prefs_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Preferences>(&text).ok())
            .unwrap_or_default();
        let font_size = prefs
            .font_size
            .clamp(*FONT_SIZE_RANGE.start(), *FONT_SIZE_RANGE.end());
        let prefs = Preferences { font_size, ..prefs };

        let catalog_path = app_config_dir.join(PROFILE_CATALOG_FILE);
        let (catalog, catalog_write_blocked, should_create_catalog) =
            match std::fs::read_to_string(&catalog_path) {
                Ok(text) => match serde_json::from_str::<ProfileCatalog>(&text)
                    .map_err(|_| ())
                    .and_then(ProfileCatalog::validated)
                {
                    Ok(catalog) => (catalog, false, false),
                    Err(()) => {
                        tracing::warn!("service profile catalog unavailable label=invalid_data");
                        (ProfileCatalog::default(), true, false)
                    }
                },
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    (ProfileCatalog::default(), false, true)
                }
                Err(_) => {
                    tracing::warn!("service profile catalog unavailable label=read_failed");
                    (ProfileCatalog::default(), true, false)
                }
            };

        let store = Self {
            prefs_path,
            prefs: Mutex::new(prefs),
            catalog_path,
            catalog: Mutex::new(catalog),
            catalog_write_blocked,
            secret,
            profile_keychain_service,
            migrate_legacy_alibaba,
            secret_cache: Mutex::new(HashMap::new()),
            is_ui_test,
        };
        if should_create_catalog && !is_ui_test && store.persist_catalog().is_err() {
            tracing::warn!("service profile catalog unavailable label=create_failed");
        }
        if !store.catalog_write_blocked {
            let provider = store
                .active_profile()
                .map(|profile| profile.provider)
                .unwrap_or(ProviderKind::AlibabaCloud);
            let mut prefs = store.prefs.lock().unwrap();
            let original = prefs.clone();
            normalize_preferences_value(&mut prefs, provider);
            if *prefs != original && store.persist_preferences_value(&prefs).is_err() {
                tracing::warn!("preferences unavailable label=normalization_write_failed");
            }
        }
        store
    }

    #[cfg(test)]
    pub fn in_memory(secret: Box<dyn SecretStore>, is_ui_test: bool) -> Self {
        Self::in_memory_with_scope(secret, is_ui_test, PROFILE_KEYCHAIN_SERVICE, true)
    }

    #[cfg(test)]
    fn in_memory_with_scope(
        secret: Box<dyn SecretStore>,
        is_ui_test: bool,
        profile_keychain_service: &'static str,
        migrate_legacy_alibaba: bool,
    ) -> Self {
        Self {
            prefs_path: PathBuf::new(),
            prefs: Mutex::new(Preferences::default()),
            catalog_path: PathBuf::new(),
            catalog: Mutex::new(ProfileCatalog::default()),
            catalog_write_blocked: false,
            secret,
            profile_keychain_service,
            migrate_legacy_alibaba,
            secret_cache: Mutex::new(HashMap::new()),
            is_ui_test,
        }
    }

    #[cfg(test)]
    fn at_path(app_config_dir: PathBuf, secret: Box<dyn SecretStore>) -> Self {
        Self::load_with_secret(
            app_config_dir,
            false,
            secret,
            PROFILE_KEYCHAIN_SERVICE,
            true,
        )
    }

    pub fn is_ui_test(&self) -> bool {
        self.is_ui_test
    }

    pub fn preferences(&self) -> Preferences {
        self.prefs.lock().unwrap().clone()
    }

    /// Applies and durably persists a preference update as one in-process
    /// transaction. A failed write leaves the published in-memory snapshot
    /// unchanged, so callers never report settings that will vanish on restart.
    pub fn save_preferences(&self, update: impl FnOnce(&mut Preferences)) -> Result<(), String> {
        let mut current = self.prefs.lock().unwrap();
        let mut next = current.clone();
        update(&mut next);
        next.font_size = next
            .font_size
            .clamp(*FONT_SIZE_RANGE.start(), *FONT_SIZE_RANGE.end());
        self.persist_preferences_value(&next)?;
        *current = next;
        Ok(())
    }

    /// Applies a preference update and the active provider's capability
    /// normalization before committing either change.
    pub fn save_preferences_for_active_profile(
        &self,
        update: impl FnOnce(&mut Preferences),
    ) -> Result<(), String> {
        let provider = self.active_profile()?.provider;
        self.save_preferences(|prefs| {
            update(prefs);
            normalize_preferences_value(prefs, provider);
        })
    }

    pub fn profile_catalog(&self) -> Result<(String, Vec<ServiceProfile>), String> {
        if self.catalog_write_blocked {
            return Err(PROFILE_CATALOG_UNAVAILABLE.to_string());
        }
        let catalog = self.catalog.lock().unwrap();
        Ok((catalog.active_profile_id.clone(), catalog.profiles.clone()))
    }

    /// Safe fallback used only for best-effort broadcasts after the initial
    /// `settings_get` has already reported a catalog error.
    pub fn profile_catalog_or_default(&self) -> (String, Vec<ServiceProfile>) {
        let catalog = self.catalog.lock().unwrap();
        (catalog.active_profile_id.clone(), catalog.profiles.clone())
    }

    pub fn active_profile(&self) -> Result<ServiceProfile, String> {
        let (active_id, profiles) = self.profile_catalog()?;
        profiles
            .into_iter()
            .find(|profile| profile.id == active_id)
            .ok_or_else(|| PROFILE_CATALOG_UNAVAILABLE.to_string())
    }

    pub fn create_profile(
        &self,
        provider: ProviderKind,
        name: &str,
    ) -> Result<ServiceProfile, String> {
        self.mutate_catalog(|catalog| {
            if catalog.profiles.len() >= MAXIMUM_PROFILE_COUNT {
                return Err("No more service profiles can be added.".to_string());
            }
            let profile = ServiceProfile::new(
                format!("profile-{}", uuid::Uuid::new_v4().simple()),
                name,
                provider,
            )
            .map_err(|error| error.to_string())?;
            catalog.profiles.push(profile.clone());
            Ok(profile)
        })
    }

    pub fn update_profile(&self, profile_id: &str, name: &str) -> Result<ServiceProfile, String> {
        self.mutate_catalog(|catalog| {
            let current = catalog
                .profiles
                .iter_mut()
                .find(|profile| profile.id == profile_id)
                .ok_or_else(|| PROFILE_NOT_FOUND.to_string())?;
            let updated = ServiceProfile::new(current.id.clone(), name, current.provider)
                .map_err(|error| error.to_string())?;
            *current = updated.clone();
            Ok(updated)
        })
    }

    pub fn select_profile(&self, profile_id: &str) -> Result<(), String> {
        if self.catalog_write_blocked {
            return Err(PROFILE_CATALOG_UNAVAILABLE.to_string());
        }

        let mut catalog = self.catalog.lock().unwrap();
        let mut next_catalog = catalog.clone();
        let provider = next_catalog
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .map(|profile| profile.provider)
            .ok_or_else(|| PROFILE_NOT_FOUND.to_string())?;
        next_catalog.active_profile_id = profile_id.to_string();
        next_catalog
            .clone()
            .validated()
            .map_err(|_| PROFILE_CATALOG_UNAVAILABLE.to_string())?;

        let mut prefs = self.prefs.lock().unwrap();
        let previous_prefs = prefs.clone();
        let mut next_prefs = previous_prefs.clone();
        normalize_preferences_value(&mut next_prefs, provider);

        self.persist_preferences_value(&next_prefs)?;
        if let Err(error) = self.persist_catalog_value(&next_catalog) {
            if self.persist_preferences_value(&previous_prefs).is_err() {
                tracing::warn!("preferences unavailable label=profile_select_rollback_failed");
            }
            return Err(error);
        }

        *prefs = next_prefs;
        *catalog = next_catalog;
        Ok(())
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<(), String> {
        if self.catalog_write_blocked {
            return Err(PROFILE_CATALOG_UNAVAILABLE.to_string());
        }
        let mut catalog = self.catalog.lock().unwrap();
        if catalog.profiles.len() == 1 {
            return Err(LAST_PROFILE.to_string());
        }
        let profile = catalog
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .cloned()
            .ok_or_else(|| PROFILE_NOT_FOUND.to_string())?;
        let previous_secret = self
            .load_api_key_for_profile(&profile)
            .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())?;

        let previous_catalog = catalog.clone();
        let mut next_catalog = previous_catalog.clone();
        next_catalog
            .profiles
            .retain(|candidate| candidate.id != profile_id);
        if next_catalog.active_profile_id == profile_id {
            next_catalog.active_profile_id = next_catalog.profiles[0].id.clone();
        }
        next_catalog
            .clone()
            .validated()
            .map_err(|_| PROFILE_CATALOG_UNAVAILABLE.to_string())?;

        let next_provider = next_catalog
            .profiles
            .iter()
            .find(|candidate| candidate.id == next_catalog.active_profile_id)
            .map(|candidate| candidate.provider)
            .ok_or_else(|| PROFILE_CATALOG_UNAVAILABLE.to_string())?;
        let mut prefs = self.prefs.lock().unwrap();
        let previous_prefs = prefs.clone();
        let mut next_prefs = previous_prefs.clone();
        normalize_preferences_value(&mut next_prefs, next_provider);

        // Persist the normalized preferences before selecting their provider.
        // The old provider also accepts the current providers' normalized
        // subset, and startup revalidates both documents after a crash.
        self.persist_preferences_value(&next_prefs)?;
        if self.delete_api_key_for_profile(&profile).is_err() {
            let secret_restored = previous_secret
                .as_deref()
                .map(|value| self.save_api_key_for_profile(&profile, value))
                .transpose()
                .is_ok();
            if self.persist_preferences_value(&previous_prefs).is_err() {
                tracing::warn!("preferences unavailable label=profile_delete_rollback_failed");
            }
            if !secret_restored {
                tracing::warn!("service profile delete rollback failed label=credential_restore");
            }
            return Err(CREDENTIAL_STORE_UNAVAILABLE.to_string());
        }
        if let Err(error) = self.persist_catalog_value(&next_catalog) {
            // Restore the credential when the public catalog cannot commit.
            // Deleting the secret first makes a process interruption leave a
            // visible profile with a missing key, never an unreachable secret.
            let secret_restored = previous_secret
                .as_deref()
                .map(|value| self.save_api_key_for_profile(&profile, value))
                .transpose()
                .is_ok();
            if self.persist_preferences_value(&previous_prefs).is_err() {
                tracing::warn!("preferences unavailable label=profile_delete_rollback_failed");
            }
            if !secret_restored {
                tracing::warn!("service profile delete rollback failed label=credential_restore");
            }
            return Err(error);
        }

        *catalog = next_catalog;
        *prefs = next_prefs;
        Ok(())
    }

    pub fn credential_state(&self, profile: &ServiceProfile) -> CredentialState {
        match self.load_api_key_for_profile(profile) {
            Ok(Some(value)) => {
                match ProviderCredentials::decode_from_keychain(profile.provider, &value) {
                    Ok(_) => CredentialState::Present,
                    Err(_) => CredentialState::Unavailable,
                }
            }
            Ok(None) => CredentialState::Missing,
            Err(_) => CredentialState::Unavailable,
        }
    }

    #[cfg(test)]
    fn load_api_key(&self) -> Result<Option<String>, String> {
        let profile = self.active_profile()?;
        self.load_api_key_for_profile(&profile)
            .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())
    }

    pub fn save_api_key(&self, profile_id: &str, api_key: &str) -> Result<(), String> {
        self.save_credentials(profile_id, &ProviderCredentials::api_key(api_key))
    }

    pub fn save_credentials(
        &self,
        profile_id: &str,
        credentials: &ProviderCredentials,
    ) -> Result<(), String> {
        let profile = self.profile(profile_id)?;
        let value = credentials
            .encode_for_keychain(profile.provider)
            .map_err(|error| error.to_string())?;
        self.save_api_key_for_profile(&profile, &value)
    }

    pub fn delete_api_key(&self, profile_id: &str) -> Result<(), String> {
        let profile = self.profile(profile_id)?;
        self.delete_api_key_for_profile(&profile)
    }

    /// The validated configuration used to start a session. The provider is
    /// resolved natively from the active profile; credentials never cross IPC.
    pub fn configuration(&self) -> Result<LiveTranslationConfiguration, String> {
        let prefs = self.prefs.lock().unwrap().clone();
        let profile = self.active_profile()?;
        let api_key = self
            .load_api_key_for_profile(&profile)
            .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())?
            .unwrap_or_default();
        let credentials = ProviderCredentials::decode_from_keychain(profile.provider, &api_key)
            .map_err(|error| error.to_string())?;
        LiveTranslationConfiguration::with_credentials(
            profile.provider,
            credentials,
            prefs.source_language,
            prefs.target_language,
            prefs.translation_mode,
        )
        .validated()
        .map_err(|error| error.to_string())
    }

    /// Applies listening-time constraints without changing profile metadata.
    pub fn prepare_for_listening(&self) -> Result<(), String> {
        let provider = self
            .active_profile()
            .map(|profile| profile.provider)
            .unwrap_or(ProviderKind::AlibabaCloud);
        self.save_preferences(|prefs| normalize_preferences_value(prefs, provider))
    }

    fn persist_preferences_value(&self, prefs: &Preferences) -> Result<(), String> {
        if self.is_ui_test || self.prefs_path.as_os_str().is_empty() {
            return Ok(());
        }
        let bytes =
            serde_json::to_vec_pretty(prefs).map_err(|_| PREFERENCES_UNAVAILABLE.to_string())?;
        atomic_write(&self.prefs_path, &bytes).map_err(|_| PREFERENCES_UNAVAILABLE.to_string())
    }

    fn profile(&self, profile_id: &str) -> Result<ServiceProfile, String> {
        let (_, profiles) = self.profile_catalog()?;
        profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| PROFILE_NOT_FOUND.to_string())
    }

    fn mutate_catalog<T>(
        &self,
        update: impl FnOnce(&mut ProfileCatalog) -> Result<T, String>,
    ) -> Result<T, String> {
        if self.catalog_write_blocked {
            return Err(PROFILE_CATALOG_UNAVAILABLE.to_string());
        }
        let mut catalog = self.catalog.lock().unwrap();
        let mut next = catalog.clone();
        let result = update(&mut next)?;
        next.clone()
            .validated()
            .map_err(|_| PROFILE_CATALOG_UNAVAILABLE.to_string())?;
        self.persist_catalog_value(&next)?;
        *catalog = next;
        Ok(result)
    }

    fn persist_catalog(&self) -> Result<(), String> {
        let catalog = self.catalog.lock().unwrap().clone();
        self.persist_catalog_value(&catalog)
    }

    fn persist_catalog_value(&self, catalog: &ProfileCatalog) -> Result<(), String> {
        if self.catalog_write_blocked {
            return Err(PROFILE_CATALOG_UNAVAILABLE.to_string());
        }
        if self.is_ui_test || self.catalog_path.as_os_str().is_empty() {
            return Ok(());
        }
        let bytes = serde_json::to_vec_pretty(catalog)
            .map_err(|_| PROFILE_CATALOG_UNAVAILABLE.to_string())?;
        atomic_write(&self.catalog_path, &bytes)
            .map_err(|_| PROFILE_CATALOG_UNAVAILABLE.to_string())
    }

    fn load_api_key_for_profile(
        &self,
        profile: &ServiceProfile,
    ) -> Result<Option<String>, SecretStoreError> {
        let account = credential_account(profile);
        let destination = self.load_secret(self.profile_keychain_service, &account)?;
        if let Some(value) = destination {
            if value.trim().is_empty() {
                return Err(SecretStoreError::Unavailable);
            }
            // A profile-scoped credential is already authoritative. Do not
            // touch the separate migration marker on the normal read path:
            // macOS authorizes Keychain items independently, so reading both
            // accounts can produce two password prompts after an intentional
            // signing-identity migration. Save, delete, and actual legacy
            // migration still persist the marker before it matters.
            return Ok(Some(value));
        }
        if !is_default_alibaba(profile) || !self.migrate_legacy_alibaba {
            return Ok(None);
        }

        match self.load_secret(
            self.profile_keychain_service,
            LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
        )? {
            Some(value) if value == LEGACY_MIGRATION_TOMBSTONE_VALUE => return Ok(None),
            Some(_) => return Err(SecretStoreError::Unavailable),
            None => {}
        }

        let mut saw_blank_legacy_value = false;
        for service in [
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_SERVICE_V2,
            LEGACY_KEYCHAIN_SERVICE,
        ] {
            let Some(legacy) = self.load_secret(service, LEGACY_KEYCHAIN_ACCOUNT)? else {
                continue;
            };
            if legacy.trim().is_empty() {
                saw_blank_legacy_value = true;
                continue;
            }

            self.save_secret(self.profile_keychain_service, &account, &legacy)?;
            let verified = self.load_secret_uncached(self.profile_keychain_service, &account)?;
            if verified.as_deref() != Some(legacy.as_str()) {
                self.secret_cache
                    .lock()
                    .unwrap()
                    .remove(&cache_key(self.profile_keychain_service, &account));
                return Err(SecretStoreError::Unavailable);
            }
            self.cache_secret(self.profile_keychain_service, &account, verified);
            if self.write_legacy_tombstone().is_err() {
                tracing::warn!("credential migration deferred label=tombstone_write_failed");
            }
            return Ok(Some(legacy));
        }

        if saw_blank_legacy_value {
            return Err(SecretStoreError::Unavailable);
        }

        // An empty scan is not a migration. Leave it retryable so a credential
        // created by an older installed version can still be imported later.
        Ok(None)
    }

    fn delete_api_key_for_profile(&self, profile: &ServiceProfile) -> Result<(), String> {
        if is_default_alibaba(profile) && self.migrate_legacy_alibaba {
            // The tombstone must be durable before deletion so a missing new
            // slot can never revive an older single-slot Alibaba credential.
            self.write_legacy_tombstone()
                .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
            // Explicit removal is also the point where rollback-era single
            // slots are retired. Keep the current profile credential intact
            // unless every legacy deletion succeeds.
            self.delete_legacy_alibaba_credentials()
                .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        }
        let account = credential_account(profile);
        self.delete_secret(self.profile_keychain_service, &account)
            .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())
    }

    fn delete_legacy_alibaba_credentials(&self) -> Result<(), SecretStoreError> {
        for service in [
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_SERVICE_V2,
            LEGACY_KEYCHAIN_SERVICE,
        ] {
            self.delete_secret(service, LEGACY_KEYCHAIN_ACCOUNT)?;
        }
        Ok(())
    }

    fn save_api_key_for_profile(
        &self,
        profile: &ServiceProfile,
        value: &str,
    ) -> Result<(), String> {
        let account = credential_account(profile);
        self.save_secret(self.profile_keychain_service, &account, value)
            .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        let verified = self
            .load_secret_uncached(self.profile_keychain_service, &account)
            .map_err(|_| CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        if verified.as_deref() != Some(value) {
            self.secret_cache
                .lock()
                .unwrap()
                .remove(&cache_key(self.profile_keychain_service, &account));
            return Err(CREDENTIAL_STORE_UNAVAILABLE.to_string());
        }
        self.cache_secret(self.profile_keychain_service, &account, verified);

        if is_default_alibaba(profile)
            && self.migrate_legacy_alibaba
            && self.write_legacy_tombstone().is_err()
        {
            // Saving and verifying the profile slot is the user-visible
            // transaction. A migration marker failure is retryable and must
            // not turn a successful key replacement into an error.
            tracing::warn!("credential migration deferred label=tombstone_write_failed");
        }
        Ok(())
    }

    fn write_legacy_tombstone(&self) -> Result<(), SecretStoreError> {
        match self.load_secret(
            self.profile_keychain_service,
            LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
        )? {
            Some(value) if value == LEGACY_MIGRATION_TOMBSTONE_VALUE => return Ok(()),
            Some(_) => return Err(SecretStoreError::Unavailable),
            None => {}
        }
        self.save_secret(
            self.profile_keychain_service,
            LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
            LEGACY_MIGRATION_TOMBSTONE_VALUE,
        )?;
        let verified = self.load_secret_uncached(
            self.profile_keychain_service,
            LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
        )?;
        if verified.as_deref() != Some(LEGACY_MIGRATION_TOMBSTONE_VALUE) {
            self.secret_cache.lock().unwrap().remove(&cache_key(
                self.profile_keychain_service,
                LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
            ));
            return Err(SecretStoreError::Unavailable);
        }
        self.cache_secret(
            self.profile_keychain_service,
            LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
            verified,
        );
        Ok(())
    }

    fn load_secret(
        &self,
        service: &str,
        account: &str,
    ) -> Result<Option<String>, SecretStoreError> {
        let key = cache_key(service, account);
        let mut cache = self.secret_cache.lock().unwrap();
        if let Some(value) = cache.get(&key) {
            return value.clone();
        }
        if self.is_ui_test {
            let value = if service == self.profile_keychain_service
                && account == credential_account(&ServiceProfile::alibaba_default())
            {
                Some("sk-demo-not-a-real-key".to_string())
            } else {
                None
            };
            cache.insert(key, Ok(value.clone()));
            return Ok(value);
        }
        let result = self.secret.load(service, account);
        cache.insert(key, result.clone());
        result
    }

    fn load_secret_uncached(
        &self,
        service: &str,
        account: &str,
    ) -> Result<Option<String>, SecretStoreError> {
        if self.is_ui_test {
            return self
                .secret_cache
                .lock()
                .unwrap()
                .get(&cache_key(service, account))
                .cloned()
                .unwrap_or(Ok(None));
        }
        let result = self.secret.load(service, account);
        self.secret_cache
            .lock()
            .unwrap()
            .insert(cache_key(service, account), result.clone());
        result
    }

    fn save_secret(
        &self,
        service: &str,
        account: &str,
        value: &str,
    ) -> Result<(), SecretStoreError> {
        if !self.is_ui_test {
            if let Err(error) = self.secret.save(service, account, value) {
                self.secret_cache
                    .lock()
                    .unwrap()
                    .insert(cache_key(service, account), Err(error));
                return Err(error);
            }
        }
        self.cache_secret(service, account, Some(value.to_string()));
        Ok(())
    }

    fn delete_secret(&self, service: &str, account: &str) -> Result<(), SecretStoreError> {
        if !self.is_ui_test {
            if let Err(error) = self.secret.delete(service, account) {
                self.secret_cache
                    .lock()
                    .unwrap()
                    .insert(cache_key(service, account), Err(error));
                return Err(error);
            }
        }
        self.cache_secret(service, account, None);
        Ok(())
    }

    fn cache_secret(&self, service: &str, account: &str, value: Option<String>) {
        self.secret_cache
            .lock()
            .unwrap()
            .insert(cache_key(service, account), Ok(value));
    }
}

fn credential_account(profile: &ServiceProfile) -> String {
    format!(
        "provider-profile:{}:{}:api-key",
        profile.id,
        profile.provider.wire_value()
    )
}

fn is_default_alibaba(profile: &ServiceProfile) -> bool {
    profile.id == DEFAULT_ALIBABA_PROFILE_ID && profile.provider == ProviderKind::AlibabaCloud
}

fn normalize_preferences_value(prefs: &mut Preferences, provider: ProviderKind) {
    let capabilities = provider.capabilities();
    let normalized = capabilities.normalize(ProviderPreferences {
        source_language: prefs.source_language,
        target_language: prefs.target_language,
        translation_mode: prefs.translation_mode,
    });
    prefs.source_language = normalized.source_language;
    prefs.target_language = normalized.target_language;
    prefs.translation_mode = normalized.translation_mode;
    if prefs.source_language == SourceLanguage::Chinese
        && capabilities
            .target_languages
            .contains(&TargetLanguage::Original)
    {
        prefs.target_language = TargetLanguage::Original;
    }
}

fn cache_key(service: &str, account: &str) -> SecretCacheKey {
    (service.to_string(), account.to_string())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(ErrorKind::InvalidInput, "catalog path has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(PROFILE_CATALOG_FILE);
    let temporary = parent.join(format!(
        ".{file_name}.{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        replace_file(&temporary, path)?;
        sync_directory(parent);
        Ok(())
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    write_result
}

#[cfg(unix)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(once(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(once(0))
        .collect();
    // SAFETY: both arguments are valid, NUL-terminated UTF-16 buffers for the
    // duration of the call. The flags request same-volume atomic replacement.
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(unix)]
fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Default)]
    struct FakeState {
        values: HashMap<SecretCacheKey, String>,
        unavailable: HashSet<SecretCacheKey>,
        unavailable_deletes: HashSet<SecretCacheKey>,
        loads: Vec<SecretCacheKey>,
    }

    #[derive(Clone, Default)]
    struct FakeSecretStore {
        state: Arc<Mutex<FakeState>>,
    }

    impl FakeSecretStore {
        fn put(&self, service: &str, account: &str, value: &str) {
            self.state
                .lock()
                .unwrap()
                .values
                .insert(cache_key(service, account), value.to_string());
        }

        fn value(&self, service: &str, account: &str) -> Option<String> {
            self.state
                .lock()
                .unwrap()
                .values
                .get(&cache_key(service, account))
                .cloned()
        }

        fn make_unavailable(&self, service: &str, account: &str) {
            self.state
                .lock()
                .unwrap()
                .unavailable
                .insert(cache_key(service, account));
        }

        fn make_delete_unavailable(&self, service: &str, account: &str) {
            self.state
                .lock()
                .unwrap()
                .unavailable_deletes
                .insert(cache_key(service, account));
        }

        fn load_count(&self, service: &str, account: &str) -> usize {
            self.state
                .lock()
                .unwrap()
                .loads
                .iter()
                .filter(|candidate| **candidate == cache_key(service, account))
                .count()
        }
    }

    impl SecretStore for FakeSecretStore {
        fn load(&self, service: &str, account: &str) -> Result<Option<String>, SecretStoreError> {
            let key = cache_key(service, account);
            let mut state = self.state.lock().unwrap();
            state.loads.push(key.clone());
            if state.unavailable.contains(&key) {
                return Err(SecretStoreError::Unavailable);
            }
            Ok(state.values.get(&key).cloned())
        }

        fn save(&self, service: &str, account: &str, value: &str) -> Result<(), SecretStoreError> {
            let key = cache_key(service, account);
            let mut state = self.state.lock().unwrap();
            if state.unavailable.contains(&key) {
                return Err(SecretStoreError::Unavailable);
            }
            state.values.insert(key, value.to_string());
            Ok(())
        }

        fn delete(&self, service: &str, account: &str) -> Result<(), SecretStoreError> {
            let key = cache_key(service, account);
            let mut state = self.state.lock().unwrap();
            if state.unavailable.contains(&key) || state.unavailable_deletes.contains(&key) {
                return Err(SecretStoreError::Unavailable);
            }
            state.values.remove(&key);
            Ok(())
        }
    }

    fn settings(fake: &FakeSecretStore) -> SettingsStore {
        SettingsStore::in_memory(Box::new(fake.clone()), false)
    }

    fn openai_profile(store: &SettingsStore, name: &str) -> ServiceProfile {
        store
            .create_profile(ProviderKind::OpenAIRealtime, name)
            .unwrap()
    }

    #[test]
    fn credentials_are_isolated_by_profile_and_provider() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        let first = openai_profile(&store, "Work");
        let second = openai_profile(&store, "Personal");

        store.save_api_key(&first.id, "sk-first").unwrap();
        store.save_api_key(&second.id, "sk-second").unwrap();

        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &credential_account(&first))
                .as_deref(),
            Some("sk-first")
        );
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &credential_account(&second))
                .as_deref(),
            Some("sk-second")
        );
        assert_ne!(credential_account(&first), credential_account(&second));
    }

    #[test]
    fn creating_a_profile_never_copies_the_active_secret() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        store
            .save_api_key(DEFAULT_ALIBABA_PROFILE_ID, "sk-alibaba")
            .unwrap();
        let profile = openai_profile(&store, "OpenAI");

        assert_eq!(store.credential_state(&profile), CredentialState::Missing);
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &credential_account(&profile)),
            None
        );
    }

    #[test]
    fn development_scope_does_not_read_or_migrate_release_credentials() {
        let fake = FakeSecretStore::default();
        fake.put(
            PROFILE_KEYCHAIN_SERVICE,
            &credential_account(&ServiceProfile::alibaba_default()),
            "sk-release-profile",
        );
        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-release-legacy",
        );
        let store = SettingsStore::in_memory_with_scope(
            Box::new(fake.clone()),
            false,
            DEVELOPMENT_PROFILE_KEYCHAIN_SERVICE,
            false,
        );

        assert_eq!(store.load_api_key().unwrap(), None);
        store
            .save_api_key(DEFAULT_ALIBABA_PROFILE_ID, "sk-development")
            .unwrap();
        assert_eq!(
            fake.value(
                DEVELOPMENT_PROFILE_KEYCHAIN_SERVICE,
                &credential_account(&ServiceProfile::alibaba_default())
            )
            .as_deref(),
            Some("sk-development")
        );
        assert_eq!(
            fake.value(
                PROFILE_KEYCHAIN_SERVICE,
                &credential_account(&ServiceProfile::alibaba_default())
            )
            .as_deref(),
            Some("sk-release-profile")
        );
    }

    #[test]
    fn legacy_key_migrates_only_to_default_alibaba_and_writes_tombstone() {
        let fake = FakeSecretStore::default();
        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-legacy",
        );
        let store = settings(&fake);
        let openai = openai_profile(&store, "OpenAI");

        assert_eq!(store.credential_state(&openai), CredentialState::Missing);
        assert_eq!(store.load_api_key().unwrap().as_deref(), Some("sk-legacy"));
        assert_eq!(
            fake.value(
                PROFILE_KEYCHAIN_SERVICE,
                &credential_account(&ServiceProfile::alibaba_default())
            )
            .as_deref(),
            Some("sk-legacy")
        );
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, LEGACY_MIGRATION_TOMBSTONE_ACCOUNT)
                .as_deref(),
            Some(LEGACY_MIGRATION_TOMBSTONE_VALUE)
        );
    }

    #[test]
    fn tombstone_failure_does_not_turn_a_verified_key_save_into_an_error() {
        let fake = FakeSecretStore::default();
        fake.make_unavailable(PROFILE_KEYCHAIN_SERVICE, LEGACY_MIGRATION_TOMBSTONE_ACCOUNT);
        let default = ServiceProfile::alibaba_default();
        let store = settings(&fake);

        store
            .save_api_key(DEFAULT_ALIBABA_PROFILE_ID, "sk-replacement")
            .unwrap();
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &credential_account(&default))
                .as_deref(),
            Some("sk-replacement")
        );
        assert_eq!(
            store.load_api_key().unwrap().as_deref(),
            Some("sk-replacement")
        );

        // A fresh process reads only the authoritative profile slot. Explicit
        // deletion remains fail-closed because it must persist the marker
        // before removing the key.
        let restarted = settings(&fake);
        assert_eq!(
            restarted.load_api_key().unwrap().as_deref(),
            Some("sk-replacement")
        );
        assert_eq!(
            fake.load_count(PROFILE_KEYCHAIN_SERVICE, LEGACY_MIGRATION_TOMBSTONE_ACCOUNT),
            1
        );
        assert_eq!(
            restarted
                .delete_api_key(DEFAULT_ALIBABA_PROFILE_ID)
                .unwrap_err(),
            CREDENTIAL_STORE_UNAVAILABLE
        );
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &credential_account(&default))
                .as_deref(),
            Some("sk-replacement")
        );
    }

    #[test]
    fn existing_profile_credential_does_not_read_migration_items() {
        let fake = FakeSecretStore::default();
        let default = ServiceProfile::alibaba_default();
        fake.put(
            PROFILE_KEYCHAIN_SERVICE,
            &credential_account(&default),
            "sk-profile",
        );
        fake.put(
            PROFILE_KEYCHAIN_SERVICE,
            LEGACY_MIGRATION_TOMBSTONE_ACCOUNT,
            LEGACY_MIGRATION_TOMBSTONE_VALUE,
        );
        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-legacy",
        );
        let store = settings(&fake);

        assert_eq!(store.credential_state(&default), CredentialState::Present);
        assert_eq!(store.load_api_key().unwrap().as_deref(), Some("sk-profile"));
        assert_eq!(
            fake.load_count(PROFILE_KEYCHAIN_SERVICE, &credential_account(&default)),
            1
        );
        assert_eq!(
            fake.load_count(PROFILE_KEYCHAIN_SERVICE, LEGACY_MIGRATION_TOMBSTONE_ACCOUNT),
            0
        );
        assert_eq!(
            fake.load_count(LEGACY_KEYCHAIN_SERVICE_V3, LEGACY_KEYCHAIN_ACCOUNT),
            0
        );
    }

    #[test]
    fn explicit_deletion_prevents_legacy_credential_resurrection() {
        let fake = FakeSecretStore::default();
        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-legacy",
        );
        let store = settings(&fake);
        assert_eq!(store.load_api_key().unwrap().as_deref(), Some("sk-legacy"));
        store.delete_api_key(DEFAULT_ALIBABA_PROFILE_ID).unwrap();

        assert_eq!(
            fake.value(LEGACY_KEYCHAIN_SERVICE_V3, LEGACY_KEYCHAIN_ACCOUNT),
            None
        );

        // A new process has an empty cache but sees the durable tombstone.
        let restarted = settings(&fake);
        assert_eq!(restarted.load_api_key().unwrap(), None);
        assert_eq!(
            fake.load_count(LEGACY_KEYCHAIN_SERVICE_V3, LEGACY_KEYCHAIN_ACCOUNT),
            1
        );
    }

    #[test]
    fn failed_legacy_cleanup_keeps_the_current_default_credential() {
        let fake = FakeSecretStore::default();
        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-legacy",
        );
        let store = settings(&fake);
        assert_eq!(store.load_api_key().unwrap().as_deref(), Some("sk-legacy"));
        fake.make_delete_unavailable(LEGACY_KEYCHAIN_SERVICE_V3, LEGACY_KEYCHAIN_ACCOUNT);

        assert_eq!(
            store
                .delete_api_key(DEFAULT_ALIBABA_PROFILE_ID)
                .unwrap_err(),
            CREDENTIAL_STORE_UNAVAILABLE
        );
        assert_eq!(store.load_api_key().unwrap().as_deref(), Some("sk-legacy"));
    }

    #[test]
    fn missing_legacy_credential_does_not_suppress_a_future_import() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        assert_eq!(store.load_api_key().unwrap(), None);
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, LEGACY_MIGRATION_TOMBSTONE_ACCOUNT)
                .as_deref(),
            None
        );

        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V3,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-created-by-older-version",
        );
        let restarted = settings(&fake);
        assert_eq!(
            restarted.load_api_key().unwrap().as_deref(),
            Some("sk-created-by-older-version")
        );
    }

    #[test]
    fn blank_legacy_slot_does_not_hide_a_later_valid_credential() {
        let fake = FakeSecretStore::default();
        fake.put(LEGACY_KEYCHAIN_SERVICE_V3, LEGACY_KEYCHAIN_ACCOUNT, "   ");
        fake.put(
            LEGACY_KEYCHAIN_SERVICE_V2,
            LEGACY_KEYCHAIN_ACCOUNT,
            "sk-valid",
        );
        let store = settings(&fake);

        assert_eq!(store.load_api_key().unwrap().as_deref(), Some("sk-valid"));
    }

    #[test]
    fn only_blank_legacy_values_fail_closed_without_a_tombstone() {
        let fake = FakeSecretStore::default();
        fake.put(LEGACY_KEYCHAIN_SERVICE_V3, LEGACY_KEYCHAIN_ACCOUNT, "   ");
        let store = settings(&fake);

        assert_eq!(
            store.load_api_key().unwrap_err(),
            CREDENTIAL_STORE_UNAVAILABLE
        );
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, LEGACY_MIGRATION_TOMBSTONE_ACCOUNT),
            None
        );
    }

    #[test]
    fn unavailable_credential_store_is_not_reported_as_missing() {
        let fake = FakeSecretStore::default();
        let default = ServiceProfile::alibaba_default();
        fake.make_unavailable(PROFILE_KEYCHAIN_SERVICE, &credential_account(&default));
        let store = settings(&fake);

        assert_eq!(
            store.credential_state(&default),
            CredentialState::Unavailable
        );
        assert_eq!(
            store.load_api_key().unwrap_err(),
            CREDENTIAL_STORE_UNAVAILABLE
        );
        // All windows share the first failed read, avoiding repeated OS
        // authorization prompts for the same slot during one app launch.
        assert_eq!(
            fake.load_count(PROFILE_KEYCHAIN_SERVICE, &credential_account(&default)),
            1
        );
    }

    #[test]
    fn profile_crud_preserves_catalog_invariants() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        let created = openai_profile(&store, "  Work  ");
        assert_eq!(created.name, "Work");
        store.update_profile(&created.id, "Production").unwrap();
        store.select_profile(&created.id).unwrap();
        assert_eq!(store.active_profile().unwrap().id, created.id);

        store.delete_profile(DEFAULT_ALIBABA_PROFILE_ID).unwrap();
        assert_eq!(store.profile_catalog().unwrap().1.len(), 1);
        assert_eq!(store.delete_profile(&created.id).unwrap_err(), LAST_PROFILE);
    }

    #[test]
    fn deleting_a_profile_without_a_saved_credential_is_idempotent() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        let profile = openai_profile(&store, "No credential");

        store.delete_profile(&profile.id).unwrap();

        assert!(store
            .profile_catalog()
            .unwrap()
            .1
            .iter()
            .all(|candidate| candidate.id != profile.id));
    }

    #[test]
    fn structured_credentials_remain_keychain_only_and_resolve_configuration() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        let profile = store
            .create_profile(ProviderKind::AzureOpenAIRealtime, "Azure")
            .unwrap();
        let secret = "azure-private-value";
        let credentials = ProviderCredentials::AzureOpenAI {
            endpoint: "https://mimi.openai.azure.com".into(),
            deployment: "translate".into(),
            transcription_deployment: "transcribe".into(),
            api_key: secret.into(),
        };

        store.save_credentials(&profile.id, &credentials).unwrap();
        assert_eq!(store.credential_state(&profile), CredentialState::Present);
        store.select_profile(&profile.id).unwrap();
        let configuration = store.configuration().unwrap();
        assert_eq!(configuration.credentials, credentials);

        let catalog = serde_json::to_string(&store.profile_catalog().unwrap()).unwrap();
        assert!(!catalog.contains(secret));
        assert!(!catalog.contains("mimi.openai.azure.com"));
    }

    #[test]
    fn profile_count_is_bounded() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        for index in 1..MAXIMUM_PROFILE_COUNT {
            store
                .create_profile(ProviderKind::OpenAIRealtime, &format!("Profile {index}"))
                .unwrap();
        }
        assert!(store
            .create_profile(ProviderKind::OpenAIRealtime, "One too many")
            .is_err());
        assert_eq!(
            store.profile_catalog().unwrap().1.len(),
            MAXIMUM_PROFILE_COUNT
        );
    }

    #[test]
    fn failed_credential_delete_rolls_profile_metadata_back() {
        let fake = FakeSecretStore::default();
        let store = settings(&fake);
        let profile = openai_profile(&store, "Work");
        store.save_api_key(&profile.id, "sk-work").unwrap();
        let account = credential_account(&profile);
        fake.make_delete_unavailable(PROFILE_KEYCHAIN_SERVICE, &account);

        assert_eq!(
            store.delete_profile(&profile.id).unwrap_err(),
            CREDENTIAL_STORE_UNAVAILABLE
        );
        assert!(store
            .profile_catalog()
            .unwrap()
            .1
            .iter()
            .any(|candidate| candidate.id == profile.id));
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &account).as_deref(),
            Some("sk-work")
        );
    }

    #[test]
    fn failed_catalog_commit_does_not_delete_a_profile_credential() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-profile-commit-failure-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let store = SettingsStore::at_path(directory.clone(), Box::new(fake.clone()));
        let profile = store
            .create_profile(ProviderKind::OpenAIRealtime, "Work")
            .unwrap();
        store.save_api_key(&profile.id, "sk-work").unwrap();
        let account = credential_account(&profile);

        let catalog_path = directory.join(PROFILE_CATALOG_FILE);
        std::fs::remove_file(&catalog_path).unwrap();
        std::fs::create_dir(&catalog_path).unwrap();

        assert_eq!(
            store.delete_profile(&profile.id).unwrap_err(),
            PROFILE_CATALOG_UNAVAILABLE
        );
        assert!(store
            .profile_catalog()
            .unwrap()
            .1
            .iter()
            .any(|candidate| candidate.id == profile.id));
        assert_eq!(
            fake.value(PROFILE_KEYCHAIN_SERVICE, &account).as_deref(),
            Some("sk-work")
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn catalog_is_atomic_json_and_contains_no_secret() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-profile-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let store = SettingsStore::at_path(directory.clone(), Box::new(fake));
        let profile = store
            .create_profile(ProviderKind::OpenAIRealtime, "Work")
            .unwrap();
        store
            .save_api_key(&profile.id, "super-secret-value")
            .unwrap();

        let catalog_path = directory.join(PROFILE_CATALOG_FILE);
        let text = std::fs::read_to_string(&catalog_path).unwrap();
        let catalog: ProfileCatalog = serde_json::from_str(&text).unwrap();
        assert_eq!(catalog.schema_version, PROFILE_CATALOG_SCHEMA_VERSION);
        assert!(!text.contains("super-secret-value"));
        assert!(!text.contains("apiKey"));
        assert!(std::fs::read_dir(&directory).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn failed_preference_write_leaves_memory_unchanged() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-preferences-failure-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let store = SettingsStore::at_path(directory.clone(), Box::new(fake));
        let before = store.preferences();

        std::fs::remove_file(directory.join(PROFILE_CATALOG_FILE)).unwrap();
        std::fs::remove_dir(&directory).unwrap();
        std::fs::write(&directory, "not a directory").unwrap();

        assert_eq!(
            store
                .save_preferences(|prefs| prefs.font_size = 20.0)
                .unwrap_err(),
            PREFERENCES_UNAVAILABLE
        );
        assert_eq!(store.preferences(), before);

        let _ = std::fs::remove_file(directory);
    }

    #[test]
    fn legacy_preferences_default_to_centered_card_presentation() {
        let preferences: Preferences = serde_json::from_str("{}").unwrap();

        assert_eq!(preferences.subtitle_alignment, SubtitleAlignment::Center);
        assert!(!preferences.subtitle_blends_with_background);
    }

    #[test]
    fn loading_normalizes_preferences_for_the_active_provider() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-preferences-normalize-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        let openai = ServiceProfile::new("openai", "OpenAI", ProviderKind::OpenAIRealtime).unwrap();
        let catalog = ProfileCatalog {
            schema_version: PROFILE_CATALOG_SCHEMA_VERSION,
            active_profile_id: openai.id.clone(),
            profiles: vec![ServiceProfile::alibaba_default(), openai.clone()],
        };
        std::fs::write(
            directory.join(PROFILE_CATALOG_FILE),
            serde_json::to_vec_pretty(&catalog).unwrap(),
        )
        .unwrap();
        let stale = Preferences {
            source_language: SourceLanguage::Japanese,
            target_language: TargetLanguage::Original,
            translation_mode: TranslationMode::HighQuality,
            ..Preferences::default()
        };
        std::fs::write(
            directory.join("preferences.json"),
            serde_json::to_vec_pretty(&stale).unwrap(),
        )
        .unwrap();
        fake.put(
            PROFILE_KEYCHAIN_SERVICE,
            &credential_account(&openai),
            "sk-openai-test",
        );

        let store = SettingsStore::at_path(directory.clone(), Box::new(fake));
        let normalized = store.preferences();
        assert_eq!(normalized.source_language, SourceLanguage::Automatic);
        assert_eq!(
            normalized.target_language,
            TargetLanguage::SimplifiedChinese
        );
        assert_eq!(normalized.translation_mode, TranslationMode::Turbo);
        assert_eq!(
            store.configuration().unwrap().provider,
            ProviderKind::OpenAIRealtime
        );

        let persisted: Preferences =
            serde_json::from_slice(&std::fs::read(directory.join("preferences.json")).unwrap())
                .unwrap();
        assert_eq!(persisted, normalized);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn loading_persists_chinese_source_with_original_target() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-preferences-chinese-original-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let stale = Preferences {
            source_language: SourceLanguage::Chinese,
            target_language: TargetLanguage::English,
            ..Preferences::default()
        };
        std::fs::write(
            directory.join("preferences.json"),
            serde_json::to_vec_pretty(&stale).unwrap(),
        )
        .unwrap();

        let store = SettingsStore::at_path(directory.clone(), Box::new(fake));
        let normalized = store.preferences();
        assert_eq!(normalized.source_language, SourceLanguage::Chinese);
        assert_eq!(normalized.target_language, TargetLanguage::Original);

        let persisted: Preferences =
            serde_json::from_slice(&std::fs::read(directory.join("preferences.json")).unwrap())
                .unwrap();
        assert_eq!(persisted, normalized);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn explicit_provider_preserves_chinese_translation_target() {
        let mut preferences = Preferences {
            source_language: SourceLanguage::Chinese,
            target_language: TargetLanguage::English,
            ..Preferences::default()
        };

        normalize_preferences_value(&mut preferences, ProviderKind::TencentCloud);
        assert_eq!(preferences.source_language, SourceLanguage::Chinese);
        assert_eq!(preferences.target_language, TargetLanguage::English);

        preferences.target_language = TargetLanguage::SimplifiedChinese;
        normalize_preferences_value(&mut preferences, ProviderKind::TencentCloud);
        assert_eq!(preferences.target_language, TargetLanguage::English);
    }

    #[test]
    fn failed_profile_selection_rolls_preferences_back() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-profile-selection-rollback-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let store = SettingsStore::at_path(directory.clone(), Box::new(fake));
        let openai = store
            .create_profile(ProviderKind::OpenAIRealtime, "OpenAI")
            .unwrap();
        store
            .save_preferences(|prefs| {
                prefs.source_language = SourceLanguage::Japanese;
                prefs.target_language = TargetLanguage::Original;
                prefs.translation_mode = TranslationMode::HighQuality;
            })
            .unwrap();
        let before = store.preferences();

        let catalog_path = directory.join(PROFILE_CATALOG_FILE);
        std::fs::remove_file(&catalog_path).unwrap();
        std::fs::create_dir(&catalog_path).unwrap();

        assert_eq!(
            store.select_profile(&openai.id).unwrap_err(),
            PROFILE_CATALOG_UNAVAILABLE
        );
        assert_eq!(store.preferences(), before);
        assert_eq!(
            store.active_profile().unwrap().id,
            DEFAULT_ALIBABA_PROFILE_ID
        );
        let persisted: Preferences =
            serde_json::from_slice(&std::fs::read(directory.join("preferences.json")).unwrap())
                .unwrap();
        assert_eq!(persisted, before);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn invalid_catalog_is_preserved_and_blocks_mutation() {
        let fake = FakeSecretStore::default();
        let directory = std::env::temp_dir().join(format!(
            "mimi-profile-invalid-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(PROFILE_CATALOG_FILE);
        std::fs::write(&path, r#"{"schemaVersion":99,"profiles":[]}"#).unwrap();
        let store = SettingsStore::at_path(directory.clone(), Box::new(fake));

        assert_eq!(
            store
                .create_profile(ProviderKind::OpenAIRealtime, "Work")
                .unwrap_err(),
            PROFILE_CATALOG_UNAVAILABLE
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"schemaVersion":99,"profiles":[]}"#
        );
        let fallback = crate::commands::SettingsSnapshotPayload::from_store(&store);
        assert!(fallback
            .profiles
            .iter()
            .all(|profile| profile.credential_state == CredentialState::Unavailable));
        let fallback_json = serde_json::to_value(fallback).unwrap();
        assert!(fallback_json.get("error").is_none());
        assert!(fallback_json.get("credentialError").is_none());

        let _ = std::fs::remove_dir_all(directory);
    }
}
