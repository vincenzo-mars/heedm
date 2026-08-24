# Reference

Superficie API dell'app: comandi Tauri, tipi condivisi e componenti Svelte.
Per come funzionano dentro, vedi [`architecture.md`](architecture.md).

## Comandi Tauri

Registrati in `lib.rs` con il percorso completo del modulo: la macro `generate_handler!` genera anche un `__cmd__<nome>` e non segue i re-export, quindi `pub use` nel `mod.rs` non basterebbe.

### Impostazioni e percorsi (`commands/mod.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `get_stt_settings` | — | `SttSettings` | Legge `settings.json` (default se assente), poi ricalcola `local_ready` verificando `local_model_path` sul disco: il flag persistito non è mai la fonte di verità |
| `save_stt_settings` | `settings: SttSettings` | `Result<(), String>` | Scrive `settings.json` |
| `get_local_model_path` | — | `String` | Path assoluto del modello; crea la cartella padre così "Mostra nel Finder" funziona anche prima del download |
| `get_recordings_dir` | — | `String` | Path assoluto della cartella registrazioni, creata se assente |

Gli helper di percorso (`model_dir`, `local_model_path`, `recordings_dir`, `bundled_bin_path`, `llm_model_path`, `llm_models_dir`, `llm_cache_dir`) prendono un `&SttSettings` già caricato, per non rileggere `settings.json` a ogni chiamata. `llm_model_path` ritorna `None` se non è stato scelto un modello; `llm_cache_dir` resta solo per pulire la vecchia cache nativa di llama-server (vedi `clear_llm_cache` sotto).

### STT (`commands/stt.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `download_local_model` | — | `Result<(), String>` | Scarica il modello in streaming su `<model>.bin.part`, emette `download-progress`, fa rename atomico a fine download e aggiorna `local_ready` (informativo, vedi sopra) |
| `start_stt_server` | — | `Result<(), String>` | Spawna `whisper-server` se la porta è libera e attende fino a 60s. Se la porta è già occupata, il lockfile decide: nostro processo col modello atteso → adottato senza rispawn, nostro col modello diverso → terminato e rispawnato, altrimenti si usa quello che risponde (vedi [`architecture.md`](architecture.md)) |
| `stop_stt_server` | — | `Result<(), String>` | Killa il processo tracciato, attende che la porta 8080 si liberi (timeout 5s). Se lo slot è vuoto ma la porta è occupata, termina per pid solo un processo che il lockfile riconosce come nostro; altrimenti errore esplicito |
| `restart_stt_server` | — | `Result<(), String>` | `stop_stt_server` + `start_stt_server`; fallisce con lo stesso errore se lo stop fallisce |
| `delete_local_model` | — | `Result<(), String>` | Ferma il server (il processo tiene il `.bin` aperto), poi cancella il file del modello. Errore se lo stop fallisce |
| `check_stt_server` | — | `String` | `"running"` oppure `"stopped"` |
| `transcribe_recording` | `path: String` | `Result<TranscriptResult, String>` | POST a `/inference`, scrive `transcript.json` accanto al WAV; su errore scrive `transcript_error.json` (sidecar `{ message }`), su successo lo rimuove se presente. Riusata anche per il rilancio: nessuna firma dedicata |

