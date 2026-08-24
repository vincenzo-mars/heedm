use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use std::sync::{Arc, Mutex};

pub struct MicInfo {
    pub stream: Stream,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Un solo builder per tutti i formati campione: la conversione a f32 la fa
/// dasp (via `cpal::FromSample`), che gestisce correttamente anche l'offset
/// dei formati unsigned.
fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            samples
                .lock()
                .unwrap()
                .extend(data.iter().map(|&s| f32::from_sample(s)));
        },
        |e| eprintln!("mic stream error: {e}"),
        None,
    )
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

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(&device, &stream_config, samples),
        SampleFormat::I16 => build_stream::<i16>(&device, &stream_config, samples),
        SampleFormat::U16 => build_stream::<u16>(&device, &stream_config, samples),
        _ => return Err(format!("Unsupported mic sample format: {sample_format:?}")),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    Ok(MicInfo { stream, sample_rate, channels })
}
