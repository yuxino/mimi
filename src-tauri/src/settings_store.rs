//! Preferences (languages, mode, font size, overlay state)
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

/// Keychain service owned by this app. Items created by the original Swift
/// app (and by the first Tauri build that reused its service) carry a
/// partition-list ACL tied to the creating binary: every read prompts for
/// authorization, and dev rebuilds change the binary's cdhash so the grant
/// never sticks. Credentials therefore live in this app's own service, whose
/// default ACL (created by the `security` CLI) never prompts; values found in
/// the older services are migrated here on first successful read.
pub const KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.credentials.v3";
/// Service reused by the first Tauri build; inherited the Swift-era ACL.
pub const KEYCHAIN_SERVICE_TAURI_V2: &str = "app.yuxino.mimi.credentials.v2";
/// Service written by the original Swift app.
pub const LEGACY_KEYCHAIN_SERVICE: &str = "app.yuxino.mimi.translation";
pub const KEYCHAIN_ACCOUNT: &str = "dashscope-api-key";

pub const FONT_SIZE_RANGE: std::ops::RangeInclusive<f64> = 14.0..=20.0;
pub const DEFAULT_FONT_SIZE: f64 = 18.0;

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
    pub overlay_locked: bool,
    pub overlay_frame: Option<OverlayFrame>,
    pub frame_layout_version: u64,
    /// UI language override: `None` follows the system language, `"zh"` forces
    /// Chinese, `"en"` forces English. Kept in preferences so the choice
    /// survives app restarts.
    pub ui_language: Option<String>,
    /// The app identity (bundle id) the keychain item's access-control list
    /// was last rebound to (see `SettingsStore::rebind_keychain_acl`).
    /// `None` means it has never been rebound for this identity.
    pub keychain_rebound_identity: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            source_language: SourceLanguage::Automatic,
            target_language: TargetLanguage::SimplifiedChinese,
            translation_mode: TranslationMode::LowLatency,
            font_size: DEFAULT_FONT_SIZE,
            overlay_locked: false,
            overlay_frame: None,
            frame_layout_version: 0,
            ui_language: None,
            keychain_rebound_identity: None,
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
        if let Some(value) = load_from_keyring(KEYCHAIN_SERVICE)? {
            return Ok(Some(value));
        }
        // Migrate from the older services. Reading the Swift-era items may
        // prompt once for keychain authorization; the migrated value lands in
        // this app's own service, whose default ACL never prompts again.
        for legacy in [KEYCHAIN_SERVICE_TAURI_V2, LEGACY_KEYCHAIN_SERVICE] {
            if let Some(value) = load_from_keyring(legacy)? {
                save_to_keyring(KEYCHAIN_SERVICE, &value)?;
                return Ok(Some(value));
            }
        }
        Ok(None)
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
    /// In-memory credential cache. Keychain access can block on an
    /// authorization prompt; caching serializes every caller behind a single
    /// load so concurrent `settings_get` calls (one per window) can neither
    /// each trigger their own prompt nor starve the async runtime.
    key_cache: Mutex<Option<Result<Option<String>, String>>>,
    is_ui_test: bool,
}

impl SettingsStore {
    /// Loads preferences from `app_config_dir()/preferences.json`.
    pub fn load(app_config_dir: PathBuf, is_ui_test: bool) -> Self {
        let prefs_path = app_config_dir.join("preferences.json");
        let prefs = std::fs::read_to_string(&prefs_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Preferences>(&text).ok())
            .unwrap_or_default();
        let font_size = prefs
            .font_size
            .clamp(*FONT_SIZE_RANGE.start(), *FONT_SIZE_RANGE.end());
        let prefs = Preferences { font_size, ..prefs };
        Self {
            prefs_path,
            prefs: Mutex::new(prefs),
            secret: Box::new(KeyringSecretStore),
            key_cache: Mutex::new(None),
            is_ui_test,
        }
    }