### LLM locale (`commands/llm.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `get_system_memory_gb` | — | `f64` | RAM totale in GB, per il badge "consigliato" nella ricerca modelli |
| `search_hf_models` | `query: String` | `Vec<HfModelSummary>` | Proxy verso l'API di ricerca Hugging Face (`filter=gguf&pipeline_tag=text-generation`) |
| `get_hf_model_files` | `repoId: String` | `HfModelDetail` | Proxy verso il dettaglio repo (`?blobs=true`, unico modo per avere `size` reale); filtra ai soli file `.gguf` |
| `set_llm_model` | `repo: String, file: String, sizeBytes: u64` | `Result<(), String>` | Persiste `llm_hf_repo`/`llm_hf_file`/`llm_size_bytes` in `settings.json`; non avvia/riavvia/scarica da solo |
| `download_llm_model` | — | `Result<(), String>` | Scarica il GGUF scelto direttamente da Hugging Face (stesso pattern di `download_local_model`, `commands/download.rs`), emette `llm-download-progress`. Non usa il downloader nativo di llama-server (`--hf-repo`/`--hf-file`): la sua barra di progresso è gated su `isatty(stdout)` nel sorgente di llama.cpp e non produce output sotto `Stdio::piped()` (vedi `DEVLOG.md`) |
| `check_llm_server` | — | `String` | `"stopped"` (porta chiusa) / `"loading"` (porta aperta, `/health` non ancora 200) / `"running"` (`/health` 200) |
| `start_llm_server` | — | `Result<(), String>` | Errore se il modello non è ancora scaricato (`llm_model_path` non esiste su disco); spawna `llama-server` con `--model <path locale>`; ritorna subito senza attendere `/health` (il caricamento in RAM/VRAM può richiedere parecchi secondi) |
| `stop_llm_server` | — | `Result<(), String>` | Come `stop_stt_server`, ma sul processo/porta LLM (`commands/server.rs`, condiviso) |
| `restart_llm_server` | — | `Result<(), String>` | `stop_llm_server` + `start_llm_server` |
| `clear_llm_cache` | — | `Result<(), String>` | Ferma il server, poi cancella sia `llm-models/` (i modelli scaricati da heedm) sia la vecchia `llm-cache/` (cache nativa di llama-server, chi aveva già scaricato un modello con una versione precedente dell'app) |
| `read_recording_notes` | `folderPath: String` | `RecordingNotes` | File mancante/non parsabile → documento vuoto, mai un errore |
| `write_recording_notes` | `folderPath: String, notes: RecordingNotes` | `Result<(), String>` | Whole-document replace, scrittura atomica (`.part` + rename) |
| `delete_recording_notes` | `folderPath: String` | `Result<(), String>` | `NotFound` trattato come successo |

### Registrazione (`commands/recording.rs`)

| Command | Input | Output | Effetti |
|---|---|---|---|
| `start_recording` | — | `Result<(), String>` | Errore se già in corso, oppure se il modello non è scaricato o whisper-server non è in esecuzione (vedi `ensure_stt_ready` in [`architecture.md`](architecture.md)); altrimenti avvia mic e audio di sistema |
| `stop_recording` | — | `Result<String, String>` | Ferma, applica AEC, scrive il WAV, ritorna il path |
| `list_recordings` | — | `Result<Vec<RecordingEntry>, String>` | Scansiona la cartella Records, ordina per nome decrescente. `error` (messaggio da `transcript_error.json`) è letto solo quando `transcript` è assente |
| `import_audio_file` | — | `Result<Option<String>, String>` | Stessa guardia `ensure_stt_ready` di `start_recording`; poi picker, decodifica, salva come registrazione. `None` se annullato |

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
| `llm-download-progress` | `{ step: "llm" \| "done", pct: number }` | `download_llm_model` |

Evento separato dal precedente invece di un nuovo `step` su `download-progress`: quell'evento ha già due listener indipendenti (`SettingsPanel` e `Onboarding`), e uno `step: "done"` condiviso farebbe scattare per errore, nel listener whisper di `SettingsPanel`, il codice che marca `localReady = true`.

## Tipi condivisi (`src/lib/types.ts`)

`SttSettings`, `RecordingEntry`, `TranscriptResult`, `TranscriptSegment`, `DownloadProgress`, `LlmDownloadProgress`, `ServerStatus`, `SttStatus`, `TranscriptStatus`, `NotesSummary`, `ChatRole`, `ChatMessage`, `RecordingNotes`, `HfModelSummary`, `HfGgufFile`, `HfModelDetail`.

`ServerStatus` = `"checking" | "starting" | "running" | "error" | "stopped" | "loading"`, stato di un server locale gestito dall'app (whisper o LLM). `"stopped"` è distinto da `"error"`: è l'esito di uno stop esplicito (impostazioni), non di un fallimento. `"loading"` è specifico dell'LLM (`llama-server` apre la porta prima di aver caricato il modello; whisper-server no, quindi non lo usa mai). `SttStatus` è un alias di `ServerStatus` per non far churnare `SttIndicator`/`SettingsPanel`.

`LlmDownloadProgress { step: "llm" | "done", pct: number }`: stessa forma di `DownloadProgress`, ma su un evento dedicato (vedi sopra).

`SttSettings.llmHfRepo`/`llmHfFile: string` (`""` = nessun modello scelto) sono il modello LLM selezionato dall'utente in Settings. `llmSizeBytes: number` è la size del file scelto (nota dal frontend al momento della selezione, da `HfGgufFile.size_bytes`), usata solo per etichettare il bottone di download. `llmReady: boolean` rispecchia `localReady`: ricalcolato da `get_stt_settings` verificando `llm_model_path` sul disco, mai la fonte di verità è il valore persistito.

`NotesSummary { text, key_points: string[], actions: string[], open_questions: string[], model, generated_at }`, `ChatMessage { id, role: ChatRole, content, at }` (`ChatRole` = `"user" | "assistant"`), `RecordingNotes { version, summary: NotesSummary | null, messages: ChatMessage[] }`: rispecchiano 1:1 le struct Rust in `commands/llm.rs` (snake_case, come `RecordingEntry`/`TranscriptResult`, non camelCase come `SttSettings`).

`HfModelSummary { id, downloads, license }` (risultato di ricerca), `HfGgufFile { filename, size_bytes }`, `HfModelDetail { gated, files: HfGgufFile[] }` (dettaglio repo).

`RecordingEntry.error: string | null` è il messaggio dell'ultimo fallimento di trascrizione (da `transcript_error.json`), sempre `null` quando `transcript` è presente.

`TranscriptStatus` = `"transcribed" | "pending" | "failed"`, stato derivato (non un campo persistito) di una `RecordingEntry`.

Helper di presentazione nello stesso file:

| Helper | Ruolo |
|---|---|
| `speakerInfo(speaker)` | Mappa la chiave grezza a `{ label, color }`: `"0"` → YOU blu, `"1"` → THEM verde, tutto il resto → Sconosciuto grigio |
| `groupSegments(segments)` | Accorpa segmenti consecutivi con lo stesso speaker |
| `formatDuration(ms)` | `hh:mm:ss`, per il cronometro di registrazione |
| `formatSeconds(s)` | `m:ss`, per i timestamp dei segmenti |
| `formatElapsed(ms)` | `12.4s` sotto il minuto, poi `m:ss` |
| `transcriptStatus(entry)` | `RecordingEntry` → `TranscriptStatus` (`transcript` presente → `"transcribed"`, altrimenti `error` presente → `"failed"`, altrimenti `"pending"`) |
| `transcriptStatusInfo(entry)` | `transcriptStatus(entry)` → `{ label, className }` per il badge a 3 stati di `RecordsList` |

## Componenti Svelte

Svelte 5 con le rune (`$state`, `$derived`, `$effect`, `$props`), un componente per file sotto `src/lib/`, nessuna libreria di state management.

### `App.svelte` (root)

Stato: `isRecording`, `durationMs`, `error`, `isTranscribing`, `transcribeMs`, `sttStatus`, `modelReady`, `llmStatus`, `showSettings`, `view` (`"record" | "list" | "detail"`), `selectedEntry`.

`refreshLlmState(opts?: { attemptStart?: boolean }): Promise<SttSettings | null>` è il mirror di `refreshSttState` per il server LLM: stessa forma, stessa fonte di verità (`get_stt_settings` + `check_llm_server`), ma **default invertito** — `attemptStart` è `false` qui contro il `true` di STT. Al mount si osserva soltanto: caricare un modello LLM da 1-5GB per una feature che molte sessioni non toccano mai non ha senso, a differenza di whisper che serve sempre. Un secondo `$effect` fa polling di `check_llm_server` ogni 2s finché `llmStatus` è `"loading"`/`"starting"`, perché a differenza di whisper il tempo di avvio (download compreso) è imprevedibile e nessun secondo click dell'utente lo farebbe altrimenti aggiornare. Passata a `SettingsPanel` (`onLlmServerRefresh`) e a `RecordingDetail`→`TranscriptNotes` (`onLlmRefresh`).

`canRecord = $derived(sttStatus === "running" && modelReady)` è il gate lato UI per registrazione e import (rispecchia `ensure_stt_ready` lato backend, vedi [`architecture.md`](architecture.md)); `recordGateReason` (`$derived.by`) è il messaggio in italiano da mostrare quando `!canRecord`, che distingue "modello non scaricato" da "server non attivo". Quando `!canRecord`: il bottone REC resta abilitato per lo STOP (non blocca una registrazione già in corso) ma non per l'avvio, il bottone "o carica un file" è disabilitato, entrambi mostrano `recordGateReason` come `title`, e lo stesso testo appare come paragrafo visibile sotto i bottoni. `handleRecord`/`handleImport` ripetono il controllo su `canRecord` anche a runtime (non solo sull'attributo `disabled`), per coerenza con la guardia backend.

