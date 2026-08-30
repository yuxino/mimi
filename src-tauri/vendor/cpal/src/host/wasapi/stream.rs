use std::{
    mem,
    ops::ControlFlow,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, SendError, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use windows::Win32::{
    Foundation::{self, PROPERTYKEY, WAIT_OBJECT_0},
    Media::Audio,
    System::{Performance, SystemServices, Threading},
};

use crate::{
    host::{
        emit_error, equilibrium::fill_equilibrium, frames_to_duration, latch::Latch,
        ErrorCallbackArc,
    },
    traits::StreamTrait,
    Data, Error, ErrorKind, FrameCount, InputCallbackInfo, InputStreamTimestamp,
    OutputCallbackInfo, OutputStreamTimestamp, ResultExt, SampleFormat, SampleRate, StreamConfig,
    StreamInstant,
};

/// Returns the current default audio endpoint for `flow`, or `None` if none exists.
///
/// Used by `OnDeviceStateChanged` and `OnDeviceRemoved` to cover the edge case where the
/// default device becomes unavailable with no replacement: in that situation Windows does
/// not fire `OnDefaultDeviceChanged`, so these callbacks must signal the run loop instead.
/// When a replacement *does* exist this returns `Some` and we skip signalling, letting
/// `OnDefaultDeviceChanged` fire as the sole notifier and avoiding a double wakeup.
fn get_current_default(flow: Audio::EDataFlow) -> Option<Audio::IMMDevice> {
    super::device::current_default_endpoint(flow)
}

/// Fires a Windows auto-reset event when the system default audio device changes, allowing
/// the stream run loop to deliver `ErrorKind::DeviceChanged` to the caller.
pub(crate) struct DefaultDeviceMonitor {
    enumerator: Audio::IMMDeviceEnumerator,
    client: Audio::IMMNotificationClient,
    event: Foundation::HANDLE,
    pub(crate) pending_device_changed: Arc<AtomicBool>,
}

// SAFETY: `IMMDeviceEnumerator` and `IMMNotificationClient` are COM objects used only for
// register/unregister (in new/drop) and `SetEvent` (from the Windows notification thread).
// All of these are thread-safe operations on Windows.
unsafe impl Send for DefaultDeviceMonitor {}
unsafe impl Sync for DefaultDeviceMonitor {}

impl DefaultDeviceMonitor {
    pub fn new(
        enumerator: Audio::IMMDeviceEnumerator,
        flow: Audio::EDataFlow,
    ) -> Result<Self, Error> {
        let event =
            unsafe { Threading::CreateEventW(None, false, false, None).map_err(Error::from)? };

        let pending_device_changed = Arc::new(AtomicBool::new(false));
        let client: Audio::IMMNotificationClient = DefaultDeviceNotificationImpl {
            flow,
            event,
            pending_device_changed: pending_device_changed.clone(),
        }
        .into();

        unsafe {
            enumerator
                .RegisterEndpointNotificationCallback(&client)
                .map_err(Error::from)?;
        }

        Ok(Self {
            enumerator,
            client,
            event,
            pending_device_changed,
        })
    }
}

impl Drop for DefaultDeviceMonitor {
    fn drop(&mut self) {
        // Ensure COM is initialised on this thread before making COM calls. Drop can run on
        // any thread (e.g. the audio run thread), which may not have called CoInitialize.
        crate::host::com::com_initialized();
        unsafe {
            // Synchronous: waits for any in-progress IMMNotificationClient callback to finish
            // before returning. Notification callbacks must not invoke the user error callback:
            // if the user dropped the Stream in response, this call would deadlock waiting for
            // the in-progress callback. Instead, callbacks set `pending_device_changed` and
            // the audio thread delivers the error on its next iteration.
            //
            // Only close the event handle on success; if unregister fails the callback may
            // still hold a reference and could later call SetEvent on a closed/reused handle.
            if self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.client)
                .is_ok()
            {
                let _ = Foundation::CloseHandle(self.event);
            }
        }
    }
}

#[windows::core::implement(Audio::IMMNotificationClient)]
struct DefaultDeviceNotificationImpl {
    flow: Audio::EDataFlow,
    event: Foundation::HANDLE,
    pending_device_changed: Arc<AtomicBool>,
}

