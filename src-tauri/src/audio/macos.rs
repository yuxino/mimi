//! macOS system-audio capture through ScreenCaptureKit (audio only, own
//! process audio excluded, 16 kHz mono), ported from
//! `Sources/MimiApp/SystemAudioCapture.swift`.
//!
//! ScreenCaptureKit types are main-thread-only (!Send), mirroring the Swift
//! `@MainActor` confinement. All SCStream/handler/content objects are
//! therefore created, used, and destroyed inside closures dispatched to the
//! main thread; the shareable-content and capture completions (which fire on
//! arbitrary queues) re-dispatch to the main thread through a raw-pointer
//! wrapper that is only ever dereferenced on the main thread.

use crate::audio::SystemAudioCaptureError;
use crate::core::pcm16::PCM16Encoder;
use crate::pipeline_log;
use core_media::block_buffer::{CMBlockBufferGetDataLength, CMBlockBufferGetDataPointer};
use core_media::format_description::CMAudioFormatDescriptionGetStreamBasicDescription;
use core_media::sample_buffer::{
    CMSampleBufferGetDataBuffer, CMSampleBufferGetFormatDescription, CMSampleBufferRef,
};
use core_media::time::CMTime;
use dispatch2::{DispatchQueue, DispatchQueueAttr};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_core_audio_types::{
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsSignedInteger, AudioStreamBasicDescription,
};
use objc2_foundation::{NSArray, NSObjectProtocol, NSString};
use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::Resampler;
use screen_capture_kit::shareable_content::{SCDisplay, SCRunningApplication, SCShareableContent};
use screen_capture_kit::stream::{
    SCContentFilter, SCStream, SCStreamConfiguration, SCStreamDelegate, SCStreamOutput,
    SCStreamOutputType,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

/// Executes a closure on the main thread (wraps `AppHandle::run_on_main_thread`).
pub type MainThreadDispatcher = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

type AudioSender = mpsc::UnboundedSender<Vec<u8>>;
type ErrorSender = mpsc::UnboundedSender<String>;

struct AudioHandlerState {
    audio_tx: AudioSender,
    error_tx: ErrorSender,
    last_audio_buffer_at: Mutex<Option<Instant>>,
    /// Decoded-buffer counter used to throttle diagnostics.
    decoded_buffers: Mutex<u64>,
    /// 16 kHz resampler; rebuilt when the stream sample rate changes.
    /// SCStream delivers the device rate (typically 48 kHz) regardless of the
    /// requested configuration, so the samples must be resampled before the
    /// ASR pipeline (which expects 16 kHz PCM).
    resampler: Mutex<Option<rubato::Fft<f32>>>,
    pending_frames: Mutex<Vec<f32>>,
}

/// A pointer that is only ever dereferenced on the main thread, inside the
/// closure it is delivered to. `unsafe impl Send` is sound because raw pointer
/// values carry no ownership and the pointee never crosses threads.
struct MainThreadPtr(*mut ());
unsafe impl Send for MainThreadPtr {}

impl MainThreadPtr {
    /// Consumes the wrapper and returns the raw pointer. Called inside the
    /// main-thread closure so the whole wrapper (with its Send impl) is
    /// captured rather than the bare field.
    fn into_inner(self) -> *mut () {
        self.0
    }
}

pub struct MimiAudioStreamHandlerIvars {
    state_ptr: *mut c_void,
}

define_class!(
    // SAFETY:
    // - The superclass NSObject has no subclassing requirements.
    // - MimiAudioStreamHandler does not implement Drop.
    #[unsafe(super(NSObject))]
    #[name = "MimiAudioStreamHandler"]
    #[ivars = MimiAudioStreamHandlerIvars]
    pub struct MimiAudioStreamHandler;

    unsafe impl NSObjectProtocol for MimiAudioStreamHandler {}

    unsafe impl SCStreamOutput for MimiAudioStreamHandler {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        unsafe fn stream_did_output_sample_buffer(
            &self,
            _stream: &SCStream,
            sample_buffer: CMSampleBufferRef,
            of_type: SCStreamOutputType,
        ) {
            if of_type != SCStreamOutputType::Audio {
                return;
            }
            let state = unsafe { &*(self.ivars().state_ptr as *const AudioHandlerState) };
            let now = Instant::now();
            if let Some(last) = *state.last_audio_buffer_at.lock().unwrap() {
                let gap_ms = now.saturating_duration_since(last).as_millis();
                if gap_ms > 500 {
                    pipeline_log!("capture gapMs={}", gap_ms);
                }
            }
            *state.last_audio_buffer_at.lock().unwrap() = Some(now);

            match capture_to_pcm16(state, sample_buffer) {
                Ok(Some(data)) if !data.is_empty() => {
                    // Content-free capture diagnostics: report decoded-buffer
                    // statistics every 200 buffers.
                    let count = {
                        let mut count = state.decoded_buffers.lock().unwrap();
                        *count += 1;
                        *count
                    };
                    if count % 200 == 1 {
                        let nonzero = data.iter().filter(|byte| **byte != 0).count();
                        pipeline_log!(
                            "capture decode buffers={} bytes={} nonzero={}/{}",
                            count,
                            data.len(),
                            nonzero,
                            data.len()
                        );
                    }
                    let _ = state.audio_tx.send(data);
                }
                Ok(None) | Ok(Some(_)) => {}
                Err(error) => {
                    let _ = state.error_tx.send(error.to_string());
                }
            }
        }
    }

    unsafe impl SCStreamDelegate for MimiAudioStreamHandler {
        #[unsafe(method(stream:didStopWithError:))]
        unsafe fn stream_did_stop_with_error(
            &self,
            _stream: &SCStream,
            error: &objc2_foundation::NSError,
        ) {
            let state = unsafe { &*(self.ivars().state_ptr as *const AudioHandlerState) };
            let _ = state
                .error_tx
                .send(format!("System audio capture stopped: {error}"));
        }
    }
);

/// Tracks the last reported stream format so the capture diagnostics only
/// print when the format changes (avoids log spam at 50 buffers/second).
pub static FORMAT_SIGNATURE: std::sync::Mutex<Option<(u32, u32, u32)>> =
    std::sync::Mutex::new(None);

// Main-thread-only live capture state. Only ever accessed from main-thread
// closures (see `MainThreadDispatcher`).
thread_local! {
    static MAIN_STREAM: RefCell<Option<Retained<SCStream>>> = const { RefCell::new(None) };
    static MAIN_HANDLER: RefCell<Option<Retained<MimiAudioStreamHandler>>> =
        const { RefCell::new(None) };
    static MAIN_STATE: RefCell<Option<Box<AudioHandlerState>>> = const { RefCell::new(None) };
}

#[derive(Clone)]
pub struct MacSystemAudioCapture {
    dispatcher: MainThreadDispatcher,
    started: Arc<AtomicBool>,
}

impl MacSystemAudioCapture {
    pub fn new(dispatcher: MainThreadDispatcher) -> Self {
        Self {
            dispatcher,
            started: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts capture. `audio_tx` receives PCM16 buffers; `error_tx` receives
    /// capture failure descriptions.
    pub async fn start(
        &self,
        audio_tx: AudioSender,
        error_tx: ErrorSender,
    ) -> Result<(), SystemAudioCaptureError> {
        if self.started.swap(true, Ordering::SeqCst) {
            return Err(SystemAudioCaptureError::AlreadyRunning);
        }

        // Phase 1 (main thread): ask ScreenCaptureKit for shareable content.
        let dispatcher = Arc::clone(&self.dispatcher);
        let phase2_dispatcher = Arc::clone(&self.dispatcher);
        let (content_tx, content_rx) = oneshot::channel::<Result<(), String>>();
        let content_tx = Arc::new(Mutex::new(Some(content_tx)));
        dispatcher(Box::new(move || {
            // ScreenCaptureKit calls can raise Objective-C exceptions; catch
            // them here so they surface as errors instead of aborting.
            let _ = objc2::exception::catch(AssertUnwindSafe(|| {
                SCShareableContent::get_shareable_content_excluding_desktop_windows(
                    false,
                    false,
                    move |content, error| {
                        // This completion fires on an arbitrary queue; re-dispatch
                        // phase 2 to the main thread with the content.
                        let content = match (content, error) {
                            (Some(content), _) => content,
                            (None, Some(error)) => {
                                if let Some(tx) = content_tx.lock().unwrap().take() {
                                    let _ = tx.send(Err(error.to_string()));
                                }
                                return;
                            }
                            (None, None) => {
                                if let Some(tx) = content_tx.lock().unwrap().take() {
                                    let _ = tx.send(Err(
                                    "mimi could not find a display to use for system audio capture."
                                        .into(),
                                ));
                                }
                                return;
                            }
                        };
                        let ptr = MainThreadPtr(Box::into_raw(Box::new(content)) as *mut ());
                        let audio_tx = audio_tx.clone();
                        let error_tx = error_tx.clone();
                        let dispatcher = Arc::clone(&phase2_dispatcher);
                        let content_tx = Arc::clone(&content_tx);
                        dispatcher(Box::new(move || {
                            let raw = ptr.into_inner();
                            let content =
                                unsafe { *Box::from_raw(raw as *mut Retained<SCShareableContent>) };
                            let result = objc2::exception::catch(AssertUnwindSafe(|| {
                                start_capture_on_main(content, audio_tx, error_tx)
                            }));
                            let result = match result {
                            Ok(result) => result,
                            Err(exception) => match exception {
                                Some(exception) => Err(exception_message(&exception)),
                                None => Err(
                                    "System audio capture raised an unknown Objective-C exception."
                                        .into(),
                                ),
                            },
                        };
                            if let Some(tx) = content_tx.lock().unwrap().take() {
                                let _ = tx.send(result);
                            }
                        }));
                    },
                );
            }));
        }));

        match content_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                self.started.store(false, Ordering::SeqCst);
                Err(SystemAudioCaptureError::Other(error))
            }
            Err(_) => {
                self.started.store(false, Ordering::SeqCst);
                Err(SystemAudioCaptureError::NoDisplay)
            }
        }
    }

    pub async fn stop(&self) {
        if !self.started.swap(false, Ordering::SeqCst) {
            return;
        }
        let (done_tx, done_rx) = oneshot::channel::<()>();
        let dispatcher = Arc::clone(&self.dispatcher);
        dispatcher(Box::new(move || {
            let stream = MAIN_STREAM.take();
            if let Some(stream) = stream {
                stream.stop_capture(move |_error| {
                    // Stop completion fires on an arbitrary queue; nothing to
                    // forward after the stream is gone.
                });
                let _ = MAIN_STATE.take();
                let _ = MAIN_HANDLER.take();
            }
            let _ = done_tx.send(());
        }));
        let _ = done_rx.await;
    }
}

/// Runs entirely on the main thread: builds the filter/configuration/handler,
/// creates the stream, and starts capture.
fn start_capture_on_main(
    content: Retained<SCShareableContent>,
    audio_tx: AudioSender,
    error_tx: ErrorSender,
) -> Result<(), String> {
    pipeline_log!("audio3 capture phase2 start");
    // Pick the main display, falling back to the first one.
    let displays = content.displays();
    let main_display_id = unsafe { core_graphics2::display::CGMainDisplayID() };
    let mut chosen_display: Option<Retained<SCDisplay>> = None;
    for index in 0..displays.len() {
        let display = displays.objectAtIndex(index);
        if display.display_id() == main_display_id {
            chosen_display = Some(display);
            break;
        }
    }
    let Some(display) = chosen_display.or_else(|| displays.firstObject()) else {
        return Err("mimi could not find a display to use for system audio capture.".into());
    };

    // Exclude this app from the captured audio.
    let own_bundle_id = own_bundle_identifier();
    let applications = content.applications();
    let mut excluded: Vec<Retained<SCRunningApplication>> = Vec::new();
    for index in 0..applications.len() {
        let app = applications.objectAtIndex(index);
        if own_bundle_id
            .as_ref()
            .is_some_and(|own| app.bundle_identifier().to_string() == *own)
        {
            excluded.push(app);
        }
    }
    let excluded = NSArray::from_retained_slice(&excluded);
    let no_windows = NSArray::new();
    let filter = SCContentFilter::init_with_display_exclude_applications(
        SCContentFilter::alloc(),
        &display,
        &excluded,
        &no_windows,
    );
    pipeline_log!("audio3 capture filter built");

    // Audio-only stream configuration: 16 kHz mono, own audio excluded.
    let configuration = SCStreamConfiguration::new();
    configuration.set_captures_audio(true);
    configuration.set_excludes_current_process_audio(true);
    configuration.set_sample_rate(16_000.0);
    configuration.set_channel_count(1);
    configuration.set_width(2);
    configuration.set_height(2);
    configuration.set_minimum_frame_interval(CMTime {
        value: 1,
        timescale: 1,
        flags: 0,
        epoch: 0,
    });
    configuration.set_queue_depth(3);
    // The `screen-capture-kit` crate's `set_show_cursor` sends the wrong
    // selector (`setShowCursor:`); the property is `showsCursor` →
    // `setShowsCursor:`.
    let _: () = unsafe { msg_send![&*configuration, setShowsCursor: false] };

    // Handler + stream.
    let state = Box::new(AudioHandlerState {
        audio_tx,
        error_tx,
        last_audio_buffer_at: Mutex::new(None),
        decoded_buffers: Mutex::new(0),
        resampler: Mutex::new(None),
        pending_frames: Mutex::new(Vec::new()),
    });
    let state_ptr: *mut c_void = (&*state as *const AudioHandlerState) as *mut c_void;
    let this = MimiAudioStreamHandler::alloc().set_ivars(MimiAudioStreamHandlerIvars { state_ptr });
    let handler: Retained<MimiAudioStreamHandler> = unsafe { msg_send![super(this), init] };
    let output = ProtocolObject::from_ref(&*handler);
    let delegate = ProtocolObject::from_ref(&*handler);

    let stream = SCStream::init_with_filter(SCStream::alloc(), &filter, &configuration, delegate);
    pipeline_log!("audio3 capture stream created");

    // Deliver audio samples on a dedicated serial queue.
    let queue = DispatchQueue::new("app.yuxino.mimi.system-audio", DispatchQueueAttr::SERIAL);
    if let Err(error) = stream.add_stream_output(output, SCStreamOutputType::Audio, &queue) {
        return Err(error.to_string());
    }
    pipeline_log!("audio3 capture output attached");

    // Start capture; on failure, clean everything up.
    let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
    stream.start_capture(move |error| {
        let _ = tx.send(error.map(|error| error.to_string()));
    });
    if let Ok(Some(error)) = rx.recv_timeout(std::time::Duration::from_secs(15)) {
        let _ = stream.remove_stream_output(output, SCStreamOutputType::Audio);
        return Err(error);
    }

    pipeline_log!("audio3 capture started");
    MAIN_STREAM.set(Some(stream));
    MAIN_HANDLER.set(Some(handler));
    MAIN_STATE.set(Some(state));
    Ok(())
}

/// Renders an Objective-C exception as a content-free error description.
fn exception_message(exception: &objc2::exception::Exception) -> String {
    let description: Retained<NSString> = unsafe { msg_send![exception, description] };
    let trimmed: String = description.to_string().chars().take(300).collect();
    if trimmed.is_empty() {
        "System audio capture raised an Objective-C exception.".to_string()
    } else {
        format!("System audio capture failed: {trimmed}")
    }
}

fn own_bundle_identifier() -> Option<String> {
    use objc2_foundation::NSBundle;
    NSBundle::mainBundle()
        .bundleIdentifier()
        .map(|id| id.to_string())
}

/// Extracts the sample buffer as f32 mono samples (honoring the stream's
/// format description), resamples them to 16 kHz with `rubato` when needed,
/// and quantizes to PCM16 for the ASR pipeline.
fn capture_to_pcm16(
    state: &AudioHandlerState,
    sample_buffer: CMSampleBufferRef,
) -> Result<Option<Vec<u8>>, SystemAudioCaptureError> {
    let (samples, sample_rate) = unsafe {
        let block_buffer = CMSampleBufferGetDataBuffer(sample_buffer);
        if block_buffer.is_null() {
            return Ok(None);
        }
        let mut length_at_offset: usize = 0;
        let mut total_length: usize = 0;
        let mut data_ptr: *mut c_void = null_mut();
        let status = CMBlockBufferGetDataPointer(
            block_buffer,
            0,
            &mut length_at_offset,
            &mut total_length,
            (&mut data_ptr) as *mut *mut c_void,
        );
        if status != 0 || data_ptr.is_null() || length_at_offset == 0 {
            let _ = CMBlockBufferGetDataLength(block_buffer);
            return Ok(None);
        }

        let format_description = CMSampleBufferGetFormatDescription(sample_buffer);
        if format_description.is_null() {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        }
        let asbd = CMAudioFormatDescriptionGetStreamBasicDescription(format_description);
        if asbd.is_null() {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        }
        let asbd = &*asbd;
        let bytes = std::slice::from_raw_parts(data_ptr as *const u8, length_at_offset);

        // Log the stream format once per format change.
        let signature = (
            asbd.mBitsPerChannel,
            asbd.mChannelsPerFrame,
            asbd.mFormatFlags as u32,
        );
        let mut last_format = FORMAT_SIGNATURE.lock().unwrap();
        if *last_format != Some(signature) {
            *last_format = Some(signature);
            let nonzero = bytes.iter().filter(|byte| **byte != 0).count();
            pipeline_log!(
                "capture format bytes={} asbd={}Hz {}ch {}bit flags={:#x} nonzero={}/{}",
                bytes.len(),
                asbd.mSampleRate as u64,
                asbd.mChannelsPerFrame,
                asbd.mBitsPerChannel,
                asbd.mFormatFlags,
                nonzero,
                bytes.len()
            );
        }

        (decode_to_f32_mono(bytes, asbd)?, asbd.mSampleRate)
    };

    if samples.is_empty() {
        return Ok(None);
    }

    // Resample to 16 kHz when the device rate differs from the ASR pipeline.
    let pcm = if (sample_rate - TARGET_SAMPLE_RATE).abs() < 0.5 {
        PCM16Encoder::encode(&[samples])
    } else {
        let mut pending = state.pending_frames.lock().unwrap();
        pending.extend_from_slice(&samples);
        let mut resampler_guard = state.resampler.lock().unwrap();
        if resampler_guard.is_none() {
            let resampler = rubato::Fft::<f32>::new(
                sample_rate as usize,
                TARGET_SAMPLE_RATE as usize,
                RESAMPLE_CHUNK_FRAMES,
                1,
                rubato::FixedSync::Input,
            )
            .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))?;
            *resampler_guard = Some(resampler);
        }
        let mut out = Vec::new();
        let resampler = resampler_guard.as_mut().unwrap();
        while pending.len() >= RESAMPLE_CHUNK_FRAMES {
            let chunk: Vec<f32> = pending.drain(..RESAMPLE_CHUNK_FRAMES).collect();
            let chunk_refs = [chunk];
            let input = SequentialSliceOfVecs::new(&chunk_refs, 1, RESAMPLE_CHUNK_FRAMES)
                .map_err(|error| SystemAudioCaptureError::Other(error.to_string()))?;
            match resampler.process(&input, None) {
                Ok(output) => out.extend_from_slice(&output.take_data()),
                Err(_) => break,
            }
        }
        if out.is_empty() {
            return Ok(None);
        }
        PCM16Encoder::encode(&[out])
    };

    Ok(Some(pcm))
}

