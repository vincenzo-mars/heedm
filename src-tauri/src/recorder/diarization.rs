use serde::{Deserialize, Serialize};

use super::audio::rms;

const WINDOW_SECS: f64 = 0.2;
const DOMINANCE_RATIO: f32 = 1.5;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpeakerLabel {
    Mic,
    Sys,
    Both,
}

impl SpeakerLabel {
    /// Stessa rappresentazione del `#[serde(rename_all = "lowercase")]` sopra —
    /// usata per popolare `TranscriptSegment.speaker: Option<String>` senza
    /// passare per un round-trip JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpeakerLabel::Mic => "mic",
            SpeakerLabel::Sys => "sys",
            SpeakerLabel::Both => "both",
        }
    }

    fn from_energies(mic_rms: f32, sys_rms: f32) -> Self {
        if mic_rms > sys_rms * DOMINANCE_RATIO {
            SpeakerLabel::Mic
        } else if sys_rms > mic_rms * DOMINANCE_RATIO {
            SpeakerLabel::Sys
        } else {
            SpeakerLabel::Both
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SpeakerInterval {
    pub start: f64,
    pub end: f64,
    pub label: SpeakerLabel,
}

/// Stima un timeline di attività mic-vs-sistema confrontando l'energia RMS dei due
/// buffer grezzi (pre-mix) a finestre fisse, poi accorpa finestre consecutive con
/// la stessa etichetta in intervalli — formato compatto da serializzare come sidecar.
pub fn estimate_timeline(mic: &[f32], sys: &[f32], sample_rate: u32) -> Vec<SpeakerInterval> {
    let window_len = (WINDOW_SECS * sample_rate as f64).round() as usize;
    if window_len == 0 {
        return Vec::new();
    }

    let total_len = mic.len().max(sys.len());
    let mut intervals: Vec<SpeakerInterval> = Vec::new();

    let mut offset = 0usize;
    while offset < total_len {
        let end = (offset + window_len).min(total_len);
        let mic_window = mic.get(offset..mic.len().min(end)).unwrap_or(&[]);
        let sys_window = sys.get(offset..sys.len().min(end)).unwrap_or(&[]);

        let label = SpeakerLabel::from_energies(rms(mic_window), rms(sys_window));
        let start_secs = offset as f64 / sample_rate as f64;
        let end_secs = end as f64 / sample_rate as f64;

        match intervals.last_mut() {
            Some(last) if last.label == label => last.end = end_secs,
            _ => intervals.push(SpeakerInterval {
                start: start_secs,
                end: end_secs,
                label,
            }),
        }

        offset = end;
    }

    intervals
}