impl Audio::IMMNotificationClient_Impl for DefaultDeviceNotificationImpl_Impl {
    fn OnDefaultDeviceChanged(
        &self,
        flow: Audio::EDataFlow,
        role: Audio::ERole,
        _pwstrdefaultdeviceid: &windows::core::PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == self.flow && role == Audio::eConsole {
            // SAFETY: event handle is valid for the lifetime of DefaultDeviceMonitor, which
            // outlives all uses of this HANDLE copy.
            unsafe {
                if Threading::SetEvent(self.event).is_err() {
                    self.pending_device_changed.store(true, Ordering::Relaxed);
                }
            }
        }
        Ok(())
    }

    fn OnDeviceStateChanged(
        &self,
        _pwstrdeviceid: &windows::core::PCWSTR,
        dwnewstate: Audio::DEVICE_STATE,
    ) -> windows::core::Result<()> {
        // `DEVICE_STATE_UNPLUGGED`: physical jack disconnected; endpoint still exists in the
        // collection but produces no audio. `OnDeviceRemoved` does *not* fire for this state.
        // `DEVICE_STATE_NOTPRESENT`: hardware absent; endpoint may persist as a ghost record.
        // `DEVICE_STATE_DISABLED`: device was manually disabled by the user.
        //
        // Only signal when there is no replacement default; if one exists `OnDefaultDeviceChanged`
        // will fire instead, avoiding a double wakeup.
        let is_unavailable = dwnewstate == Audio::DEVICE_STATE_DISABLED
            || dwnewstate == Audio::DEVICE_STATE_NOTPRESENT
            || dwnewstate == Audio::DEVICE_STATE_UNPLUGGED;
        if is_unavailable && get_current_default(self.flow).is_none() {
            // SAFETY: event handle is valid for the lifetime of DefaultDeviceMonitor.
            unsafe {
                if Threading::SetEvent(self.event).is_err() {
                    self.pending_device_changed.store(true, Ordering::Relaxed);
                }
            }
        }
        Ok(())
    }

