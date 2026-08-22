//! UI-independent models, configuration, protocols, subtitle assembly, and
//! pipeline diagnostics.

pub mod committer;
pub mod configuration;
pub mod diagnostics;
pub mod models;
pub mod openai_transcript_committer;
pub mod pcm16;
pub mod protocols;
pub mod provider;
pub mod session;
pub mod subtitle_reducer;
