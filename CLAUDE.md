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
  commands.rs                    # Tauri commands esposti al frontend
  recorder/
    mod.rs                       # structs condivise (RecorderState, SysAudioStop)
    mic.rs                       # cattura microfono via cpal
    mixer.rs                     # mix mic + system audio
    writer.rs                    # scrittura WAV con hound
    system_audio/
      mod.rs                     # routing platform-specific
      macos.rs                   # SCStream (ScreenCaptureKit)
      linux.rs                   # PulseAudio/PipeWire monitor source
      windows.rs                 # stub (non implementato)

src/
  main.ts                        # entry point Svelte
  App.svelte                     # componente root
  App.css                        # stili
  lib/
    types.ts                     # tipi e helper condivisi
    SttIndicator.svelte          # indicatore stato server STT
    SettingsPanel.svelte         # modal impostazioni/download modello
    RecordingItem.svelte         # card singola registrazione
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

## Convenzioni

- Lingua: italiano per testo UI e messaggi utente; inglese per codice Rust/TS, nomi variabili, commenti tecnici
- Nessun `unwrap()` senza gestione errore nel path principale
- Errori: propagati come `Result<_, String>` verso il frontend

## Regola documentazione (OBBLIGATORIA)

Ogni volta che modifichi o aggiungi codice, aggiorna il doc corrispondente:

| Modifica a... | Aggiorna... |
|---|---|
| `recorder/` (qualsiasi file) | `docs/backend/recorder.md` |
| `commands.rs` — sezione STT | `docs/backend/stt.md` |
| `commands.rs` — sezione recording | `docs/backend/commands.md` |
| `src/` (Svelte) | `docs/frontend/ui.md` |
| flusso dati o dipendenze | `docs/architecture.md` |
| decisione non ovvia o cambio architettura | aggiungi entry in `DEVLOG.md` |

Non aspettare che l'utente lo chieda. Aggiorna sempre contestualmente alla modifica del codice.