`refreshSttState(opts?: { attemptStart?: boolean }): Promise<SttSettings | null>` è il punto unico di riallineamento fra stato reale (whisper-server + modello sul disco) e stato mostrato in UI: legge `get_stt_settings` (aggiorna `modelReady` da `localReady`) e `check_stt_server`, poi imposta `sttStatus` di conseguenza (`"checking"` mentre gira, poi `"running"` / `"stopped"` / `"starting"`→`"running"` / `"error"`). Con `attemptStart: true` (default) un server non in esecuzione viene avviato, come oggi al mount; con `attemptStart: false` un server fermo resta `"stopped"` invece di essere riavviato automaticamente, il caso di chi lo ha appena fermato di proposito dalle impostazioni. Ritorna le `SttSettings` lette (o `null` in errore). È chiamata al mount, da `onSaved` di `SettingsPanel` e dalla `onContinue` di `Onboarding`; chiunque cambi lo stato del server o del modello (impostazioni, rilancio trascrizione) la richiama invece di duplicare la logica di check/avvio. Passata ai figli come callback: a `SettingsPanel` direttamente come `onServerRefresh` (richiamata dopo ogni azione sul server) e via `onSaved` (dopo un salvataggio); a `Onboarding` direttamente come `onContinue`.

Rendering condizionato da `modelReady`, valutato allo stesso modo indipendentemente dal motivo per cui il modello manca (primo avvio mai configurato, o modello eliminato in un secondo momento dalle impostazioni): se `!modelReady` l'intera UI normale è sostituita da `Onboarding` a schermo intero, nessun ramo separato per i due casi.

