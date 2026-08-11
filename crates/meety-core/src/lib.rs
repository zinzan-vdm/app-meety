pub mod audio;
pub mod cloud_guard;
pub mod diarization;
pub mod encryption;
pub mod error;
pub mod live_notes;
pub mod llm;
pub mod mcp_server;
pub mod memory;
pub mod paths;
pub mod permissions;
pub mod qos;
pub mod recipes;
pub mod server;
pub mod speaker_memory;
pub mod storage;
pub mod text;
pub mod transcription;
pub mod user_profile;

pub use error::{MeetyError, Result};

// On Linux, the voice_activity_detector crate's static LazyLock holding an ort
// Session and the ort crate's global Environment trigger a harmless
// free(): invalid pointer during libonnxruntime.so's internal cleanup at
// process exit. All tests pass correctly — this is a cosmetic atexit-ordering
// issue. We catch SIGABRT during the exit phase (via an atexit hook) and call
// _exit(0) so the crash doesn't kill CI. The handler is only armed during
// process teardown, so genuine crashes during test execution still abort.
#[cfg(target_os = "linux")]
mod exit {
    use std::sync::Once;

    extern "C" {
        fn atexit(handler: extern "C" fn()) -> i32;
        fn signal(sig: i32, handler: unsafe extern "C" fn()) -> usize;
        fn _exit(status: i32) -> !;
    }

    const SIGABRT: i32 = 6;

    unsafe extern "C" fn handle_sigabrt() {
        // Terminate immediately without running further atexit handlers.
        // The "free(): invalid pointer" was already printed to stderr by
        // glibc's malloc_printerr before abort() was called.
        _exit(0);
    }

    extern "C" fn on_exit() {
        // Arm the SIGABRT handler just before _dl_fini runs, so the
        // harmless exit-time crash is swallowed but nothing else is.
        unsafe { signal(SIGABRT, handle_sigabrt); }
    }

    pub fn suppress() {
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            atexit(on_exit);
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn suppress_exit_sigabrt() {
        #[cfg(target_os = "linux")]
        crate::exit::suppress();
    }
}