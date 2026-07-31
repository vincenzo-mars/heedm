# Reference

Superficie API dell'app: comandi Tauri, tipi condivisi e componenti Svelte.
Per come funzionano dentro, vedi [`architecture.md`](architecture.md).

## Comandi Tauri

Registrati in `lib.rs` con il percorso completo del modulo: la macro `generate_handler!` genera anche un `__cmd__<nome>` e non segue i re-export, quindi `pub use` nel `mod.rs` non basterebbe.

### Impostazioni e percorsi (`commands/mod.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `get_stt_settings` | — | `SttSettings` | Legge `settings.json`, ritorna il default se assente |
| `save_stt_settings` | `settings: SttSettings` | `Result<(), String>` | Scrive `settings.json` |
| `get_local_model_path` | — | `String` | Path assoluto del modello; crea la cartella padre così "Mostra nel Finder" funziona anche prima del download |
| `get_recordings_dir` | — | `String` | Path assoluto della cartella registrazioni, creata se assente |

Gli helper di percorso (`model_dir`, `local_model_path`, `recordings_dir`, `bundled_bin_path`) prendono un `&SttSettings` già caricato, per non rileggere `settings.json` a ogni chiamata.

### STT (`commands/stt.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `download_local_model` | — | `Result<(), String>` | Scarica il modello in streaming, emette `download-progress`, aggiorna `local_ready` |
| `start_stt_server` | — | `Result<(), String>` | Spawna `whisper-server` se la porta è libera, attende fino a 60s |
| `check_stt_server` | — | `String` | `"running"` oppure `"stopped"` |
| `transcribe_recording` | `path: String` | `Result<TranscriptResult, String>` | POST a `/inference`, scrive `transcript.json` accanto al WAV |

### Registrazione (`commands/recording.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `start_recording` | — | `Result<(), String>` | Avvia mic e audio di sistema; errore se già in corso |
| `stop_recording` | — | `Result<String, String>` | Ferma, applica AEC, scrive il WAV, ritorna il path |
| `get_recording_status` | — | `Result<RecordingStatus, String>` | Flag e durata corrente |
| `list_recordings` | — | `Result<Vec<RecordingEntry>, String>` | Scansiona la cartella Records, ordina per nome decrescente |
| `import_audio_file` | — | `Result<Option<String>, String>` | Picker, decodifica, salva come registrazione. `None` se annullato |

### Permessi OS (`commands/mod.rs` + `permissions.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `check_screen_recording_permission` | — | `bool` | `CGPreflightScreenCaptureAccess`, sola lettura, nessun prompt |
| `open_permission_settings` | `pane: "microphone" \| "screen-recording"` | `Result<(), String>` | Apre il pannello Privacy & Security via `open x-apple.systempreferences:...` |

`pane` è convertito in un enum chiuso prima di toccare il comando di sistema: un valore non riconosciuto ritorna `Err` e non arriva mai a `open`.

Per il microfono non c'è un check di stato: l'unica API è `AVCaptureDevice.authorizationStatus`, Objective-C, e fare il bridging con `objc2` solo per un pallino non vale la complessità.

## Eventi

| Evento | Payload | Emesso da |
|---|---|---|
| `download-progress` | `{ step: "model" \| "done", pct: number }` | `download_local_model` |

## Tipi condivisi (`src/lib/types.ts`)

`SttSettings`, `RecordingStatus`, `RecordingEntry`, `TranscriptResult`, `TranscriptSegment`, `DownloadProgress`, `SttStatus`.

Helper di presentazione nello stesso file:

| Helper | Ruolo |
|---|---|
| `speakerInfo(speaker)` | Mappa la chiave grezza a `{ label, color }`: `"0"` → YOU blu, `"1"` → THEM verde, tutto il resto → Sconosciuto grigio |
| `groupSegments(segments)` | Accorpa segmenti consecutivi con lo stesso speaker |
| `formatDuration(ms)` | `hh:mm:ss`, per il cronometro di registrazione |
| `formatSeconds(s)` | `m:ss`, per i timestamp dei segmenti |
| `formatElapsed(ms)` | `12.4s` sotto il minuto, poi `m:ss` |