Flusso:
1. Mount → `refreshSttState()`; se `!modelReady` si mostra `Onboarding` al posto della UI normale
2. `Onboarding` scarica il modello e a fine download invoca `onContinue` → `refreshSttState()` → `modelReady` torna `true` → si torna automaticamente alla UI normale
3. REC → `start_recording`; il cronometro è un timer locale del frontend (stesso pattern di `transcribeMs`), nessun polling IPC
4. STOP → `stop_recording` → `transcribe_recording`
5. "o carica un file" (solo a riposo) → `import_audio_file` → `transcribe_recording` → vista lista
6. Bottone lista in alto a destra → `RecordsList` → click su una entry → `RecordingDetail`
7. Rilancio trascrizione (da `RecordsList` o `RecordingDetail`) → `retryTranscription(folderPath)`

`retryTranscription(folderPath: string): Promise<RecordingEntry[]>` è la callback passata a `RecordsList` e `RecordingDetail` come `onRetry`. Riusa lo stesso `isTranscribing` del flusso REC/import (nessun secondo flag): se `isRecording || isTranscribing` è già vero esce subito senza fare nulla (guardia ridondante rispetto al `disabled` dei bottoni, stesso pattern di `handleImport`/`handleRecord`). Altrimenti mette `isTranscribing = true`, chiama `transcribe_recording` su `<folderPath>/recording.wav` ignorando un eventuale errore (già persistito su disco da `transcribe_recording` come `transcript.json` o `transcript_error.json`, non serve duplicarlo nel banner `error` della vista REC), poi in ogni caso rimette `isTranscribing = false`. A quel punto richiama `list_recordings` (unica fonte di verità, nessun refresh locale scollegato dal backend) e, se `selectedEntry` è impostato, lo riallinea con l'entry corrispondente nell'array appena letto — così `RecordingDetail`, se aperto sul record rilanciato, non resta con una prop stale. Ritorna l'array fresco: `RecordsList` lo usa per aggiornare la propria lista, `RecordingDetail` lo ignora (il suo aggiornamento passa da `selectedEntry`).

