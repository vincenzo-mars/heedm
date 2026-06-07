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
