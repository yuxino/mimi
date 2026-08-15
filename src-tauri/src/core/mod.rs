//! UI-independent core: models, configuration, protocols, subtitle assembly,
//! and pipeline diagnostics. Ported 1:1 from the Swift `MimiCore` target.

pub mod committer;
pub mod configuration;
pub mod diagnostics;
pub mod models;
pub mod pcm16;
pub mod protocols;
pub mod session;
pub mod subtitle_reducer;
