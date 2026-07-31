//! Primitive audio condivise da recorder, AEC, diarizzazione e comandi:
//! energia, downmix, resampling, mix e scrittura WAV. Vivono qui perché
//! `rms` e la conversione a i16 erano duplicate in tre punti diversi.

use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::Path;

/// Rate unico per registrazione *e* trascrizione: whisper.cpp richiede 16kHz
/// (`read_wav` in `common.cpp`), e registrare direttamente a questo rate
/// elimina il resampling a valle per le nuove registrazioni.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Downmix di un buffer interleaved a mono. Prende ownership per restituire il
/// buffer intatto quando è già mono, senza copiarlo.
pub fn to_mono(samples: Vec<f32>, channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples;
    }
    samples
        .chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Resample lineare puro-Rust: gestisce rate arbitrari in ingresso, inclusi
/// rapporti non interi (es. 44100 -> 16000). Sufficiente per parlato/ASR, non
/// per audio musicale ad alta fedeltà.
///
/// Assume un buffer **mono**: su audio interleaved interpolerebbe fra canali
/// diversi, quindi va sempre chiamato dopo `to_mono`.
pub fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

pub fn mix(mic: &[f32], sys: &[f32]) -> Vec<f32> {
    let len = mic.len().max(sys.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let m = mic.get(i).copied().unwrap_or(0.0);
        let s = sys.get(i).copied().unwrap_or(0.0);
        out.push((m + s).clamp(-1.0, 1.0));
    }
    out
}

fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

pub fn write_wav(
    samples: &[f32],
    path: &Path,
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        writer.write_sample(to_i16(s)).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

/// Scrive un WAV a 16 bit su un buffer in memoria invece che su file.
pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>, String> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut buf, spec).map_err(|e| e.to_string())?;
        for &s in samples {
            writer.write_sample(to_i16(s)).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}
