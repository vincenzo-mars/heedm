# Devlog — Heedm

Journal condiviso di implementazione. Ogni sessione aggiunge un'entry in cima.

Formato:
```
## YYYY-MM-DD — <titolo>
**Obiettivo:** ...
**Fatto:** ...
**Decisioni:** ...
**Prossimi passi:** ...
```

---

## 2026-06-07 — Cartelle configurabili: modello whisper + registrazioni audio

**Obiettivo:** Permettere all'utente di vedere e cambiare dove l'app mette il modello whisper (+ binary) e i file audio registrati, invece di averli nascosti/scelti a runtime con dialog.

**Fatto:**
- `SttSettings` esteso con `model_dir: Option<String>` e `recordings_dir: Option<String>` (`None` = default)
- Path helper in `commands.rs` ora prendono `&SttSettings` già caricato: `model_dir`, `local_bin_path`, `local_model_path`, nuove `default_recordings_dir`/`recordings_dir`
- Nuovi command: `get_recordings_dir` (path corrente), `pick_directory` (dialog `pick_folder`, riusato per entrambe le cartelle)
- `stop_recording`: rimosso il dialog "salva con nome" — ora scrive direttamente in `recordings_dir/recording-<unix_ts>.wav`, creando la cartella se serve
- `SettingsPanel.svelte`: due righe `.path-row` (modello / registrazioni), ciascuna con path corrente + "Cambia cartella" + "Mostra nel Finder"
- Doc aggiornati: `docs/backend/stt.md`, `docs/backend/commands.md`, `docs/frontend/ui.md`

**Decisioni:**
- Default scelti seguendo convenzioni macOS: `model_dir` resta in `app_data_dir` (dati persistenti non-utente, `~/Library/Application Support/...`), `recordings_dir` di default in `audio_dir()/Heedm/Records` = `~/Music/Heedm/Records/` (contenuto utente, visibile, indicizzato Spotlight; sottocartella `Records` separa le registrazioni da altri eventuali file app dentro `Heedm`)
- Nome file con timestamp leggibile locale invece di unix epoch (`Registrazione 2026-06-07 15.30.12.wav`) — aggiunta dipendenza `chrono` (già presente come transitiva, ora diretta) per `Local::now().format(...)`
- Cambio `model_dir` dopo download non fa migrazione automatica del file (~1.5GB): il pannello chiede conferma esplicita e avvisa che servirà riscaricare nella nuova posizione (`localReady` resettato a `false`)
- `stop_recording` salva sempre in automatico nella cartella configurata invece di chiedere ogni volta — coerente con l'idea di "cartella di default" e più semplice da spiegare

---

## 2026-06-07 — UI: finestra più grande + sezione modello in Settings

**Obiettivo:** Finestra leggermente più ampia; pannello impostazioni che mostra stato/percorso del modello Whisper locale e permette di riscaricarlo.

**Fatto:**
- `tauri.conf.json`: dimensioni finestra `480x360` → `560x420`
- Nessuna configurazione "trascrizione cloud" presente (verificato — già rimossa in setup iniziale, niente da fare)
- Nuovo command `get_local_model_path` (`commands.rs`) — ritorna percorso assoluto di `models/ggml-large-v3-turbo.bin` in `app_data_dir`
- `SettingsPanel.svelte`: mostra sempre stato installazione + percorso del modello su disco; pulsante "Scarica"/"Scarica di nuovo" (label dipende da `localReady`); pulsante "Mostra nel Finder" → `revealItemInDir` (`@tauri-apps/plugin-opener`)
- CSS aggiunto in `App.css`: `.model-path`, `.model-actions`, `.reveal-btn`
- Doc aggiornati: `docs/frontend/ui.md`, `docs/backend/stt.md`

**Decisioni:**
- Riuso di `tauri-plugin-opener` (già dipendenza) per "Mostra nel Finder" invece di aggiungere plugin nuovo

---

## 2026-06-07 — Setup linting/formatting con Biome

**Obiettivo:** Lint e format consistenti su TS/Svelte/JSON, sostituendo strumenti sparsi.