    fn OnDeviceAdded(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        // Only signal when there is no replacement default; if one exists `OnDefaultDeviceChanged`
        // will fire instead, avoiding a double wakeup.
        if get_current_default(self.flow).is_none() {
            // SAFETY: event handle is valid for the lifetime of DefaultDeviceMonitor.
            unsafe {
                if Threading::SetEvent(self.event).is_err() {
                    self.pending_device_changed.store(true, Ordering::Relaxed);
                }
            }
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _pwstrdeviceid: &windows::core::PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

pub struct Stream {
    /// The high-priority audio processing thread calling callbacks.
    /// Option used for moving out in destructor.
    ///
    /// TODO: Actually set the thread priority.
    thread: Option<JoinHandle<()>>,

    // Commands processed by the `run()` method that is currently running.
    // `pending_scheduled_event` must be signalled whenever a command is added here, so that it
    // will get picked up.
    commands: Sender<Command>,

    // This event is signalled after a new entry is added to `commands`, so that the `run()`
    // method can be notified.
    pending_scheduled_event: Foundation::HANDLE,

    // Callback size in frames.
    period_frames: FrameCount,

    // QueryPerformanceFrequency result, cached at construction (constant for the system lifetime).
    qpc_frequency: u64,

    // Present for default-device streams; fires `ErrorKind::DeviceChanged` when the system
    // default changes. Dropped after the run thread joins, ensuring the HANDLE is not
    // waited on when it is closed.
    _default_device_monitor: Option<DefaultDeviceMonitor>,

    // Latch that ensures no callbacks fire before the caller receives the `Stream` handle.
    latch: Latch,
}

// SAFETY: Windows Event HANDLEs are safe to send between threads - they are designed for
// synchronization. All fields of Stream are Send:
// - JoinHandle<()> is Send
// - Sender<Command> is Send
// - Foundation::HANDLE is Send (Windows synchronization primitive)
// - Latch is Send
// See: https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createeventa
unsafe impl Send for Stream {}

// SAFETY: Windows Event HANDLEs are safe to access from multiple threads simultaneously.
// All synchronization operations (SetEvent, WaitForSingleObject) are thread-safe.
// All fields of Stream are Sync:
// - JoinHandle<()> is Sync
// - Sender<Command> is Sync (uses internal synchronization)
// - Foundation::HANDLE for event objects supports concurrent access
// - Latch is Sync
// The audio thread owns all COM objects, so no cross-thread COM access occurs.
unsafe impl Sync for Stream {}

// Compile-time assertion that Stream is Send and Sync
crate::assert_stream_send!(Stream);
crate::assert_stream_sync!(Stream);

struct RunContext {
    // Streams that have been created in this event loop.
    stream: StreamInner,

    // Handles corresponding to the `event` field of each element of `voices`. Must always be in
    // sync with `voices`, except that the first element is always `pending_scheduled_event`.
    handles: Vec<Foundation::HANDLE>,

    commands: Receiver<Command>,

    // Set by a device-change notification callback when SetEvent fails. The audio loop delivers
    // DeviceChanged on its next iteration.
    pending_device_changed: Option<Arc<AtomicBool>>,

    // Owned here so the worker thread closes it on exit in a self-join case.
    pending_scheduled_event: Foundation::HANDLE,
}

impl Drop for RunContext {
    fn drop(&mut self) {
        unsafe {
            let _ = Foundation::CloseHandle(self.pending_scheduled_event);
        }
    }
}

// Once we start running the eventloop, the RunContext will not be moved.
unsafe impl Send for RunContext {}

pub enum Command {
    PlayStream,
    PauseStream,
    Terminate,
}

pub enum AudioClientFlow {
    Render {
        render_client: Audio::IAudioRenderClient,
    },
    Capture {
        capture_client: Audio::IAudioCaptureClient,
    },
}

pub struct StreamInner {
    pub audio_client: Audio::IAudioClient,
    pub audio_clock: Audio::IAudioClock,
    pub client_flow: AudioClientFlow,
    // Event that is signalled by WASAPI whenever audio data must be written.
    pub event: Foundation::HANDLE,
    // True if the stream is currently playing. False if paused.
    pub playing: bool,
    // Number of frames of audio data in the underlying buffer allocated by WASAPI.
    pub max_frames_in_buffer: FrameCount,
    // Callback size in frames.
    pub period_frames: FrameCount,
    // Number of bytes that each frame occupies.
    pub bytes_per_frame: u16,
    // The configuration with which the stream was created.
    pub config: StreamConfig,
    // The sample format with which the stream was created.
    pub sample_format: SampleFormat,
    // Hardware pipeline latency.
    pub stream_latency: Duration,
}

impl Stream {
    pub(crate) fn new_input<D>(
        stream_inner: StreamInner,
        mut data_callback: D,
        error_callback: ErrorCallbackArc,
        default_device_monitor: Option<DefaultDeviceMonitor>,
    ) -> Result<Stream, Error>
    where
        D: FnMut(&Data, &InputCallbackInfo) + Send + 'static,
    {
        let pending_scheduled_event = unsafe {
            Threading::CreateEventA(None, false, false, windows::core::PCSTR(ptr::null()))
        }
        .expect("cpal: could not create input stream event");
        let (tx, rx) = channel();

        let period_frames = stream_inner.period_frames;
        let mut qpc_frequency: i64 = 0;
        unsafe {
            Performance::QueryPerformanceFrequency(&mut qpc_frequency)
                .expect("QueryPerformanceFrequency failed");
            debug_assert_ne!(qpc_frequency, 0, "QueryPerformanceFrequency returned zero");
        }

        let mut handles = vec![pending_scheduled_event, stream_inner.event];
        if let Some(ref monitor) = default_device_monitor {
            handles.push(monitor.event);
        }

        let pending_device_changed = default_device_monitor
            .as_ref()
            .map(|m| m.pending_device_changed.clone());
        let run_context = RunContext {
            handles,
            stream: stream_inner,
            commands: rx,
            pending_device_changed,
            pending_scheduled_event,
        };

        // The latch is released just before the `Stream` is returned so the worker cannot fire any
        // callbacks before the caller has the handle.
        let mut latch = Latch::new();
        let waiter = latch.waiter();

        let thread = thread::Builder::new()
            .name("cpal_wasapi_in".to_owned())
            .spawn(move || {
                waiter.wait();
                run_input(run_context, &mut data_callback, &error_callback)
            })
            .map_err(|e| {
                Error::with_message(
                    ErrorKind::ResourceExhausted,
                    format!("Failed to create audio thread: {e}"),
                )
            })?;

        latch.add_thread(thread.thread().clone());
        let stream = Stream {
            thread: Some(thread),
            commands: tx,
            pending_scheduled_event,
            period_frames,
            qpc_frequency: qpc_frequency as u64,
            _default_device_monitor: default_device_monitor,
            latch,
        };
        Ok(stream)
    }

    pub(crate) fn new_output<D>(
        stream_inner: StreamInner,
        mut data_callback: D,
        error_callback: ErrorCallbackArc,
        default_device_monitor: Option<DefaultDeviceMonitor>,
    ) -> Result<Stream, Error>
    where
        D: FnMut(&mut Data, &OutputCallbackInfo) + Send + 'static,
    {
        let pending_scheduled_event = unsafe {
            Threading::CreateEventA(None, false, false, windows::core::PCSTR(ptr::null()))
        }
        .expect("cpal: could not create output stream event");
        let (tx, rx) = channel();

        let period_frames = stream_inner.period_frames;
        let mut qpc_frequency: i64 = 0;
        unsafe {
            Performance::QueryPerformanceFrequency(&mut qpc_frequency)
                .expect("QueryPerformanceFrequency failed");
            debug_assert_ne!(qpc_frequency, 0, "QueryPerformanceFrequency returned zero");
        }

        let mut handles = vec![pending_scheduled_event, stream_inner.event];
        if let Some(ref monitor) = default_device_monitor {
            handles.push(monitor.event);
        }

        let pending_device_changed = default_device_monitor
            .as_ref()
            .map(|m| m.pending_device_changed.clone());
        let run_context = RunContext {
            handles,
            stream: stream_inner,
            commands: rx,
            pending_device_changed,
            pending_scheduled_event,
        };

        // The latch is released just before the `Stream` is returned so the worker cannot fire any
        // callbacks before the caller has the handle.
        let mut latch = Latch::new();
        let waiter = latch.waiter();

        let thread = thread::Builder::new()
            .name("cpal_wasapi_out".to_owned())
            .spawn(move || {
                waiter.wait();
                run_output(run_context, &mut data_callback, &error_callback)
            })
            .map_err(|e| {
                Error::with_message(
                    ErrorKind::ResourceExhausted,
                    format!("Failed to create audio thread: {e}"),
                )
            })?;

        latch.add_thread(thread.thread().clone());
        let stream = Stream {
            thread: Some(thread),
            commands: tx,
            pending_scheduled_event,
            period_frames,
            qpc_frequency: qpc_frequency as u64,
            _default_device_monitor: default_device_monitor,
            latch,
        };
        Ok(stream)
    }

    /// Releases the latch so the worker thread can begin processing audio callbacks.
    pub(crate) fn signal_ready(&self) {
        self.latch.release();
    }

    fn push_command(&self, command: Command) -> Result<(), SendError<Command>> {
        self.commands.send(command)?;
        unsafe {
            Threading::SetEvent(self.pending_scheduled_event).unwrap();
        }
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // Release the latch in case the stream is dropped before signal_ready() was called.
        self.signal_ready();

        let _ = self.push_command(Command::Terminate);
        if let Some(handle) = self.thread.take() {
            // Prevent self-join: Terminate was sent; the thread exits after the current callback
            // returns. pending_scheduled_event is closed by RunContext::drop on the worker thread,
            // covering both the self-join case (where we cannot join here) and the normal case
            // (where the thread exits and drops RunContext before join() returns).
            if handle.thread().id() != std::thread::current().id() {
                let _ = handle.join();
            }
        }
    }
}

impl StreamTrait for Stream {
    fn play(&self) -> Result<(), Error> {
        self.push_command(Command::PlayStream).map_err(|_| {
            Error::with_message(
                ErrorKind::StreamInvalidated,
                "Stream command channel closed",
            )
        })?;
        Ok(())
    }

    fn pause(&self) -> Result<(), Error> {
        self.push_command(Command::PauseStream).map_err(|_| {
            Error::with_message(
                ErrorKind::StreamInvalidated,
                "Stream command channel closed",
            )
        })?;
        Ok(())
    }

    fn now(&self) -> StreamInstant {
        let mut counter: i64 = 0;
        unsafe {
            Performance::QueryPerformanceCounter(&mut counter)
                .expect("QueryPerformanceCounter failed");
        }
        // Convert to 100-nanosecond units first, matching the precision of WASAPI QPCPosition
        // values delivered to callbacks. This keeps `now()` on the same 100 ns grid as
        // callback/capture/playback instants, avoiding false sub-100 ns deltas.
        let units_100ns = counter as u128 * 10_000_000 / self.qpc_frequency as u128;
        let nanos = units_100ns * 100;
        StreamInstant::new(
            (nanos / 1_000_000_000) as u64,
            (nanos % 1_000_000_000) as u32,
        )
    }

    fn buffer_size(&self) -> Result<FrameCount, Error> {
        Ok(self.period_frames)
    }
}

impl Drop for StreamInner {
    fn drop(&mut self) {
        unsafe {
            let _ = Foundation::CloseHandle(self.event);
        }
    }
}

// Process any pending commands that are queued within the `RunContext`.
// Returns `true` if the loop should continue running, `false` if it should terminate.
fn process_commands(run_context: &mut RunContext) -> Result<bool, Error> {
    // Process the pending commands.
    for command in run_context.commands.try_iter() {
        match command {
            Command::PlayStream => unsafe {
                if !run_context.stream.playing {
                    run_context
                        .stream
                        .audio_client
                        .Start()
                        .context("Failed to start audio client")?;
                    run_context.stream.playing = true;
                }
            },
            Command::PauseStream => unsafe {
                if run_context.stream.playing {
                    run_context
                        .stream
                        .audio_client
                        .Stop()
                        .context("Failed to stop audio client")?;
                    run_context.stream.playing = false;
                }
            },
            Command::Terminate => {
                return Ok(false);
            }
        }
    }

    Ok(true)
}
// Wait for any of the given handles to be signalled.
//
// Returns the index of the `handle` that was signalled, or an `Err` if
// `WaitForMultipleObjectsEx` fails.
//
// This is called when the `run` thread is ready to wait for the next event. The
// next event might be some command submitted by the user (the first handle) or
// might indicate that one of the streams is ready to deliver or receive audio.
fn wait_for_handle_signal(handles: &[Foundation::HANDLE]) -> Result<usize, Error> {
    debug_assert!(handles.len() <= SystemServices::MAXIMUM_WAIT_OBJECTS as usize);
    let result = unsafe {
        Threading::WaitForMultipleObjectsEx(
            handles,
            false,               // Don't wait for all, just wait for the first
            Threading::INFINITE, // TODO: allow setting a timeout
            false,               // irrelevant parameter here
        )
    };
    if result == Foundation::WAIT_FAILED {
        return Err(Error::with_message(
            ErrorKind::StreamInvalidated,
            "Failed to wait for audio event",
        ));
    }
    // Notifying the corresponding task handler.
    let handle_idx = (result.0 - WAIT_OBJECT_0.0) as usize;
    Ok(handle_idx)
}

// Get the number of available frames that are available for writing/reading.
#[inline]
fn get_available_frames(stream: &StreamInner) -> Result<FrameCount, Error> {
    unsafe {
        let padding = stream
            .audio_client
            .GetCurrentPadding()
            .context("Failed to get current padding")?;
        Ok(stream.max_frames_in_buffer - padding)
    }
}

fn run_input(
    mut run_ctxt: RunContext,
    data_callback: &mut dyn FnMut(&Data, &InputCallbackInfo),
    error_callback: &ErrorCallbackArc,
) {
    #[cfg(feature = "realtime")]
    if let Err(err) = boost_current_thread_priority(
        run_ctxt.stream.period_frames,
        run_ctxt.stream.config.sample_rate,
    ) {
        emit_error(error_callback, err);
    }

    // WASAPI marks silent capture packets instead of promising that their
    // backing bytes contain equilibrium samples. Allocate and initialise one
    // maximum-sized scratch packet outside the realtime loop so callbacks can
    // receive valid silence without reading or allocating per packet.
    let silent_byte_count = run_ctxt.stream.max_frames_in_buffer as usize
        * run_ctxt.stream.bytes_per_frame as usize;
    let mut silent_storage = vec![0_u64; silent_byte_count.div_ceil(mem::size_of::<u64>())];
    let silent_buffer = aligned_byte_slice(&mut silent_storage, silent_byte_count);
    fill_equilibrium(silent_buffer, run_ctxt.stream.sample_format);

    loop {
        match process_commands_and_await_signal(&mut run_ctxt, error_callback) {
            ControlFlow::Break(()) => break,
            ControlFlow::Continue(false) => continue,
            ControlFlow::Continue(true) => {}
        }
        let capture_client = match run_ctxt.stream.client_flow {
            AudioClientFlow::Capture { ref capture_client } => capture_client.clone(),
            _ => unreachable!(),
        };
        if let Err(err) = process_input(
            &run_ctxt.stream,
            capture_client,
            data_callback,
            &mut *silent_buffer,
        ) {
            emit_error(error_callback, err);
            break;
        }
    }
}

fn run_output(
    mut run_ctxt: RunContext,
    data_callback: &mut dyn FnMut(&mut Data, &OutputCallbackInfo),
    error_callback: &ErrorCallbackArc,
) {
    #[cfg(feature = "realtime")]
    if let Err(err) = boost_current_thread_priority(
        run_ctxt.stream.period_frames,
        run_ctxt.stream.config.sample_rate,
    ) {
        emit_error(error_callback, err);
    }

    loop {
        match process_commands_and_await_signal(&mut run_ctxt, error_callback) {
            ControlFlow::Break(()) => break,
            ControlFlow::Continue(false) => continue,
            ControlFlow::Continue(true) => {}
        }
        let render_client = match run_ctxt.stream.client_flow {
            AudioClientFlow::Render { ref render_client } => render_client.clone(),
            _ => unreachable!(),
        };
        if let Err(err) = process_output(&run_ctxt.stream, render_client, data_callback) {
            emit_error(error_callback, err);
            break;
        }
    }
}

/// Attempts to elevate the current thread to real-time or high-priority scheduling.
#[cfg(feature = "realtime")]
fn boost_current_thread_priority(
    period_frames: FrameCount,
    sample_rate: SampleRate,
) -> Result<(), Error> {
    match audio_thread_priority::promote_current_thread_to_real_time(period_frames, sample_rate) {
        Ok(_) => Ok(()),
        Err(_) => unsafe {
            let thread_handle = Threading::GetCurrentThread();
            Threading::SetThreadPriority(thread_handle, Threading::THREAD_PRIORITY_TIME_CRITICAL)
                .context("Failed to promote audio thread to real-time priority")
        },
    }
}

fn process_commands_and_await_signal(
    run_context: &mut RunContext,
    error_callback: &ErrorCallbackArc,
) -> ControlFlow<(), bool> {
    // Process queued commands.
    match process_commands(run_context) {
        Ok(true) => (),
        Ok(false) => return ControlFlow::Break(()),
        Err(err) => {
            emit_error(error_callback, err);
            return ControlFlow::Break(());
        }
    };

    if let Some(ref flag) = run_context.pending_device_changed {
        if flag.swap(false, Ordering::Relaxed) {
            emit_error(
                error_callback,
                Error::with_message(ErrorKind::DeviceChanged, "Default audio device changed"),
            );
        }
    }

    // Wait for any of the handles to be signalled.
    let handle_idx = match wait_for_handle_signal(&run_context.handles) {
        Ok(idx) => idx,
        Err(err) => {
            emit_error(error_callback, err);
            return ControlFlow::Break(());
        }
    };

    // Handle layout: 0 = pending_scheduled_event (commands), 1 = WASAPI audio event,
    // 2+ = default-device change event (only present for default-device streams).
    // Continue(true)  = audio event fired, proceed to process audio this iteration.
    // Continue(false) = command or device-change event, loop around and wait again.
    if handle_idx >= 2 {
        emit_error(
            error_callback,
            Error::with_message(ErrorKind::DeviceChanged, "Default audio device changed"),
        );
        return ControlFlow::Continue(false);
    }
    ControlFlow::Continue(handle_idx != 0)
}

// The loop for processing pending input data.
fn process_input(
    stream: &StreamInner,
    capture_client: Audio::IAudioCaptureClient,
    data_callback: &mut dyn FnMut(&Data, &InputCallbackInfo),
    silent_buffer: &mut [u8],
) -> Result<(), Error> {
    unsafe {
        // Get the available data in the shared buffer.
        let mut buffer: *mut u8 = ptr::null_mut();
        loop {
            let mut frames_available = match capture_client.GetNextPacketSize() {
                Ok(0) => return Ok(()),
                Ok(f) => f,
                Err(err) => return Err(Error::from(err)),
            };
            let mut qpc_position: u64 = 0;
            let mut flags = mem::MaybeUninit::uninit();
            let result = capture_client.GetBuffer(
                &mut buffer,
                &mut frames_available,
                flags.as_mut_ptr(),
                None,
                Some(&mut qpc_position),
            );

            match result {
                // TODO: Can this happen?
                Err(e) if e.code() == Audio::AUDCLNT_S_BUFFER_EMPTY => continue,
                Err(e) => return Err(Error::from(e)),
                Ok(_) => (),
            }

            // Once GetBuffer succeeds, WASAPI requires exactly one matching
            // ReleaseBuffer even if packet validation or timestamp creation
            // fails. Keep all fallible processing inside one result, release
            // first, then propagate the original error.
            let packet_result = (|| -> Result<(), Error> {
                let byte_count = frames_available as usize * stream.bytes_per_frame as usize;
                let packet_flags = flags.assume_init();
                let data = capture_packet_source(buffer, byte_count, packet_flags, silent_buffer)?
                    as *mut ();
                let len = byte_count / stream.sample_format.sample_size();
                let data = Data::from_parts(data, len, stream.sample_format);

                // The `qpc_position` is in 100 nanosecond units. Convert it to nanoseconds.
                let timestamp = input_timestamp(stream, qpc_position)?;
                let info = InputCallbackInfo { timestamp };
                data_callback(&data, &info);
                Ok(())
            })();
            let release_result = capture_client
                .ReleaseBuffer(frames_available)
                .context("Failed to release capture buffer");
            packet_result?;
            release_result?;
        }
    }
}

/// Chooses bytes for one WASAPI capture packet. Silent packets may expose a
/// null pointer or undefined values, so they must use the pre-filled
/// equilibrium scratch buffer without inspecting the native pointer.
fn capture_packet_source(
    native_buffer: *mut u8,
    byte_count: usize,
    flags: u32,
    silent_buffer: &mut [u8],
) -> Result<*mut u8, Error> {
    const SILENT_FLAG: u32 = Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32;
    if flags & SILENT_FLAG != 0 {
        if byte_count > silent_buffer.len() {
            return Err(Error::with_message(
                ErrorKind::StreamInvalidated,
                "WASAPI silent packet exceeded the capture buffer",
            ));
        }
        return Ok(silent_buffer.as_mut_ptr());
    }
    if native_buffer.is_null() {
        return Err(Error::with_message(
            ErrorKind::StreamInvalidated,
            "WASAPI returned a null non-silent capture buffer",
        ));
    }
    Ok(native_buffer)
}

/// Views 8-byte-aligned storage as the exact packet-sized byte slice required
/// by `Data::from_parts`. Every CPAL sample format used by WASAPI has an
/// alignment of at most eight bytes.
fn aligned_byte_slice(storage: &mut [u64], byte_count: usize) -> &mut [u8] {
    assert!(byte_count <= mem::size_of_val(storage));
    // SAFETY: `storage` owns at least `byte_count` initialized bytes and its
    // u64 allocation provides sufficient alignment for all sample formats.
    unsafe { std::slice::from_raw_parts_mut(storage.as_mut_ptr().cast::<u8>(), byte_count) }
}

// The loop for writing output data.
fn process_output(
    stream: &StreamInner,
    render_client: Audio::IAudioRenderClient,
    data_callback: &mut dyn FnMut(&mut Data, &OutputCallbackInfo),
) -> Result<(), Error> {
    // The number of frames available for writing.
    let frames_available = match get_available_frames(stream)? {
        0 => return Ok(()), // TODO: Can this happen?
        n => n,
    };

    unsafe {
        let buffer = render_client
            .GetBuffer(frames_available)
            .map_err(Error::from)?;

        debug_assert!(!buffer.is_null());

        let byte_count = frames_available as usize * stream.bytes_per_frame as usize;
        let buffer_slice = std::slice::from_raw_parts_mut(buffer, byte_count);
        fill_equilibrium(buffer_slice, stream.sample_format);

        let data = buffer as *mut ();
        let len = byte_count / stream.sample_format.sample_size();
        let mut data = Data::from_parts(data, len, stream.sample_format);
        let sample_rate = stream.config.sample_rate;
        let timestamp = output_timestamp(stream, frames_available, sample_rate)?;
        let info = OutputCallbackInfo { timestamp };
        data_callback(&mut data, &info);

        render_client
            .ReleaseBuffer(frames_available, 0)
            .map_err(Error::from)?;
    }

    Ok(())
}

/// Use the stream's `IAudioClock` to produce the current stream instant.
///
/// Uses the QPC position produced via the `GetPosition` method.
#[inline]
fn stream_instant(stream: &StreamInner) -> Result<StreamInstant, Error> {
    let mut position: u64 = 0;
    let mut qpc_position: u64 = 0;
    unsafe {
        stream
            .audio_clock
            .GetPosition(&mut position, Some(&mut qpc_position))
            .context("Failed to get clock position")?;
    };
    // The `qpc_position` is in 100-nanosecond units.
    let nanos = qpc_position as u128 * 100;
    let instant = StreamInstant::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    );
    Ok(instant)
}

/// Produce the input stream timestamp.
///
/// `buffer_qpc_position` is the `qpc_position` returned via the `GetBuffer` call on the capture
/// client. It represents the instant at which the first sample of the retrieved buffer was
/// captured.
#[inline]
fn input_timestamp(
    stream: &StreamInner,
    buffer_qpc_position: u64,
) -> Result<InputStreamTimestamp, Error> {
    // The `qpc_position` is in 100-nanosecond units.
    let nanos = buffer_qpc_position as u128 * 100;
    let capture = StreamInstant::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    );
    let callback = stream_instant(stream)?;
    Ok(InputStreamTimestamp { capture, callback })
}

/// Produce the output stream timestamp.
///
/// `frames_available` is the number of frames available for writing as reported by subtracting the
/// result of `GetCurrentPadding` from the maximum buffer size.
///
/// `sample_rate` is the rate at which audio frames are processed by the device.
#[inline]
fn output_timestamp(
    stream: &StreamInner,
    frames_available: FrameCount,
    sample_rate: SampleRate,
) -> Result<OutputStreamTimestamp, Error> {
    let callback = stream_instant(stream)?;
    // `padding` is the number of frames already queued in the endpoint buffer ahead of the
    // frames we are about to write. Those frames must drain before ours are heard.
    let padding = stream.max_frames_in_buffer - frames_available;
    let playback = callback + (frames_to_duration(padding, sample_rate) + stream.stream_latency);
    Ok(OutputStreamTimestamp { callback, playback })
}

#[cfg(test)]
mod capture_packet_tests {
    use super::*;

