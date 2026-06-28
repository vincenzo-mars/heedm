// ponytail: simplified offline AEC — delay estimation via cross-correlation + least-squares subtraction.
// Covers the dominant echo path (direct acoustic). For multi-path reverb, use webrtc-audio-processing.

const MAX_DELAY_MS: usize = 200;
const CORR_WINDOW: usize = 32_000; // 2s at 16kHz — enough to find delay without scanning full buffer
const MIN_CORR: f32 = 0.10; // skip AEC if echo is negligible

pub fn cancel_echo(mic: &[f32], sys: &[f32], sample_rate: u32) -> Vec<f32> {
    let max_lag = (MAX_DELAY_MS * sample_rate as usize / 1000).min(sys.len());
    let (delay, corr) = find_delay(mic, sys, max_lag);
    if corr < MIN_CORR {
        return mic.to_vec();
    }
    let alpha = estimate_alpha(mic, sys, delay);
    subtract(mic, sys, delay, alpha)
}

fn find_delay(mic: &[f32], sys: &[f32], max_lag: usize) -> (usize, f32) {
    let mic_rms = rms(mic);
    let sys_rms = rms(sys);
    if mic_rms == 0.0 || sys_rms == 0.0 {
        return (0, 0.0);
    }

    let n = mic.len().min(sys.len()).min(CORR_WINDOW + max_lag);
    let mut best = (0usize, 0.0f32);
    for lag in 0..=max_lag {
        let len = n.saturating_sub(lag);
        if len == 0 {
            break;
        }
        let c: f32 = mic[lag..lag + len]
            .iter()
            .zip(sys[..len].iter())
            .map(|(m, s)| m * s)
            .sum::<f32>()
            / (len as f32 * mic_rms * sys_rms);
        if c > best.1 {
            best = (lag, c);
        }
    }
    best
}

fn estimate_alpha(mic: &[f32], sys: &[f32], delay: usize) -> f32 {
    let n = mic.len().min(sys.len() + delay);
    let (num, den) = (delay..n).fold((0.0f32, 0.0f32), |(num, den), i| {
        let s = sys[i - delay];
        (num + mic[i] * s, den + s * s)
    });
    if den == 0.0 {
        0.0
    } else {
        (num / den).clamp(0.0, 1.0)
    }
}

fn subtract(mic: &[f32], sys: &[f32], delay: usize, alpha: f32) -> Vec<f32> {
    let mut out = mic.to_vec();
    for i in delay..out.len() {
        if i - delay < sys.len() {
            out[i] = (out[i] - alpha * sys[i - delay]).clamp(-1.0, 1.0);
        }
    }
    out
}

fn rms(s: &[f32]) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}
