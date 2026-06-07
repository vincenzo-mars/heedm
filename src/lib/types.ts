// ── Types ─────────────────────────────────────────────────────────────────────

export interface RecordingStatus {
  is_recording: boolean;
  duration_ms: number;
}

export interface SttSettings {
  localReady: boolean;
  configured: boolean;
}

export interface DownloadProgress {
  step: "binary" | "model" | "done";
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
}

export interface Recording {
  id: string;
  path: string;
  filename: string;
  status: "transcribing" | "done" | "error";
  transcript?: TranscriptResult;
  error?: string;
}

export type SttStatus = "checking" | "starting" | "running" | "error";

// ── Helpers ───────────────────────────────────────────────────────────────────

export const SPEAKER_COLORS = [
  "#3b82f6", "#10b981", "#f59e0b", "#ef4444",
  "#8b5cf6", "#ec4899", "#06b6d4", "#84cc16",
];

export function speakerColor(speaker: string): string {
  const match = speaker.match(/(\d+)$/);
  const idx = match ? parseInt(match[1], 10) : 0;
  return SPEAKER_COLORS[idx % SPEAKER_COLORS.length];
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

export function groupSegments(segments: TranscriptSegment[]) {
  const groups: { speaker: string; text: string; start: number }[] = [];
  for (const seg of segments) {
    const speaker = seg.speaker ?? "Unknown";
    const last = groups[groups.length - 1];
    if (last && last.speaker === speaker) {
      last.text += " " + seg.text.trim();
    } else {
      groups.push({ speaker, text: seg.text.trim(), start: seg.start });
    }
  }
  return groups;
}
