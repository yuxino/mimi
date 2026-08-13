//! Preferences (workspace ID, languages, mode, font size, overlay state) stored
//! in `app_config_dir()/preferences.json`, and the DashScope API key stored in
//! the OS keychain (macOS Keychain / Windows Credential Manager) via `keyring`.
//! No plaintext, source-controlled, or environment-variable credential fallback.
