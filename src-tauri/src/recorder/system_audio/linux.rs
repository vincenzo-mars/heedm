use std::sync::{Arc, Mutex};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};

use crate::recorder::SysAudioStop;

struct LinuxCapture {
    _stream: cpal::Stream,
}

impl SysAudioStop for LinuxCapture {
    fn stop(&mut self) -> Result<(), String> {
        Ok(()) // drop stops the stream
    }
}

pub fn start(
    samples: Arc<Mutex<Vec<f32>>>,
    _sample_rate: u32,
    _channels: u16,
) -> Result<Box<dyn SysAudioStop>, String> {
    let host = cpal::default_host();

    let device = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .find(|d| {
            d.name()
                .map(|n| n.to_lowercase().contains("monitor"))
                .unwrap_or(false)
        })
        .ok_or("No PulseAudio/PipeWire monitor source found")?;

    let config = device
        .default_input_config()
        .map_err(|e| e.to_string())?;
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let err_fn = |e| eprintln!("sys audio stream error: {e}");

    let stream = match sample_format {
        SampleFormat::F32 => {
            let buf = samples.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| buf.lock().unwrap().extend_from_slice(data),
                err_fn,
                None,
            )
        }
        SampleFormat::I16 => {
            let buf = samples.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let mut g = buf.lock().unwrap();
                    for &s in data {
                        g.push(s as f32 / i16::MAX as f32);
                    }
                },
                err_fn,
                None,
            )
        }
        _ => return Err(format!("Unsupported sample format: {sample_format:?}")),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    Ok(Box::new(LinuxCapture { _stream: stream }))
}
