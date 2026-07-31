# Heedm — istruzioni per Claude

## Progetto

**Heedm** è un'app desktop macOS per registrare audio (microfono + sistema) e trascriverlo localmente con whisper.cpp.

- Stack: Tauri v2 + Svelte 5 + TypeScript + Vite
- Backend: Rust (src-tauri/)
- Frontend: Svelte (src/)
- Target: macOS (arm64 + x64), con supporto futuro Linux e Windows
- STT: solo locale tramite whisper-server (whisper.cpp), porta 8080
- Nessun Docker, nessuna dipendenza cloud

## Architettura — mappa moduli

```
src-tauri/src/
  main.rs                        # entry point Rust (boilerplate Tauri)
  lib.rs                         # setup app, registra tutti i commands
  commands/
    mod.rs                       # settings, percorsi, permessi OS
    stt.rs                       # download modello, ciclo di vita whisper-server, trascrizione
    recording.rs                 # start/stop/status/list registrazione, import file
  permissions.rs                 # permessi OS (Screen Recording via CoreGraphics, pannelli System Settings)
  recorder/
    mod.rs                       # structs condivise (RecorderState, SysAudioStop)
    audio.rs                     # primitive condivise: rms, to_mono, resample, mix, WAV
    mic.rs                       # cattura microfono via cpal
    aec.rs                       # echo cancellation: toglie il system audio rientrato nel mic
    diarization.rs               # timeline speaker (mic/sys/both) da energia dei due canali
    system_audio/
      mod.rs                     # routing platform-specific
      macos.rs                   # SCStream (ScreenCaptureKit)
      linux.rs                   # PulseAudio/PipeWire monitor source

src/
  main.ts                        # entry point Svelte
  App.svelte                     # componente root
  App.css                        # stili
  lib/
    types.ts                     # tipi e helper condivisi
    utils.ts                     # cn() (clsx + tailwind-merge)
    Button.svelte                # bottone dell'app (varianti ghost/icon)
    SttIndicator.svelte          # indicatore stato server STT
    SettingsPanel.svelte         # modal impostazioni/download modello
    RecordsList.svelte           # lista registrazioni
    RecordingDetail.svelte       # dettaglio singola registrazione
    TranscriptView.svelte        # rendering trascrizione con speaker

docs/
  architecture.md                # flusso dati end-to-end
  backend/
    recorder.md                  # pipeline registrazione audio
    stt.md                       # integrazione Whisper
    commands.md                  # reference tutti i Tauri commands
  frontend/
    ui.md                        # componenti Svelte
```

## Comandi

- Dev: `source ~/.cargo/env && npm run tauri dev` — richiede `src-tauri/binaries/whisper-server` (se manca: `bash scripts/build-whisper-server.sh`)
- Lint/format: `npm run lint:fix` (biome)
- Typecheck FE: `npm run typecheck`
- Test Rust: `source ~/.cargo/env && cargo test --manifest-path src-tauri/Cargo.toml` — oggi non linka su questo setup (vedi Gotchas macOS): la verifica reale è `cargo check`
- Avvio app, screenshot, whisper-server, smoke test STT: skill `run-heedm`

## Commit

Estende le regole globali (skill `/commit`), non le ripete. Specifico di heedm:

- Il pre-commit hook esegue `biome check --staged`: lancia `npm run lint:fix` PRIMA di `git commit`, non dopo il fallimento
- Un commit che tocca `src/` o `src-tauri/` deve includere il doc corrispondente (vedi Regola documentazione): l'hook `.claude/hooks/check-docs-updated.sh` lo impone, bypass `SKIP_DOCS=1`

## Verifica (OBBLIGATORIA come i doc)

- Una feature che tocca recording/STT non è finita finché lo smoke test record→transcribe non passa (skill `run-heedm`)
- Modifiche UI: verifica con screenshot reale (skill `run-heedm`), non dichiarare "fatto" senza aver visto il render

## Gotchas macOS

- Alias `$lib` via `paths` nel tsconfig, MAI `baseUrl` (deprecato, rimosso in TS 7)
- Screenshot/cattura da terminale richiedono permesso Screen Recording al terminale; UI scripting via osascript è negato su questa macchina — non tentarlo
- Swift compat libs (setup solo-CLT): `.cargo/config.toml` con `-L /Library/Developer/CommandLineTools/usr/lib/swift/macosx`
- `cargo test` fallisce in fase di link (`__swift_FORCE_LOAD_$_swiftCompatibility56` undefined, via `apple_metal` di screencapturekit): il `-L` sopra basta a `cargo check`/`build` ma non al binario di test. Problema di ambiente, non di codice

## Convenzioni

- Lingua: italiano per testo UI e messaggi utente; inglese per codice Rust/TS, nomi variabili, commenti tecnici
- Commenti: solo quando il WHY non è ovvio da codice e nomi. Mai descrivere cosa fa il codice.
- Nessun `unwrap()` senza gestione errore nel path principale
- Errori: propagati come `Result<_, String>` verso il frontend
- Nessun backwards-compatibility hack

## Regola documentazione (OBBLIGATORIA)

Ogni volta che modifichi o aggiungi codice, aggiorna il doc corrispondente:

| Modifica a... | Aggiorna... |
|---|---|
| `recorder/` (qualsiasi file) | `docs/backend/recorder.md` |
| `permissions.rs` | `docs/backend/commands.md` |
| `commands/stt.rs` | `docs/backend/stt.md` |
| `commands/mod.rs`, `commands/recording.rs` | `docs/backend/commands.md` |
| `src/` (Svelte) | `docs/frontend/ui.md` |
| flusso dati o dipendenze | `docs/architecture.md` |
| decisione non ovvia o cambio architettura | aggiungi entry in `DEVLOG.md` |

Non aspettare che l'utente lo chieda. Aggiorna sempre contestualmente alla modifica del codice.
