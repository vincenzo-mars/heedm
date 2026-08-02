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
  error: string | null;
}

export type SttStatus =
  | "checking"
  | "starting"
  | "running"
  | "error"
  | "stopped";

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

// Stato derivato di una entry, non persistito come tale: il backend scrive solo
// `transcript`/`error` su disco (vedi `transcribe_recording`), qui si ricava la
// terza etichetta per il badge. Da non confondere con `RecordingStatus` sopra,
// che è lo stato della registrazione in corso (`get_recording_status`).
export type TranscriptStatus = "transcribed" | "pending" | "failed";

export function transcriptStatus(entry: RecordingEntry): TranscriptStatus {
  if (entry.transcript) return "transcribed";
  if (entry.error) return "failed";
  return "pending";
}

const TRANSCRIPT_STATUS_INFO: Record<
  TranscriptStatus,
  { label: string; className: string }
> = {
  transcribed: {
    label: "trascritto",
    className: "border border-green-800/60 bg-green-950/40 text-green-400",
  },
  pending: {
    label: "in attesa",
    className:
      "border border-brand-cream/20 bg-transparent text-brand-cream/40",
  },
  failed: {
    label: "fallito",
    className: "border border-red-800/60 bg-red-950/40 text-red-400",
  },
};

export function transcriptStatusInfo(entry: RecordingEntry): {
  label: string;
  className: string;
} {
  return TRANSCRIPT_STATUS_INFO[transcriptStatus(entry)];
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
