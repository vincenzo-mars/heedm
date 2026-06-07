use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};
use std::sync::{Arc, Mutex};

pub struct MicInfo {
    pub stream: Stream,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn start_mic_capture(samples: Arc<Mutex<Vec<f32>>>) -> Result<MicInfo, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device found")?;

    let config = device
        .default_input_config()
        .map_err(|e| e.to_string())?;

    let sample_rate = config.sample_rate();
    let channels = config.channels();
    let sample_format = config.sample_format();
    let stream_config: StreamConfig = config.into();

    let err_fn = |e| eprintln!("mic stream error: {e}");

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
        SampleFormat::U16 => {
            let buf = samples.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    let mut g = buf.lock().unwrap();
                    for &s in data {
                        g.push((s as f32 / u16::MAX as f32) * 2.0 - 1.0);
                    }
                },
                err_fn,
                None,
            )
        }
        _ => return Err(format!("Unsupported mic sample format: {sample_format:?}")),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    Ok(MicInfo { stream, sample_rate, channels })
}
