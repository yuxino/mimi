//! Content-free pipeline diagnostics, ported from
//! `Sources/MimiCore/PipelineDiagnostics.swift`. Diagnostics contain timing,
//! counts, language codes, and sanitized error labels only; recognized or
//! translated text must never be logged.

/// Logs a pipeline diagnostic through `tracing`. Disabled when the
/// `MIMI_PIPELINE_DIAGNOSTICS` environment variable is exactly `"0"`.
#[macro_export]
macro_rules! pipeline_log {
    ($($arg:tt)*) => {
        if $crate::core::diagnostics::is_enabled() {
            tracing::info!($($arg)*);
        }
    };
}

pub fn is_enabled() -> bool {
    static CELL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CELL.get_or_init(|| std::env::var("MIMI_PIPELINE_DIAGNOSTICS").as_deref() != Ok("0"))
}

/// Milliseconds elapsed between two `Instant`s.
pub fn milliseconds(start: std::time::Instant, end: std::time::Instant) -> u64 {
    end.saturating_duration_since(start).as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milliseconds_measures_elapsed_time() {
        let start = std::time::Instant::now();
        let end = start + std::time::Duration::from_millis(150);
        assert_eq!(milliseconds(start, end), 150);
    }
}
