// ── Types ─────────────────────────────────────────────────────────────────────

export interface RecordingStatus {
  is_recording: boolean;
  duration_ms: number;
}

export interface SttSettings {
  localReady: boolean;
  configured: boolean;
  modelDir: string | null;
  recordingsDir: string | null;
}

export interface DownloadProgress {
  step: "model" | "done";
  pct: number;
}

export interface TranscriptSegment {
  id: number;
  start: number;
  end: number;
  text: string;
  speaker: string | null;
}

export interface TranscriptResult {
  task: string;
  language: string;
  duration: number;
  text: string;
  segments: TranscriptSegment[];
  transcription_ms: number | null;
}

export interface RecordingEntry {
  folder_path: string;
  name: string;
  transcript: TranscriptResult | null;
}

export type SttStatus = "checking" | "starting" | "running" | "error";

// ── Helpers ───────────────────────────────────────────────────────────────────

// Diarizzazione "a 2 vie" fatta da whisper sul WAV stereo prodotto da
// stop_recording: canale sinistro = microfono, destro = audio di sistema.
// I segmenti ambigui arrivano già normalizzati a null dal backend.
const SPEAKER_INFO: Record<string, { label: string; color: string }> = {
  "0": { label: "YOU", color: "#3b82f6" },
  "1": { label: "THEM", color: "#10b981" },
};

const UNKNOWN_SPEAKER = { label: "Sconosciuto", color: "#6b7280" };

export function speakerInfo(speaker: string): { label: string; color: string } {
  return SPEAKER_INFO[speaker] ?? UNKNOWN_SPEAKER;
}

export function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  return [h, m % 60, s % 60].map((v) => String(v).padStart(2, "0")).join(":");
}

export function formatSeconds(s: number): string {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}

// Tempo di trascrizione: sotto il minuto mostra i secondi con un decimale
// ("12.4s"), oltre passa a "m:ss" per restare leggibile.
export function formatElapsed(ms: number): string {
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return formatSeconds(ms / 1000);
}

export function groupSegments(segments: TranscriptSegment[]) {
  const groups: { speaker: string; text: string; start: number }[] = [];
  for (const seg of segments) {
    const speaker = seg.speaker ?? "unknown";
    const last = groups[groups.length - 1];
    if (last && last.speaker === speaker) {
      last.text += ` ${seg.text.trim()}`;
    } else {
      groups.push({ speaker, text: seg.text.trim(), start: seg.start });
    }
  }
  return groups;
}
