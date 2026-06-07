import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

// ── Types ─────────────────────────────────────────────────────────────────────

interface RecordingStatus {
  is_recording: boolean;
  duration_ms: number;
}

interface SttSettings {
  localReady: boolean;
  configured: boolean;
}

interface DownloadProgress {
  step: "binary" | "model" | "done";
  pct: number;
}

interface TranscriptSegment {
  id: number;
  start: number;
  end: number;
  text: string;
  speaker: string | null;
}

interface TranscriptResult {
  task: string;
  language: string;
  duration: number;
  text: string;
  segments: TranscriptSegment[];
}

interface Recording {
  id: string;
  path: string;
  filename: string;
  status: "transcribing" | "done" | "error";
  transcript?: TranscriptResult;
  error?: string;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

const SPEAKER_COLORS = [
  "#3b82f6", "#10b981", "#f59e0b", "#ef4444",
  "#8b5cf6", "#ec4899", "#06b6d4", "#84cc16",
];

function speakerColor(speaker: string): string {
  const match = speaker.match(/(\d+)$/);
  const idx = match ? parseInt(match[1], 10) : 0;
  return SPEAKER_COLORS[idx % SPEAKER_COLORS.length];
}

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  return [h, m % 60, s % 60].map((v) => String(v).padStart(2, "0")).join(":");
}

function formatSeconds(s: number): string {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}