**Fatto:**
- Aggiunto `biome.json`: indent 2 spazi, virgolette doppie, organize imports on save
- `noUnusedImports`/`noUnusedVariables` disabilitati per `*.svelte` — Biome non parsa il markup Svelte, quindi segnala falsi positivi su import/variabili usati solo nel template
- Script `npm run lint` / `lint:fix` in `package.json`
- Primo pass di formattazione su tutto il repo (`App.css` e altri file riformattati)
- Doc aggiornato: `docs/frontend/ui.md`

**Decisioni:**
- Biome al posto di ESLint+Prettier separati — un solo tool, config unica, più veloce

---

## 2026-06-07 — Migrazione frontend React → Svelte

**Obiettivo:** Sostituire React con Svelte 5, ridurre dipendenze e bundle size.

**Fatto:**
- `App.tsx` (monolitico, 4 componenti inline) splittato in `App.svelte` + componenti separati sotto `src/lib/` (`SttIndicator`, `SettingsPanel`, `RecordingItem`, `TranscriptView`)
- Tipi e helper condivisi estratti in `src/lib/types.ts`
- `main.tsx` → `main.ts` (mount via `mount()` di Svelte invece di `ReactDOM.createRoot`)
- State management con runes Svelte 5 (`$state`, `$derived`, `$effect`, `$props`) al posto di `useState`/`useEffect`/`useRef`
- Rimosso `settingsRef` (stato write-only mai letto, già morto in origine — emerso con `noUnusedLocals` su Svelte)
- Build: `vite.config.ts` passa da `@vitejs/plugin-react` a `@sveltejs/vite-plugin-svelte`; aggiunto `svelte.config.js`; `tsconfig.json` estende `@tsconfig/svelte`; typecheck via `svelte-check` al posto di `tsc`
- Dipendenze rimosse: `react`, `react-dom`, `@vitejs/plugin-react`, `@types/react`, `@types/react-dom`
- Dipendenze aggiunte: `svelte`, `@sveltejs/vite-plugin-svelte` (v6, compatibile con vite 7), `@tsconfig/svelte`, `svelte-check`
- `index.html`: `#root` → `#app`, entry `main.ts`
- Bundle JS: ~42 KB (gzip 16 KB) vs build React precedente

**Decisioni:**
- `@sveltejs/vite-plugin-svelte@7` richiede vite 8 (peer dep) — usata `^6.2.4` che supporta vite `^7.0.0`
- Componenti separati per file (idiomatico Svelte) invece di mantenere monolite
- Callback props (`onClose`, `onSaved`, `onSettingsClick`) invece di `createEventDispatcher`, in linea con le API moderne Svelte 5

**Prossimi passi:**
- Verificare l'app in `tauri dev` (build/typecheck già passano puliti)

---

## 2026-06-07 — Setup iniziale repo

**Obiettivo:** Pulire boilerplate, strutturare repo, aggiungere documentazione.

**Fatto:**
- Rimosso Docker e modalità STT esterna — app usa solo whisper-server locale
- `SttSettings` semplificata: rimossi `mode` e `external_url`
- Eliminati asset default Vite/React inutilizzati (SVG, README boilerplate)
- Icona app impostata con `tauri icon` da sorgente custom
- Titolo app: "Heedm" (maiuscolo in `productName`, `title`, `Info.plist`, `index.html`)
- `CFBundleDisplayName` aggiunto a `Info.plist` per nome corretto nel Dock macOS
- CSS morti rimossi da `App.css` (`.mode-toggle`, `.mode-btn`, `.url-*`)
- Icone Android/iOS/Windows Store rimosse (app macOS-only per ora)
- Struttura documentazione creata (`CLAUDE.md`, `DEVLOG.md`, `docs/`)
- 5 commit organizzati per area logica

**Decisioni:**
- Linux e Windows mantenuti come target futuri (`linux.rs`, `windows.rs`, `icon.ico`)
- STT Docker rimosso definitivamente — complessità non giustificata per uso locale
- Docs-as-code in `docs/` con aggiornamento contestuale obbligatorio (regola in CLAUDE.md)

**Prossimi passi:** da definire
