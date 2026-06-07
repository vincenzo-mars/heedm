use std::sync::{Arc, Mutex};

use crate::recorder::SysAudioStop;

pub struct WindowsCapture;

impl SysAudioStop for WindowsCapture {
    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

pub fn start(
    _samples: Arc<Mutex<Vec<f32>>>,
    _sample_rate: u32,
    _channels: u16,
) -> Result<Box<dyn SysAudioStop>, String> {
    Err("Windows system audio capture not yet implemented".to_string())
}
