# Automatic Language Switching Design

## Goal and provider behavior

mimi should recognize a video's spoken language without requiring the user to stop, change a setting, and reconnect. Automatic mode is segment-based: one realtime session can transcribe Japanese, English, Korean, Chinese, or another language supported by Qwen ASR as the audio changes, while every recognized segment is translated into Simplified Chinese.

Qwen-ASR-Realtime marks its input language as optional and returns detected-language metadata in transcription events. In automatic mode mimi omits the ASR language hint. Qwen-MT officially accepts `source_lang: auto`, so each partial or final transcript is independently detected before translation. Explicit Japanese, English, and Korean modes remain available because a correct language hint can improve recognition accuracy for a single-language video.

## Compatibility and settings

`SourceLanguage.automatic` is the first picker option and the default for new configurations. Existing saved choices remain intact until changed. The current user's setting will be switched to Automatic during final UI verification.

Qwen LiveTranslate's omitted source language defaults to English, so its High Quality backend cannot provide reliable language switching. If Automatic is selected, configuration resolves to the Low Latency ASR-plus-MT backend even if an old preference still contains High Quality. Both settings screens also force and lock Low Latency while Automatic is selected so the behavior is visible rather than surprising.

## Verification

Protocol tests verify that automatic ASR requests omit `input_audio_transcription.language`, automatic Qwen-MT requests send `source_lang: auto`, and automatic configurations resolve to Low Latency. Existing explicit-language JSON must remain unchanged. Final verification uses one running mimi session: play a Japanese utterance, observe Japanese-to-Chinese subtitles, then play an English utterance without reconnecting and observe English-to-Chinese subtitles.
