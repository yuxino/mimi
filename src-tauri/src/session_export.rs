//! Settings-only manual export: native save picker, then atomic file replacement.
use crate::commands::AppState;
use crate::core::session_archive::ArchiveState;
use serde::Deserialize;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

static EXPORT_BUSY: AtomicBool = AtomicBool::new(false);
struct ExportGuard;
impl Drop for ExportGuard {
    fn drop(&mut self) {
        EXPORT_BUSY.store(false, Ordering::SeqCst);
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportKind {
    Transcript,
    Audio,
}

#[tauri::command]
pub fn session_archive_state(state: State<'_, AppState>) -> ArchiveState {
    state.session.archive_state()
}

#[tauri::command]
pub async fn session_archive_clear(state: State<'_, AppState>) -> Result<(), String> {
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    state.session.clear_archive();
    Ok(())
}

#[tauri::command]
pub async fn session_export(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: ExportKind,
) -> Result<bool, String> {
    if EXPORT_BUSY
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("An export is already open.".into());
    }
    let _busy = ExportGuard;
    let (revision, bytes) = {
        let _lifecycle = state.session.settings_mutation_guard(true).await?;
        let bytes = match kind {
            ExportKind::Transcript => state.session.export_transcript(),
            ExportKind::Audio => state.session.export_audio(),
        }
        .ok_or("No session content is available to export.")?;
        (state.session.archive_revision(), bytes)
    };
    let (extension, description) = match kind {
        ExportKind::Transcript => ("txt", "Text transcript"),
        ExportKind::Audio => ("wav", "WAV audio"),
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter(description, &[extension])
        .set_file_name(format!("mimi-session.{extension}"))
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx
        .await
        .map_err(|_| "The export dialog closed unexpectedly.")?
    else {
        return Ok(false);
    };
    let path = path
        .into_path()
        .map_err(|_| "The export destination is not a local file.")?;
    // Revalidate after the picker: toggling off, clearing or starting a session
    // must invalidate the old snapshot. Serialize only the final write, never
    // the user's time in the dialog, with lifecycle and preference changes.
    let _lifecycle = state.session.settings_mutation_guard(true).await?;
    if state.session.archive_revision() != revision {
        return Err("The session changed. Please export again.".into());
    }
    tauri::async_runtime::spawn_blocking(move || write_export(&path, &bytes))
        .await
        .map_err(|_| "The export could not be completed.")?
        .map_err(|_| "The export could not be saved.")?;
    Ok(true)
}

fn write_export(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("missing parent"))?;
    // Private temporary file in the destination directory; failure removes the
    // temporary file and preserves any previous destination. No app cache copy.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_replace_only_the_selected_file_without_leaving_temporary_copies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.txt");
        std::fs::write(&path, b"previous").unwrap();
        write_export(&path, b"synthetic test content").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"synthetic test content");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_destination_leaves_existing_content_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("existing-directory");
        std::fs::create_dir(&destination).unwrap();
        assert!(write_export(&destination, b"synthetic test content").is_err());
        assert!(destination.is_dir());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