## Componenti Svelte

Svelte 5 con le rune (`$state`, `$derived`, `$effect`, `$props`), un componente per file sotto `src/lib/`, nessuna libreria di state management.

### `App.svelte` (root)

Stato: `isRecording`, `durationMs`, `error`, `isTranscribing`, `transcribeMs`, `sttStatus`, `showSettings`, `view` (`"record" | "list" | "detail"`), `selectedEntry`.

Flusso:
1. Mount → `get_stt_settings`; se non configurato apre le impostazioni, poi `ensureServer()`
2. REC → `start_recording`, poi polling di `get_recording_status` ogni 500ms per il cronometro
3. STOP → `stop_recording` → `transcribe_recording`
4. "o carica un file" (solo a riposo) → `import_audio_file` → `transcribe_recording` → vista lista
5. Bottone lista in alto a destra → `RecordsList` → click su una entry → `RecordingDetail`

### `SttIndicator.svelte`

Due elementi flottanti in basso: il pulsante impostazioni a destra e la pillola di stato a sinistra.

| Stato | Dot | Label |
|---|---|---|
| `checking` | grigio lampeggiante | "Controllo..." |
| `starting` | ambra lampeggiante | "Avvio server..." |
| `running` | verde | "Server attivo" |
| `error` | rosso | "Server non disponibile" |

Il colore semantico vive solo sul dot: il testo resta `brand-cream` per uniformità.

### `SettingsPanel.svelte`

Modal con permessi OS, download del modello e percorsi. Contenitore `max-h-[85vh]` con il corpo scrollabile: header e bottone Salva restano fissi. La sezione permessi viene per prima perché blocca tutto il resto.

### `RecordsList.svelte`, `RecordingDetail.svelte`, `TranscriptView.svelte`

Lista delle registrazioni con badge "trascritto"/"in attesa" e tempo di trascrizione; dettaglio con reveal della cartella; rendering della trascrizione raggruppata per speaker con barra colorata a sinistra.

### `Button.svelte`

L'unico componente riusato: due varianti (`ghost`, `icon`) come stringhe di utility Tailwind composte con `cn()`. Non c'è nessuna libreria di primitivi UI.

## Styling

Tailwind CSS v4 con config CSS-first (niente `tailwind.config.js`): plugin `@tailwindcss/vite` e `@import "tailwindcss"` in `src/App.css`. Utility inline nel markup, nessun `<style>` nei componenti.

Palette in `@theme`, namespace `brand-*` per non collidere con i token base di Tailwind:

| Token | Hex | Uso |
|---|---|---|
| `brand-dark` | `#290808` | sfondo app |
| `brand-darker` | `#120404` | superfici, card, pannelli |
| `brand-light` | `#c4807f` | accenti tenui |
| `brand-lighter` | `#ab2b29` | REC a riposo, bottone download |
| `brand-lightest` | `#d23434` | REC in registrazione, timer, glow |
| `brand-cream` | `#fdf6f6` | testo su fondo scuro |
| `brand-ink` | `#020000` | testo su superfici chiare |

Tema unico dark fisso: niente variante `dark:`, niente token semantici. Le opacità frazionarie (`/10`, `/40`, `/85`) sostituiscono una scala di grigi separata.

Alias `$lib` → `src/lib` dichiarato sia in `vite.config.ts` (`resolve.alias`) sia in `tsconfig.json` (`compilerOptions.paths`, mai `baseUrl`): non essendo SvelteKit non è automatico.

## Lint e format

Biome (`biome.json`): `npm run lint` e `npm run lint:fix`. Le regole `noUnusedImports`/`noUnusedVariables` sono disattivate sui `.svelte` perché il parser non legge il markup e produce falsi positivi su simboli usati solo nel template.
