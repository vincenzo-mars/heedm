pub mod aec;
pub mod diarization;
pub mod mic;
pub mod mixer;
pub mod system_audio;
pub mod writer;

use std::sync::{Arc, Mutex};

pub trait SysAudioStop: Send {
    fn stop(&mut self) -> Result<(), String>;
}

pub struct RecorderInner {
    pub is_recording: bool,
    pub start_time: Option<std::time::Instant>,
    pub mic_samples: Arc<Mutex<Vec<f32>>>,
    pub sys_samples: Arc<Mutex<Vec<f32>>>,
    pub mic_stream: Option<cpal::Stream>,
    pub sys_capture: Option<Box<dyn SysAudioStop>>,
    /// Rate di *output* della registrazione (fisso, = `TARGET_SAMPLE_RATE`):
    /// usato da mixer/diarizzazione/writer. Il microfono cattura al suo rate
    /// nativo (`mic_native_rate`) e viene ricampionato a questo in `stop_recording`.
    pub sample_rate: u32,
    /// Rate nativo del device microfono al momento della cattura (cpal,
    /// dinamico — dipende dall'hardware). Serve solo per ricampionare il
    /// buffer mic verso `sample_rate` a fine registrazione.
    pub mic_native_rate: u32,
    pub channels: u16,
}

impl RecorderInner {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            start_time: None,
            mic_samples: Arc::new(Mutex::new(Vec::new())),
            sys_samples: Arc::new(Mutex::new(Vec::new())),
            mic_stream: None,
            sys_capture: None,
            sample_rate: 16_000,
            mic_native_rate: 48_000,
            channels: 2,
        }
    }
}

pub struct RecorderState(pub tokio::sync::Mutex<RecorderInner>);

impl RecorderState {
    pub fn new() -> Self {
        Self(tokio::sync::Mutex::new(RecorderInner::new()))
    }
}
