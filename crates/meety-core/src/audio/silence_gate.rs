//! Sustained-silence gate for system-audio capture.
//!
//! macOS's ScreenCaptureKit / process-tap path pauses WAV writes after
//! a sustained period of silence (30 s), then resumes automatically when
//! audio returns. This keeps recordings from bloating with silent
//! stretches during quiet meeting moments. Windows and Linux capture
//! streams previously lacked this; the gate below is the shared,
//! alloc-free implementation all three backends use.
//!
//! The gate is safe to call from a realtime audio callback: it only
//! touches atomics and computes RMS inline (no allocation, no locks,
//! no syscalls).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Below this RMS, a chunk is treated as silence.
const SILENCE_RMS_THRESHOLD: f32 = 0.002;

/// How long audio must stay below the threshold before writes pause.
const SILENCE_PAUSE_AFTER_MS: u64 = 30_000;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub struct SilenceGate {
    last_active_ms: AtomicU64,
    paused: AtomicBool,
}

impl SilenceGate {
    pub fn new() -> Self {
        Self {
            last_active_ms: AtomicU64::new(now_ms()),
            paused: AtomicBool::new(false),
        }
    }

    /// Feed one chunk of audio. Returns `true` when the chunk should be
    /// skipped (sustained silence has paused writes and this chunk is
    /// still silent), `false` when it should be written.
    ///
    /// The chunk is treated as a whole: any RMS above the threshold
    /// marks the stream active and resumes paused writes immediately.
    pub fn should_skip(&self, samples: &[f32]) -> bool {
        let now = now_ms();
        if rms(samples) >= SILENCE_RMS_THRESHOLD {
            self.last_active_ms.store(now, Ordering::Relaxed);
            if self.paused.swap(false, Ordering::Relaxed) {
                tracing::info!(
                    rms = rms(samples),
                    threshold = SILENCE_RMS_THRESHOLD,
                    "system audio resumed — leaving silence pause"
                );
            }
            false
        } else {
            let last_active = self.last_active_ms.load(Ordering::Relaxed);
            let silent_for = now.saturating_sub(last_active);
            if silent_for >= SILENCE_PAUSE_AFTER_MS {
                if !self.paused.swap(true, Ordering::Relaxed) {
                    tracing::info!(
                        silent_for_ms = silent_for,
                        threshold = SILENCE_RMS_THRESHOLD,
                        "system audio paused after sustained silence — skipping WAV writes until audio returns"
                    );
                }
            }
            self.paused.load(Ordering::Relaxed)
        }
    }

    /// Whether writes are currently paused (diagnostics only).
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

impl Default for SilenceGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_empty_is_zero() {
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn rms_of_silence_is_below_threshold() {
        let silent = vec![0.0_f32; 4096];
        assert!(rms(&silent) < SILENCE_RMS_THRESHOLD);
    }

    #[test]
    fn rms_of_full_scale_sine_is_above_threshold() {
        let n = 48_000 / 1_000;
        let pcm: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / n as f32).sin())
            .collect();
        assert!(rms(&pcm) > 0.5);
    }

    #[test]
    fn loud_chunk_is_never_skipped() {
        let gate = SilenceGate::new();
        let n = 48_000 / 1_000;
        let loud: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / n as f32).sin())
            .collect();
        for _ in 0..10 {
            assert!(!gate.should_skip(&loud));
        }
        assert!(!gate.paused());
    }

    #[test]
    fn silence_does_not_pause_immediately() {
        let gate = SilenceGate::new();
        let silent = vec![0.0_f32; 1024];

        // A fresh gate's last-active timestamp is "now", so a silent
        // chunk within the 30 s window is still written (matches the
        // macOS behaviour: silence is recorded briefly, then paused).
        assert!(!gate.should_skip(&silent));
        assert!(!gate.paused());
    }

    #[test]
    fn silence_eventually_pauses_and_resumes() {
        let gate = SilenceGate::new();
        let silent = vec![0.0_f32; 1024];
        let loud = vec![0.5_f32; 1024];

        // Manually age the last-active timestamp to before the window.
        let past = now_ms().saturating_sub(SILENCE_PAUSE_AFTER_MS + 1000);
        gate.last_active_ms.store(past, Ordering::Relaxed);

        // A silent chunk now crosses the window → pause and skip.
        assert!(gate.should_skip(&silent));
        assert!(gate.paused());

        // Subsequent silence stays paused.
        assert!(gate.should_skip(&silent));
        assert!(gate.paused());

        // Loud audio resumes writes immediately.
        assert!(!gate.should_skip(&loud));
        assert!(!gate.paused());
    }
}
