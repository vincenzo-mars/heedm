#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

use std::sync::{Arc, Mutex};

use crate::recorder::SysAudioStop;

pub fn start(
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
) -> Result<Box<dyn SysAudioStop>, String> {
    #[cfg(target_os = "macos")]
    return macos::start(samples, sample_rate, channels);

    #[cfg(target_os = "windows")]
    return windows::start(samples, sample_rate, channels);

    #[cfg(target_os = "linux")]
    return linux::start(samples, sample_rate, channels);

    #[allow(unreachable_code)]
    Err("System audio capture not supported on this platform".to_string())
}