    #[cfg(test)]
    pub fn in_memory(secret: Box<dyn SecretStore>, is_ui_test: bool) -> Self {
        Self {
            prefs_path: PathBuf::new(),
            prefs: Mutex::new(Preferences::default()),
            secret,
            key_cache: Mutex::new(None),
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

    /// The stored API key, or `None` when missing. Older-service values are
    /// migrated to this app's own keychain service on first read. Results are
    /// cached in memory: keychain access can block on an authorization
    /// prompt, so every caller shares one load instead of racing.
    pub fn load_api_key(&self) -> Result<Option<String>, String> {
        if self.is_ui_test {
            return Ok(Some("sk-demo-not-a-real-key".into()));
        }
        let mut cache = self.key_cache.lock().unwrap();
        if let Some(cached) = cache.clone() {
            return cached;
        }
        let result = self.secret.load();
        if let Ok(Some(value)) = &result {
            self.rebind_keychain_acl(value);
        }
        *cache = Some(result.clone());
        result
    }

    /// The keychain item may have been created by an older build whose
    /// (ad-hoc) signature differs from this app's current identity, so macOS
    /// prompts for keychain authorization on every launch. Deleting and
    /// re-creating the item binds its access-control list to the current
    /// code-signing identity (stable thanks to `mimi Local Development`),
    /// after which reads no longer prompt. Runs once per identity; a failure
    /// leaves the flag unset so it retries next launch.
    fn rebind_keychain_acl(&self, value: &str) {
        let identity = current_app_identity();
        let already = self.prefs.lock().unwrap().keychain_rebound_identity.clone();
        if already.as_deref() == Some(identity.as_str()) {
            return;
        }
        let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) else {
            return;
        };
        let rebound = entry
            .delete_credential()
            .and_then(|_| entry.set_password(value));
        match rebound {
            Ok(()) => {
                self.update_preferences(|prefs| {
                    prefs.keychain_rebound_identity = Some(identity.clone());
                });
                self.persist();
                tracing::info!("keychain item ACL rebound to app identity {identity}");
            }
            Err(error) => {
                tracing::warn!("keychain ACL rebind failed ({error}); will retry next launch");
            }
        }
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

    /// Validates and saves the API key (key → OS keychain), then persists
    /// preferences. Mirrors `AppSettings.save()`.
    pub fn save_credentials(&self, api_key: &str) -> Result<(), String> {
        let prefs = self.prefs.lock().unwrap().clone();
        let configuration = LiveTranslationConfiguration::new(
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
        prefs.target_language = validated.target_language;
        prefs.translation_mode = effective_mode;
        drop(prefs);

        if !self.is_ui_test {
            self.secret.save(&validated.api_key)?;
            // Keep the in-memory cache in sync with the freshly saved value.
            *self.key_cache.lock().unwrap() = Some(Ok(Some(validated.api_key.clone())));
        }
        self.persist();
        Ok(())
    }

    /// The validated configuration used to start a session.
    pub fn configuration(&self) -> Result<LiveTranslationConfiguration, String> {
        let prefs = self.prefs.lock().unwrap().clone();
        let api_key = self.load_api_key()?.unwrap_or_default();
        LiveTranslationConfiguration::new(
            api_key,
            prefs.source_language,
            prefs.target_language,
            prefs.translation_mode,
        )
        .validated()
        .map_err(|error| error.to_string())
    }

    /// Applies the listening-time language adjustment: Chinese source forces
    /// original subtitles. Automatic source stays in the preferences as the
    /// user's choice; the recognition engine's concrete language is resolved
    /// when the session clients are built.
    pub fn prepare_for_listening(&self) {
        let mut prefs = self.prefs.lock().unwrap();
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

/// The identity this app presents to the keychain: the macOS bundle
/// identifier (dev wrapper: `app.yuxino.mimi.dev`, release:
/// `app.yuxino.mimi`). Both apps share the same config dir and keychain
/// service, so the rebind marker must distinguish them; the code-signing
/// certificate is stable ("mimi Local Development"), so the bundle id alone
/// is a stable per-app key.
fn current_app_identity() -> String {
    #[cfg(target_os = "macos")]
    {
        use objc2::msg_send;
        use objc2::runtime::{AnyClass, AnyObject};
        use std::ffi::{c_char, CStr};
        unsafe {
            if let Some(bundle_class) = AnyClass::get(c"NSBundle") {
                let main_bundle: *mut AnyObject = msg_send![bundle_class, mainBundle];
                if !main_bundle.is_null() {
                    let identifier: *mut AnyObject = msg_send![main_bundle, bundleIdentifier];
                    if !identifier.is_null() {
                        let ptr: *const c_char = msg_send![identifier, UTF8String];
                        if !ptr.is_null() {
                            if let Ok(id) = CStr::from_ptr(ptr).to_str() {
                                return id.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    // Non-macOS or lookup failure: fall back to the shared identifier.
    "app.yuxino.mimi".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingStore {
        value: Option<String>,
        loads: Arc<AtomicUsize>,
    }

    impl SecretStore for CountingStore {
        fn load(&self) -> Result<Option<String>, String> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            Ok(self.value.clone())
        }

        fn save(&self, _value: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn load_api_key_caches_the_first_result() {
        let loads = Arc::new(AtomicUsize::new(0));
        let secret = CountingStore {
            value: Some("k".into()),
            loads: Arc::clone(&loads),
        };
        let settings = SettingsStore::in_memory(Box::new(secret), false);
        assert_eq!(settings.load_api_key().unwrap().as_deref(), Some("k"));
        assert_eq!(settings.load_api_key().unwrap().as_deref(), Some("k"));
        // The second call must come from the cache, not the secret store.
        assert_eq!(loads.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn save_credentials_refreshes_the_cache() {
        let loads = Arc::new(AtomicUsize::new(0));
        let secret = CountingStore {
            value: None,
            loads: Arc::clone(&loads),
        };
        let settings = SettingsStore::in_memory(Box::new(secret), false);
        assert_eq!(settings.load_api_key().unwrap(), None);
        settings.save_credentials("sk-new-key").unwrap();
        // The saved key is served from the refreshed cache without a reload.
        assert_eq!(
            settings.load_api_key().unwrap().as_deref(),
            Some("sk-new-key")
        );
        assert_eq!(loads.load(Ordering::SeqCst), 1);
    }
}
