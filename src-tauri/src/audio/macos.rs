//! macOS system-audio capture through ScreenCaptureKit (audio only, own
//! process audio excluded, 16 kHz mono), ported from
//! `Sources/MimiApp/SystemAudioCapture.swift`.

use crate::audio::SystemAudioCaptureError;
use crate::core::pcm16::PCM16Encoder;
use crate::pipeline_log;
use core_media::block_buffer::{
    CMBlockBufferGetDataPointer,
};
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
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger,
    AudioStreamBasicDescription,
};
use objc2_foundation::{NSArray, NSObjectProtocol};
use screen_capture_kit::shareable_content::{
    SCDisplay, SCRunningApplication, SCShareableContent,
};
use screen_capture_kit::stream::{
    SCContentFilter, SCStream, SCStreamConfiguration, SCStreamDelegate, SCStreamOutput,
    SCStreamOutputType,
};
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

type AudioSender = mpsc::UnboundedSender<Vec<u8>>;
type ErrorSender = mpsc::UnboundedSender<String>;

struct AudioHandlerState {
    audio_tx: AudioSender,
    error_tx: ErrorSender,
    last_audio_buffer_at: Mutex<Option<Instant>>,
}

#[derive(Debug)]
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

            match extract_pcm16(sample_buffer) {
                Ok(Some(data)) if !data.is_empty() => {
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
        unsafe fn stream_did_stop_with_error(&self, _stream: &SCStream, error: &objc2_foundation::NSError) {
            let state = unsafe { &*(self.ivars().state_ptr as *const AudioHandlerState) };
            let _ = state
                .error_tx
                .send(format!("System audio capture stopped: {error}"));
        }
    }
);

pub struct MacSystemAudioCapture {
    stream: Mutex<Option<Retained<SCStream>>>,
    handler: Mutex<Option<Retained<MimiAudioStreamHandler>>>,
    state: Mutex<Option<Box<AudioHandlerState>>>,
}

impl MacSystemAudioCapture {
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            handler: Mutex::new(None),
            state: Mutex::new(None),
        }
    }

    /// Starts capture. `audio_tx` receives PCM16 buffers; `error_tx` receives
    /// capture failure descriptions.
    pub async fn start(
        &self,
        audio_tx: AudioSender,
        error_tx: ErrorSender,
    ) -> Result<(), SystemAudioCaptureError> {
        if self.stream.lock().unwrap().is_some() {
            return Err(SystemAudioCaptureError::AlreadyRunning);
        }

        // 1. Ask ScreenCaptureKit for shareable content (completion handler).
        let content = get_shareable_content()
            .await
            .ok_or(SystemAudioCaptureError::NoDisplay)?;

        // 2. Pick the main display, falling back to the first one.
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
        let display = chosen_display
            .or_else(|| displays.firstObject())
            .ok_or(SystemAudioCaptureError::NoDisplay)?;

        // 3. Exclude this app from the captured audio.
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

        // 4. Audio-only stream configuration: 16 kHz mono, own audio excluded.
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
        configuration.set_show_cursor(false);

        // 5. Handler + stream.
        let state = Box::new(AudioHandlerState {
            audio_tx,
            error_tx,
            last_audio_buffer_at: Mutex::new(None),
        });
        let state_ptr: *mut c_void = (&*state as *const AudioHandlerState) as *mut c_void;
        let this = MimiAudioStreamHandler::alloc()
            .set_ivars(MimiAudioStreamHandlerIvars { state_ptr });
        let handler: Retained<MimiAudioStreamHandler> = unsafe { msg_send![super(this), init] };
        let output = ProtocolObject::from_ref(&*handler);
        let delegate = ProtocolObject::from_ref(&*handler);

        let stream = SCStream::init_with_filter(
            SCStream::alloc(),
            &filter,
            &configuration,
            delegate,
        );

        // 6. Deliver audio samples on a dedicated serial queue.
        let queue = DispatchQueue::new("app.yuxino.mimi.system-audio", DispatchQueueAttr::SERIAL);
        if let Err(error) = stream.add_stream_output(output, SCStreamOutputType::Audio, &queue) {
            return Err(SystemAudioCaptureError::Other(error.to_string()));
        }

        // 7. Start capture; on failure, clean everything up.
        if let Err(error) = start_capture(&stream).await {
            let _ = stream.remove_stream_output(output, SCStreamOutputType::Audio);
            return Err(SystemAudioCaptureError::Other(error.to_string()));
        }

        *self.stream.lock().unwrap() = Some(stream);
        *self.handler.lock().unwrap() = Some(handler);
        *self.state.lock().unwrap() = Some(state);
        Ok(())
    }

    pub async fn stop(&self) {
        let stream = self.stream.lock().unwrap().take();
        let Some(stream) = stream else { return };

        let _ = stop_capture(&stream).await;
        let handler = self.handler.lock().unwrap().take();
        if let Some(ref handler) = handler {
            let output = ProtocolObject::from_ref(&**handler);
            let _ = stream.remove_stream_output(output, SCStreamOutputType::Audio);
        }
        drop(stream);
        // Drop the handler state last so callbacks never touch freed memory.
        drop(handler);
        let _ = self.state.lock().unwrap().take();
    }
}

