use std::sync::{Arc, Mutex};

use screencapturekit::prelude::*;

use crate::recorder::SysAudioStop;

struct AudioHandler {
    samples: Arc<Mutex<Vec<f32>>>,
}

impl SCStreamOutputTrait for AudioHandler {
    fn did_output_sample_buffer(
        &self,
        sample_buffer: CMSampleBuffer,
        of_type: SCStreamOutputType,
    ) {
        if of_type != SCStreamOutputType::Audio {
            return;
        }
        if let Some(buf_list) = sample_buffer.audio_buffer_list() {
            for buf in buf_list.iter() {
                let bytes = buf.data();
                let mut guard = self.samples.lock().unwrap();
                guard.reserve(bytes.len() / 4);
                for chunk in bytes.chunks_exact(4) {
                    guard.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                }
            }
        }
    }
}

struct MacOSCapture {
    stream: SCStream,
}

impl SysAudioStop for MacOSCapture {
    fn stop(&mut self) -> Result<(), String> {
        self.stream.stop_capture().map_err(|e| e.to_string())
    }
}

pub fn start(
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
) -> Result<Box<dyn SysAudioStop>, String> {
    let content = SCShareableContent::get().map_err(|e| e.to_string())?;
    let displays = content.displays();
    let display = displays.first().ok_or("No display found")?;

    let filter = SCContentFilter::create()
        .with_display(display)
        .with_excluding_windows(&[])
        .build();

    let config = SCStreamConfiguration::new()
        .with_captures_audio(true)
        .with_sample_rate(sample_rate as i32)
        .with_channel_count(channels as i32);

    let mut stream = SCStream::new(&filter, &config);
    stream.add_output_handler(AudioHandler { samples }, SCStreamOutputType::Audio);
    stream.start_capture().map_err(|e| e.to_string())?;

    Ok(Box::new(MacOSCapture { stream }))
}