    const SILENT_FLAG: u32 = Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32;

    fn silence(byte_count: usize, format: SampleFormat) -> (Vec<u64>, usize) {
        let mut storage = vec![0_u64; byte_count.div_ceil(mem::size_of::<u64>())];
        let bytes = aligned_byte_slice(&mut storage, byte_count);
        fill_equilibrium(bytes, format);
        (storage, byte_count)
    }

    #[test]
    fn silent_packet_uses_equilibrium_without_reading_native_bytes() {
        let (mut storage, byte_count) = silence(8, SampleFormat::U8);
        let silence = aligned_byte_slice(&mut storage, byte_count);

        let source = capture_packet_source(
            ptr::null_mut(),
            silence.len(),
            SILENT_FLAG,
            silence,
        )
        .unwrap();

        assert_eq!(source, silence.as_mut_ptr());
        assert_eq!(source as usize % mem::align_of::<u64>(), 0);
        assert!(silence.iter().all(|sample| *sample == 0x80));
    }

    #[test]
    fn multibyte_unsigned_silence_uses_the_format_equilibrium() {
        let (mut storage, byte_count) = silence(4, SampleFormat::U16);
        let silence = aligned_byte_slice(&mut storage, byte_count);

        let source = capture_packet_source(
            ptr::null_mut(),
            silence.len(),
            SILENT_FLAG,
            silence,
        )
        .unwrap();

        assert_eq!(source, silence.as_mut_ptr());
        assert_eq!(u16::from_ne_bytes([silence[0], silence[1]]), 32_768);
    }

    #[test]
    fn non_silent_packet_keeps_the_wasapi_buffer() {
        let mut native = [7_u8; 8];
        let mut silence = [0_u8; 8];

        let source =
            capture_packet_source(native.as_mut_ptr(), native.len(), 0, &mut silence).unwrap();

        assert_eq!(source, native.as_mut_ptr());
    }

    #[test]
    fn oversized_silent_packet_is_rejected() {
        let mut silence = [0_u8; 8];
        assert!(capture_packet_source(
            ptr::null_mut(),
            silence.len() + 1,
            SILENT_FLAG,
            &mut silence,
        )
        .is_err());
    }
}