### `SttIndicator.svelte`

Due elementi flottanti in basso: il pulsante impostazioni a destra e la pillola di stato a sinistra.

| Stato | Dot | Label |
|---|---|---|
| `checking` | grigio lampeggiante | "Controllo..." |
| `starting` | ambra lampeggiante | "Avvio server..." |
| `running` | verde | "Server attivo" |
| `error` | rosso | "Server non disponibile" |
| `stopped` | grigio fisso | "Server fermo" |
| `loading` | ambra lampeggiante | "Caricamento..." |

Il colore semantico vive solo sul dot: il testo resta `brand-cream` per uniformità. `loading` esiste solo perché `SttStatus` è ora un alias di `ServerStatus`: whisper-server non lo usa mai (apre la porta solo a modello già caricato), questo componente mostra solo lo stato STT, non quello LLM (nessuna pill separata per l'LLM, vedi `architecture.md`).

### `Onboarding.svelte`

View a schermo intero (non un modal), mostrata da `App.svelte` al posto della UI normale quando `!modelReady`. Logo (`public/icon.png`), titolo e sottotitolo fissi, poi tre stati in sequenza: CTA "Scarica il modello" → progress bar sull'evento `download-progress` (stesso comando `download_local_model` e stesso evento di `SettingsPanel`) con il messaggio "prenditi un caffè" visibile solo mentre `downloading` è vero → messaggio finale e bottone "Continua". Nessun bottone "salta": l'unica uscita è un download riuscito.

Prop: `onContinue: () => void`, chiamata dal bottone "Continua"; `App.svelte` la collega a `refreshSttState()` così `modelReady` si aggiorna e il rendering torna da solo alla UI normale, senza che `Onboarding` gestisca la propria visibilità.

### `SettingsPanel.svelte`

Modal con permessi OS, download dei modelli, controllo server STT/LLM e percorsi. Contenitore `w-[min(1400px,92vw)] max-h-[88vh]` con il corpo a griglia scrollabile (`grid-cols-1 lg:grid-cols-2`, un solo scroll esterno): header e bottone Salva restano fissi fuori dalla griglia. Colonna sinistra (core, sempre rilevante): Permessi, Modello Whisper, Server STT, Cartella registrazioni. Colonna destra (opzionale, additiva): Modello LLM, Server LLM — stessa distinzione già in [`architecture.md`](architecture.md) fra STT (sempre necessario) e LLM (riassunto/chat, lazy). Sotto il breakpoint `lg` (1024px) le due colonne si impilano in una sola, identico al layout precedente. La sezione permessi viene per prima perché blocca tutto il resto.

Prop:
- `onClose: () => void` — callback per chiudere il modal
- `onSaved: () => void` — callback invocata dopo salvataggio delle impostazioni e dopo eliminazione del modello (chiude il modal automaticamente)
- `isRecording?: boolean` — disabilita i bottoni del server se in corso una registrazione
- `isTranscribing?: boolean` — disabilita i bottoni del server se in corso una trascrizione
- `sttStatus: SttStatus` — stato del server, passato da `App.svelte` (fonte di verità unica, non duplicato localmente)
- `onServerRefresh: (opts?: { attemptStart?: boolean }) => Promise<SttSettings | null>` — `refreshSttState` di `App.svelte`, passata by reference: ogni azione sul server la richiama per riallineare lo stato condiviso (gate REC, `SttIndicator`) invece di tenere una copia locale che si disallineerebbe alla chiusura del modal con la X
- `llmStatus: ServerStatus` — stato del server LLM, passato da `App.svelte`
- `onLlmServerRefresh: (opts?: { attemptStart?: boolean }) => Promise<SttSettings | null>` — `refreshLlmState` di `App.svelte`, stesso ruolo di `onServerRefresh` ma per l'LLM

Le sezioni server STT e LLM sono due istanze di `ServerControls.svelte` (vedi sotto). Tutti i comandi server passano da due helper interni (`runStt(cmd, opts?)` / `runLlm(cmd)`: busy flag, `invoke`, refresh dello stato in `App.svelte`, errore nella card di stato); lato STT:
- **Avvia** → `start_stt_server`, poi `onServerRefresh()`
- **Riavvia** → `restart_stt_server`, poi `onServerRefresh()`
- **Spegni** → `stop_stt_server`, poi `onServerRefresh({ attemptStart: false })` (NON riavvia automaticamente)
- **Elimina modello** → `delete_local_model`, poi `onServerRefresh()` (aggiorna `modelReady` a `false`, che fa ricomparire `Onboarding` in `App.svelte`) e, se non c'è errore, `onSaved`/`onClose` per chiudere il modal

Sezione "Modello LLM": campo di ricerca con debounce (~400ms, query di default "instruct" a campo vuoto). La prima fetch è lazy: parte al primo focus del campo, non all'apertura del pannello (chi apre le impostazioni per i permessi non costa una chiamata a Hugging Face). Poi → `search_hf_models` → lista repo (nome, licenza dai tag, download) → click espande e chiama `get_hf_model_files` (i file GGUF divisi in shard, `<nome>-NNNNN-of-NNNNN.gguf`, sono filtrati via lato Rust: fuori scope, vedi `DEVLOG.md`) → lista file `.gguf` con dimensione reale e un badge "consigliato/pesante per la tua RAM" (euristica su `get_system_memory_gb`, mai bloccante) → click su un file chiama `set_llm_model` (con la size) e aggiorna `settings` in locale (`llmReady` torna `false` finché il download non completa). Repo `gated` mostrato ma non selezionabile (lucchetto + messaggio, nessun flusso di login HF).

Subito sotto il box "Selezionato" c'è il bottone di download (mirror di quello whisper: stessa progress bar, stesso stile), con 4 stati:

| Condizione | Label |
|---|---|
| nessun modello selezionato | bottone non renderizzato |
| download in corso | "Download in corso..." (disabled) |
| selezionato, non ancora scaricato | "Scarica modello (X GB)" |
| già scaricato (`llmReady`) | "Scarica di nuovo (X GB)" |

Il download (`download_llm_model`) ascolta l'evento dedicato `llm-download-progress`; a `step: "done"` ricarica `settings` da `get_stt_settings` (non setta `llmReady` in ottimismo: la verità è il file su disco) e richiama `onLlmServerRefresh`.

Sezione "Server LLM" sotto: seconda istanza di `ServerControls` con `llmStatus`/i comandi `*_llm_server`; Avvia/Riavvia sono `disabled` anche quando `!settings.llmReady` (prop `canStart`, con il messaggio "Scarica prima il modello." come `hint` nella card di stato), e il bottone danger è "Elimina modelli scaricati" (`clear_llm_cache`, cancella sia i modelli scaricati da heedm sia la vecchia cache nativa di llama-server). `sttLoading`/`sttError` e `llmLoading`/`llmError` sono stati locali separati per sezione: un errore sul server LLM non disabilita i controlli whisper e viceversa. I tre box "Percorso"/"Selezionato" sono un unico snippet Svelte locale (`pathBox`).

### `RecordsList.svelte`, `RecordingDetail.svelte`, `TranscriptView.svelte`

Lista delle registrazioni con badge a 3 stati ("trascritto"/"in attesa"/"fallito", da `transcriptStatusInfo`) e tempo di trascrizione; dettaglio con reveal della cartella; rendering della trascrizione raggruppata per speaker con barra colorata a sinistra.

Prop condivise da `RecordsList` e `RecordingDetail`: `isRecording: boolean`, `isTranscribing: boolean` (lock globale passato da `App.svelte`, combinati in un `locked = $derived(...)` locale) e `onRetry: (folderPath: string) => Promise<RecordingEntry[]>` (`retryTranscription` di `App.svelte`).

In `RecordsList` ogni riga non è più un unico `<button>`: è un `<div>` che contiene un `<button>` (nome + badge, disabilitato quando `locked`, chiama `onSelect`) e un `Button` "Rilancia" separato (nesting di bottoni non è HTML valido). Il bottone "Rilancia" compare su ogni riga, non solo su quelle fallite: il rilancio è pensato anche per un record trascritto "che non convince". Al click chiama `onRetry(entry.folder_path)` e sostituisce `entries` con l'array ritornato.

In `RecordingDetail` un bottone "Rilancia trascrizione" nell'header (stesso stile di "Apri cartella") chiama `onRetry(entry.folder_path)` ignorando il valore di ritorno: l'aggiornamento arriva da `App.svelte` che riassegna `selectedEntry`, prop che si propaga qui da sola. Il corpo mostra `TranscriptView` se `entry.transcript` è presente, altrimenti il messaggio d'errore in rosso se `entry.error` è presente, altrimenti "Trascrizione non ancora disponibile." Sotto la card trascrizione, `<TranscriptNotes {entry} {llmStatus} {onLlmRefresh} />` (prop aggiuntive di `RecordingDetail`, passate da `App.svelte`).

### `TranscriptNotes.svelte`, `TranscriptChat.svelte`, `src/lib/llm.ts`

Riassunto e chat locale sulla trascrizione di una registrazione (vedi `architecture.md` per il flusso completo). `TranscriptNotes` è l'orchestrazione: carica `read_recording_notes` al mount, genera il riassunto in streaming (`streamSummary` da `llm.ts`) automaticamente se `llmStatus === "running"` e non esiste ancora, altrimenti mostra un CTA in base allo stato (nessun modello scelto/server fermo/server in caricamento); espone il bottone "Rigenera" quando un riassunto esiste già. Persiste su disco (`write_recording_notes`) una volta per riassunto completato e una volta per turno di chat completato, mai per token. Un `AbortController` per richiesta è abortito sia da un bottone "Interrompi" sia dal teardown dell'`$effect` (uscire dal dettaglio a metà streaming non deve lasciare occupato lo slot del server locale).

Un errore in chat non persiste il messaggio utente: resta in `pendingQuestion`, passato a `TranscriptChat` come `retryDraft` così l'utente può reinviare senza riscrivere la domanda. `TranscriptChat` è puramente presentazionale: lista messaggi, bolla di streaming, textarea con invio su Enter (Shift+Enter per andare a capo), disabilitata quando `disabled` (LLM non `"running"`) o `busy`.

`llm.ts` è l'unico file che conosce l'AI SDK: `createOpenAICompatible` col `fetch` di `@tauri-apps/plugin-http` (vedi `architecture.md`), `buildTranscriptContext` (righe `[m:ss] IO/INTERLOCUTORE` o testo grezzo, con cap e flag `truncated`), `streamSummary`/`streamChatReply` (wrapper di `streamText`), `parseSummary` (parser tollerante delle quattro sezioni marcate).

### `Button.svelte`

Il bottone dell'app: cinque varianti (`ghost`, `icon`, `solid`, `danger`, `primary`) come stringhe di utility Tailwind composte con `cn()` (le classi passate via `class` vincono sui conflitti grazie a tailwind-merge, es. per padding/size diversi a parità di variante). Non c'è nessuna libreria di primitivi UI.

`solid` e `primary` sono i due bottoni di conferma: entrambi bianchi (`brand-cream`) con testo scuro e hover `brand-cream-dim`, distinti solo dalla taglia (`primary` è il CTA grande dell'onboarding). `danger` resta rosso pieno: il rosso distruttivo è convenzione, non palette di brand.

### `ServerControls.svelte`

Blocco stato + controlli di un server locale, usato due volte da `SettingsPanel` (STT e LLM). Prop: `title`, `status: ServerStatus`, `error: string | null`, `hint?: string | null` (mostrato nella card solo in assenza di errore), `loading`, `locked` (registrazione/trascrizione in corso), `canStart?: boolean` (gate extra su Avvia/Riavvia, default `true`), `dangerLabel` e le callback `onStart`/`onRestart`/`onStop`/`onDanger`. Stato mostrato: "Attivo" (`running`) / "Caricamento..." (`loading`) / "Fermo" (tutto il resto).

### `DownloadProgressBar.svelte`

Barra di avanzamento download modello, usata da `Onboarding` e da `SettingsPanel` (whisper + LLM). Prop: `progress: { step: string; pct: number }` (`step === "done"` → "Completato", altrimenti "Modello N%") e `trackClass?` per il colore della traccia (default `bg-brand-dark`; `Onboarding` passa `bg-brand-darker` perché sta su fondo `brand-dark`). Il riempimento è `brand-cream`.

## Styling

Tailwind CSS v4 con config CSS-first (niente `tailwind.config.js`): plugin `@tailwindcss/vite` e `@import "tailwindcss"` in `src/App.css`. Utility inline nel markup, nessun `<style>` nei componenti.

Palette in `@theme`, namespace `brand-*` per non collidere con i token base di Tailwind:

| Token | Hex | Uso |
|---|---|---|
| `brand-dark` | `#1b1b1d` | sfondo app |
| `brand-darker` | `#121213` | superfici, card, pannelli |
| `brand-light` | `#8e8e93` | grigio chiaro: testo secondario, bolla utente in chat |
| `brand-lighter` | `#3a3a3d` | grigio medio: superficie rialzata, bottone di conferma disabilitato |
| `brand-lightest` | `#4a4a4e` | gradino grigio più chiaro |
| `brand-cream` | `#f5f5f7` | testo su fondo scuro, bottoni di conferma |
| `brand-cream-dim` | `#e5e5e7` | hover dei bottoni di conferma |
| `brand-ink` | `#0a0a0b` | testo su superfici chiare |

Il rosso non è più colore di brand: vive in due token separati, usati solo dalla registrazione (bottone REC, alone pulsante, timer).

| Token | Hex | Uso |
|---|---|---|
| `rec` | `#ab2b29` | REC a riposo |
| `rec-strong` | `#d23434` | REC in registrazione, timer, glow (`@keyframes pulse-rec`) |

Tema unico dark fisso: niente variante `dark:`, niente token semantici. Le opacità frazionarie (`/10`, `/40`, `/85`) sostituiscono una scala di grigi separata.

Alias `$lib` → `src/lib` dichiarato sia in `vite.config.ts` (`resolve.alias`) sia in `tsconfig.json` (`compilerOptions.paths`, mai `baseUrl`): non essendo SvelteKit non è automatico.

## Lint e format

Biome (`biome.json`): `npm run lint` e `npm run lint:fix`. Le regole `noUnusedImports`/`noUnusedVariables` sono disattivate sui `.svelte` perché il parser non legge il markup e produce falsi positivi su simboli usati solo nel template.