impl Default for MacSystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

async fn get_shareable_content() -> Option<Retained<SCShareableContent>> {
    let (tx, rx) = oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    SCShareableContent::get_shareable_content_excluding_desktop_windows(
        false,
        false,
        move |content, _error| {
            if let Some(tx) = tx.lock().unwrap().take() {
                let _ = tx.send(content);
            }
        },
    );
    rx.await.ok().flatten()
}

async fn start_capture(stream: &SCStream) -> Result<(), Retained<objc2_foundation::NSError>> {
    let (tx, rx) = oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    stream.start_capture(move |error| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(error);
        }
    });
    match rx.await {
        Ok(Some(error)) => Err(error),
        _ => Ok(()),
    }
}

async fn stop_capture(stream: &SCStream) -> Result<(), Retained<objc2_foundation::NSError>> {
    let (tx, rx) = oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    stream.stop_capture(move |error| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(error);
        }
    });
    match rx.await {
        Ok(Some(error)) => Err(error),
        _ => Ok(()),
    }
}

fn own_bundle_identifier() -> Option<String> {
    use objc2_foundation::NSBundle;
    NSBundle::mainBundle().bundleIdentifier().map(|id| id.to_string())
}

/// Extracts the sample buffer as mono PCM16, honoring the stream's audio
/// format description (float or signed-integer linear PCM).
fn extract_pcm16(
    sample_buffer: CMSampleBufferRef,
) -> Result<Option<Vec<u8>>, SystemAudioCaptureError> {
    unsafe {
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
        decode_linear_pcm(bytes, asbd)
    }
}

fn decode_linear_pcm(
    bytes: &[u8],
    asbd: &AudioStreamBasicDescription,
) -> Result<Option<Vec<u8>>, SystemAudioCaptureError> {
    let channels = asbd.mChannelsPerFrame.max(1) as usize;
    let is_float = asbd.mFormatFlags & kAudioFormatFlagIsFloat != 0;
    let is_signed_int = asbd.mFormatFlags & kAudioFormatFlagIsSignedInteger != 0;
    let _is_packed = asbd.mFormatFlags & kAudioFormatFlagIsPacked != 0;

    if is_float && asbd.mBitsPerChannel == 32 {
        let bytes_per_sample = 4;
        let frames = bytes.len() / (bytes_per_sample * channels);
        if frames == 0 {
            return Ok(None);
        }
        let mut channel_data: Vec<Vec<f32>> = vec![Vec::with_capacity(frames); channels];
        for frame in 0..frames {
            for channel in 0..channels {
                let offset = (frame * channels + channel) * bytes_per_sample;
                let sample = f32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                channel_data[channel].push(sample);
            }
        }
        Ok(Some(PCM16Encoder::encode(&channel_data)))
    } else if is_signed_int && asbd.mBitsPerChannel == 16 {
        let bytes_per_sample = 2;
        let frames = bytes.len() / (bytes_per_sample * channels);
        if frames == 0 {
            return Ok(None);
        }
        let mut pcm = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let mut mixed: i32 = 0;
            for channel in 0..channels {
                let offset = (frame * channels + channel) * bytes_per_sample;
                mixed += i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
            }
            let sample = (mixed / channels as i32) as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(Some(pcm))
    } else if is_signed_int && asbd.mBitsPerChannel == 32 {
        let bytes_per_sample = 4;
        let frames = bytes.len() / (bytes_per_sample * channels);
        if frames == 0 {
            return Ok(None);
        }
        let mut pcm = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let mut mixed: i64 = 0;
            for channel in 0..channels {
                let offset = (frame * channels + channel) * bytes_per_sample;
                mixed += i32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]) as i64;
            }
            let sample = (mixed / channels as i64) as i32;
            let clamped = sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            pcm.extend_from_slice(&clamped.to_le_bytes());
        }
        Ok(Some(pcm))
    } else {
        Err(SystemAudioCaptureError::UnsupportedAudioFormat)
    }
}
