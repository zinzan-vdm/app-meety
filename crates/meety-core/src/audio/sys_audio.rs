//! System audio capture trait abstraction.
//!
//! Defines the `SystemAudioCapture` trait that each platform backend
//! implements. The factory function `start_system_capture` dispatches
//! to the correct backend based on the target OS.

use std::sync::Arc;

use crate::audio::wav_writer::AudioWavWriter;
use crate::error::Result;

/// Trait for system audio capture backends.
///
/// Each platform (macOS, Windows, Linux) provides a concrete type that
/// implements this trait. The `CaptureSession` holds a
/// `Box<dyn SystemAudioCapture>` and calls `stop()` when done.
pub trait SystemAudioCapture: Send {
    /// Stop capturing system audio and finalize the WAV file.
    ///
    /// The backend must flush any buffered samples, stop the underlying
    /// capture stream, and finalize the WAV writer (write header, close
    /// file). After this call the capture is fully torn down.
    fn stop(self: Box<Self>) -> Result<()>;
}

/// Start system audio capture on the current platform.
///
/// Dispatches to the platform-specific backend:
/// - macOS: Process Tap (14.4+) → ScreenCaptureKit fallback
/// - Windows: WASAPI loopback via cpal
/// - Linux: PulseAudio monitor source (@DEFAULT_MONITOR@) via pulse crate
pub fn start_system_capture(
    writer: Arc<AudioWavWriter>,
    target_sample_rate: u32,
) -> Result<Box<dyn SystemAudioCapture>> {
    crate::audio::system::dispatch_start(writer, target_sample_rate)
}