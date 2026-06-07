# Frontend UI

File: `src/App.tsx`, `src/App.css`

Tutta la UI è in un singolo file `App.tsx`. I componenti sono function components React senza librerie di state management esterne.

## Componenti

### `App` (root)

State:
| Stato | Tipo | Descrizione |
|---|---|---|
| `isRecording` | `boolean` | Flag registrazione attiva |
| `durationMs` | `number` | Durata corrente in ms (polling ogni 500ms) |
| `error` | `string \| null` | Errore da mostrare |
| `recordings` | `Recording[]` | Lista registrazioni della sessione |
| `sttStatus` | `SttStatus` | Stato server STT |
| `showSettings` | `boolean` | Visibilità modal impostazioni |
| `settingsRef` | `Ref<SttSettings>` | Cache settings (no re-render) |

Flusso principale:
1. Mount → `get_stt_settings` → se non configurato apre SettingsPanel → `ensureServer()`
2. REC click → `start_recording` → polling `get_recording_status` ogni 500ms
3. STOP click → `stop_recording` → riceve path WAV → aggiunge Recording con status `"transcribing"` → `transcribe_recording` → status `"done"` o `"error"`

### `SttIndicator`

Indicatore stato server STT + pulsante ⚙ impostazioni.

| Stato | Colore dot | Label |
|---|---|---|
| `checking` | grigio (blink) | "controllo..." |
| `starting` | giallo (blink) | "avvio server..." |
| `running` | verde | "server attivo" |
| `error` | rosso | "server non disponibile" |

### `SettingsPanel`

Modal per setup e download modello Whisper.

Logica:
- Mount → `get_stt_settings` + `listen("download-progress")`
- Se `localReady`: mostra messaggio "Modello installato e pronto"
- Altrimenti: mostra warning dimensioni + pulsante "Scarica"
- Durante download: progress bar 2 step (binary → model), label percentuale
- Save → `save_stt_settings` → `onSaved(settings)` → chiude modal → `ensureServer()`

### `RecordingItem`

Card per una singola registrazione.

| Status | UI |
|---|---|
| `transcribing` | Badge animato + skeleton lines |
| `done` | Badge verde + `TranscriptView` |
| `error` | Badge rosso + messaggio errore |

### `TranscriptView`

Rendering trascrizione con speaker diarization.

- `groupSegments()` aggrega segmenti consecutivi dello stesso speaker
- Ogni gruppo: label speaker colorata (colore da `SPEAKER_COLORS[idx % 8]`) + timestamp + bubble testo
- Colore speaker: estratto da numero finale del nome (`SPEAKER_00` → idx 0)

## Tipi chiave

```typescript
interface SttSettings {
  localReady: boolean;
  configured: boolean;
}

interface Recording {
  id: string;
  path: string;
  filename: string;
  status: "transcribing" | "done" | "error";
  transcript?: TranscriptResult;
  error?: string;
}

type SttStatus = "checking" | "starting" | "running" | "error";
```

## Helpers

| Funzione | Descrizione |
|---|---|
| `formatDuration(ms)` | `ms → "HH:MM:SS"` |
| `formatSeconds(s)` | `s → "M:SS"` (per timestamp segmenti) |
| `speakerColor(speaker)` | Mappa speaker → colore hex da palette 8 colori |
| `groupSegments(segments)` | Aggrega segmenti consecutivi stesso speaker |