function groupSegments(segments: TranscriptSegment[]) {
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

// ── Settings panel ────────────────────────────────────────────────────────────

function SettingsPanel({
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: (s: SttSettings) => void;
}) {
  const [settings, setSettings] = useState<SttSettings | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [dlProgress, setDlProgress] = useState<DownloadProgress | null>(null);
  const [localReady, setLocalReady] = useState(false);

  useEffect(() => {
    invoke<SttSettings>("get_stt_settings").then((s) => {
      setSettings(s);
      setLocalReady(s.localReady);
    });

    const unlisten = listen<DownloadProgress>("download-progress", (e) => {
      setDlProgress(e.payload);
      if (e.payload.step === "done") {
        setDownloading(false);
        setLocalReady(true);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function save() {
    if (!settings) return;
    const updated: SttSettings = {
      ...settings,
      localReady,
      configured: true,
    };
    await invoke("save_stt_settings", { settings: updated });
    onSaved(updated);
    onClose();
  }

  async function startDownload() {
    setDownloading(true);
    setDlProgress(null);
    try {
      await invoke("download_local_model");
    } catch (e) {
      setDownloading(false);
      alert(String(e));
    }
  }

  if (!settings) return null;

  const dlLabel =
    dlProgress?.step === "binary"
      ? `Binary ${dlProgress.pct}%`
      : dlProgress?.step === "model"
      ? `Modello ${dlProgress.pct}%`
      : dlProgress?.step === "done"
      ? "Completato"
      : null;

  return (
    <div className="settings-backdrop" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <span className="settings-title">Trascrizione</span>
          <button className="settings-close" onClick={onClose}>✕</button>
        </div>

        <div className="mode-content">
          {localReady ? (
            <p className="local-ready">Modello installato e pronto.</p>
          ) : (
            <>
              <p className="local-warning">
                Scarica whisper-server + modello large-v3-turbo (~1.5 GB).
                Necessario solo al primo avvio.
              </p>
              {downloading && dlProgress && (
                <div className="dl-progress">
                  <div className="dl-bar">
                    <div
                      className="dl-fill"
                      style={{ width: `${dlProgress.pct}%` }}
                    />
                  </div>
                  <span className="dl-label">{dlLabel}</span>
                </div>
              )}
              <button
                className="download-btn"
                onClick={startDownload}
                disabled={downloading}
              >
                {downloading ? "Download in corso..." : "Scarica"}
              </button>
            </>
          )}
        </div>

        <button className="save-btn" onClick={save}>
          Salva
        </button>
      </div>
    </div>
  );
}

// ── Transcript ────────────────────────────────────────────────────────────────

function TranscriptView({ transcript }: { transcript: TranscriptResult }) {
  const groups = groupSegments(transcript.segments);
  return (
    <div className="transcript">
      {groups.map((g, i) => {
        const color = speakerColor(g.speaker);
        return (
          <div key={i} className="speaker-block">
            <div className="speaker-label" style={{ color }}>
              {g.speaker}
              <span className="speaker-time">{formatSeconds(g.start)}</span>
            </div>
            <div className="speaker-bubble" style={{ borderLeftColor: color }}>
              {g.text}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function RecordingItem({ rec }: { rec: Recording }) {
  return (
    <div className="recording-item">
      <div className="recording-header">
        <span className="recording-filename">{rec.filename}</span>
        {rec.status === "transcribing" && (
          <span className="badge badge-transcribing">
            <span className="dots"><span /><span /><span /></span>
            trascrizione
          </span>
        )}
        {rec.status === "done" && <span className="badge badge-done">fatto</span>}
        {rec.status === "error" && <span className="badge badge-error">errore</span>}
      </div>

      {rec.status === "transcribing" && (
        <div className="skeleton-lines">
          <div className="skeleton" style={{ width: "80%" }} />
          <div className="skeleton" style={{ width: "55%" }} />
          <div className="skeleton" style={{ width: "70%" }} />
        </div>
      )}
      {rec.status === "error" && <p className="error">{rec.error}</p>}
      {rec.status === "done" && rec.transcript && (
        <TranscriptView transcript={rec.transcript} />
      )}
    </div>
  );
}

// ── STT indicator ─────────────────────────────────────────────────────────────

type SttStatus = "checking" | "starting" | "running" | "error";

function SttIndicator({
  status,
  onSettingsClick,
}: {
  status: SttStatus;
  onSettingsClick: () => void;
}) {
  const labels: Record<SttStatus, string> = {
    checking: "controllo...",
    starting: "avvio server...",
    running: "server attivo",
    error: "server non disponibile",
  };
  return (
    <div className="stt-row">
      <div className={`stt-indicator stt-${status}`}>
        <span className="stt-dot" />
        {labels[status]}
      </div>
      <button className="settings-btn" onClick={onSettingsClick} title="Impostazioni">
        ⚙
      </button>
    </div>
  );
}

// ── App ───────────────────────────────────────────────────────────────────────

function App() {
  const [isRecording, setIsRecording] = useState(false);
  const [durationMs, setDurationMs] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [sttStatus, setSttStatus] = useState<SttStatus>("checking");
  const [showSettings, setShowSettings] = useState(false);
  const settingsRef = useRef<SttSettings | null>(null);

  useEffect(() => {
    invoke<SttSettings>("get_stt_settings").then((s) => {
      settingsRef.current = s;
      if (!s.configured) setShowSettings(true);
      ensureServer();
    });
  }, []);

  async function ensureServer() {
    try {
      const status = await invoke<string>("check_stt_server");
      if (status === "running") { setSttStatus("running"); return; }
      setSttStatus("starting");
      await invoke("start_stt_server");
      setSttStatus("running");
    } catch {
      setSttStatus("error");
    }
  }

  useEffect(() => {
    if (!isRecording) return;
    const id = setInterval(async () => {
      try {
        const status = await invoke<RecordingStatus>("get_recording_status");
        setDurationMs(status.duration_ms);
      } catch {}
    }, 500);
    return () => clearInterval(id);
  }, [isRecording]);

  async function handleRecord() {
    setError(null);
    if (!isRecording) {
      try {
        await invoke("start_recording");
        setIsRecording(true);
        setDurationMs(0);
      } catch (e) {
        setError(String(e));
      }
    } else {
      try {
        const path = await invoke<string>("stop_recording");
        setIsRecording(false);
        if (!path) return;

        const filename = path.split("/").pop() ?? path;
        const id = crypto.randomUUID();

        setRecordings((prev) => [
          { id, path, filename, status: "transcribing" },
          ...prev,
        ]);

        try {
          const transcript = await invoke<TranscriptResult>("transcribe_recording", { path });
          setRecordings((prev) =>
            prev.map((r) => r.id === id ? { ...r, status: "done", transcript } : r)
          );
        } catch (e) {
          setRecordings((prev) =>
            prev.map((r) => r.id === id ? { ...r, status: "error", error: String(e) } : r)
          );
        }
      } catch (e) {
        setIsRecording(false);
        setError(String(e));
      }
    }
  }

  function handleSettingsSaved(s: SttSettings) {
    settingsRef.current = s;
    setSttStatus("checking");
    ensureServer();
  }

  return (
    <div className="app">
      <main className="recorder-section">
        <h1>heedm</h1>
        <SttIndicator status={sttStatus} onSettingsClick={() => setShowSettings(true)} />

        <button
          className={`rec-btn${isRecording ? " recording" : ""}`}
          onClick={handleRecord}
          aria-label={isRecording ? "Stop recording" : "Start recording"}
        >
          {isRecording ? "■ STOP" : "⬤ REC"}
        </button>

        {isRecording && <p className="timer">{formatDuration(durationMs)}</p>}
        {error && <p className="error">{error}</p>}
      </main>

      {recordings.length > 0 && (
        <section className="recordings-list">
          <h2 className="recordings-title">Registrazioni</h2>
          {recordings.map((rec) => (
            <RecordingItem key={rec.id} rec={rec} />
          ))}
        </section>
      )}

      {showSettings && (
        <SettingsPanel
          onClose={() => setShowSettings(false)}
          onSaved={handleSettingsSaved}
        />
      )}
    </div>
  );
}

export default App;