const TARGET_SAMPLE_RATE: f64 = 16_000.0;
const RESAMPLE_CHUNK_FRAMES: usize = 1024;

/// Decodes interleaved/planar linear PCM into mono f32 samples. Both the
/// float and signed-integer branches handle multi-channel buffers by
/// averaging channels, matching `PCM16Encoder::encode`.
fn decode_to_f32_mono(
    bytes: &[u8],
    asbd: &AudioStreamBasicDescription,
) -> Result<Vec<f32>, SystemAudioCaptureError> {
    let channels = asbd.mChannelsPerFrame.max(1) as usize;
    let is_float = asbd.mFormatFlags & kAudioFormatFlagIsFloat != 0;
    let is_signed_int = asbd.mFormatFlags & kAudioFormatFlagIsSignedInteger != 0;
    let is_planar = asbd.mFormatFlags & (1 << 6) != 0; // kAudioFormatFlagIsNonInterleaved

    if is_planar {
        // Planar layout: channel data is contiguous per channel.
        let frames_per_channel = if is_float && asbd.mBitsPerChannel == 32 {
            bytes.len() / (4 * channels)
        } else if is_signed_int && asbd.mBitsPerChannel == 16 {
            bytes.len() / (2 * channels)
        } else if is_signed_int && asbd.mBitsPerChannel == 32 {
            bytes.len() / (4 * channels)
        } else {
            return Err(SystemAudioCaptureError::UnsupportedAudioFormat);
        };
        let bytes_per_sample = (asbd.mBitsPerChannel / 8) as usize;
        let mut mono = Vec::with_capacity(frames_per_channel);
        for frame in 0..frames_per_channel {
            let mut mixed = 0.0f64;
            for channel in 0..channels {
                let offset = (frame + channel * frames_per_channel) * bytes_per_sample;
                mixed += sample_to_f32(bytes, offset, is_float, asbd.mBitsPerChannel)? as f64;
            }
            mono.push((mixed / channels as f64) as f32);
        }
        Ok(mono)
    } else if is_float && asbd.mBitsPerChannel == 32 {
        let bytes_per_sample = 4;
        let frames = bytes.len() / (bytes_per_sample * channels);
        if frames == 0 {
            return Ok(Vec::new());
        }
        let mut mono = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut mixed = 0.0f64;
            for channel in 0..channels {
                let offset = (frame * channels + channel) * bytes_per_sample;
                let sample = f32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                mixed += sample as f64;
            }
            mono.push((mixed / channels as f64) as f32);
        }
        Ok(mono)
    } else if is_signed_int && asbd.mBitsPerChannel == 16 {
        let bytes_per_sample = 2;
        let frames = bytes.len() / (bytes_per_sample * channels);
        if frames == 0 {
            return Ok(Vec::new());
        }
        let mut mono = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut mixed = 0.0f64;
            for channel in 0..channels {
                let offset = (frame * channels + channel) * bytes_per_sample;
                mixed += i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as f64;
            }
            mono.push((mixed / channels as f64 / 32_768.0) as f32);
        }
        Ok(mono)
    } else if is_signed_int && asbd.mBitsPerChannel == 32 {
        let bytes_per_sample = 4;
        let frames = bytes.len() / (bytes_per_sample * channels);
        if frames == 0 {
            return Ok(Vec::new());
        }
        let mut mono = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut mixed = 0.0f64;
            for channel in 0..channels {
                let offset = (frame * channels + channel) * bytes_per_sample;
                let sample = i32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                mixed += sample as f64;
            }
            mono.push((mixed / channels as f64 / 2_147_483_648.0) as f32);
        }
        Ok(mono)
    } else {
        Err(SystemAudioCaptureError::UnsupportedAudioFormat)
    }
}

fn sample_to_f32(
    bytes: &[u8],
    offset: usize,
    is_float: bool,
    bits: u32,
) -> Result<f32, SystemAudioCaptureError> {
    if is_float && bits == 32 {
        Ok(f32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]))
    } else if !is_float && bits == 16 {
        Ok(i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as f32 / 32_768.0)
    } else if !is_float && bits == 32 {
        Ok(i32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as f32
            / 2_147_483_648.0)
    } else {
        Err(SystemAudioCaptureError::UnsupportedAudioFormat)
    }
}
