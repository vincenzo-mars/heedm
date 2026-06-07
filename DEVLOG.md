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

## 2026-06-07 — Fix pipeline trascrizione: endpoint sbagliato + formato WAV incompatibile ("error decoding response body")

**Obiettivo:** Risolvere l'errore "error decoding response body" alla trascrizione di una registrazione reale (visibile solo ora che il bundling ha sbloccato l'avvio del server — vedi entry sotto).

**Fatto:**
- Root cause #1 — **endpoint sbagliato**: `transcribe_recording` POSTava a `/v1/audio/transcriptions` (path OpenAI-compatible). Verificato sul sorgente `examples/server/server.cpp` di whisper.cpp (sia tag `v1.7.4` che `master`): il server espone **solo** `/inference`, `/load`, `/health` — mai un endpoint OpenAI-compatible. Il 404 risultante veniva interpretato come JSON → fallimento di parsing → "error decoding response body". Fix: URL → `/inference`, rimosso anche il campo form `model: "whisper-1"` (non parte dell'API whisper.cpp)
- Root cause #2 — **formato WAV incompatibile**: le registrazioni sono salvate mono, rate nativo del mic (cpal-determined, dinamico — es. 48kHz), float32 (`recorder/writer.rs`). `read_wav` di whisper.cpp (`common.cpp`) richiede **esclusivamente** mono 16kHz 16-bit PCM, altrimenti `{"error":"failed to read WAV file"}`
- Provata la direzione del fix end-to-end: convertita manualmente una registrazione a 16kHz/16-bit/mono con `afconvert` e inviata a `/inference` → risposta 200 con trascrizione corretta (`"ciao ragazzi ciao a tutti tutto ok"`), struttura JSON compatibile con `TranscriptResult`/`TranscriptSegment` (incl. `speaker: Option<String>`, deserializzato correttamente da chiave assente — comportamento nativo serde, nessun `#[serde(default)]` necessario)
- Aggiunta `prepare_for_whisper` (`commands.rs`): converte il WAV **in memoria, solo per la richiesta** — legge con `hound::WavReader`, normalizza a `f32`, downmix a mono (media canali), resample a 16kHz via interpolazione lineare (rate arbitrario in ingresso — 44100/48000/altro, non un rapporto intero fisso), converte a `i16`, ri-codifica con `hound::WavWriter` in un `Cursor<Vec<u8>>`. Eseguita in `tokio::task::spawn_blocking` (CPU-bound)
- Doc aggiornati: `docs/backend/stt.md` (sezione "Trascrizione" + nuova sottosezione "Conversione audio in memoria"), `docs/backend/commands.md`

**Decisioni:**
- **Conversione in-memory al momento dell'invio, file salvato invariato**: il file WAV su disco resta a piena qualità (rate nativo, float32) — l'archivio dell'utente non perde qualità per un vincolo del motore STT. Solo i bytes spediti a whisper-server vengono convertiti, scartati subito dopo
- **Interpolazione lineare pura-Rust invece di crate di resampling (`rubato`) o `ffmpeg`**: `--convert` di whisper.cpp richiederebbe `ffmpeg` sulla macchina dell'utente — stesso problema di dipendenza esterna già risolto bundlando il binario, da evitare. Un crate dedicato aggiungerebbe peso/complessità per un guadagno di qualità irrilevante su parlato/ASR (non audio musicale ad alta fedeltà). L'interpolazione lineare è ~15 righe, zero dipendenze, e gestisce correttamente rate arbitrari (incl. rapporti non interi come 44100→16000)

---

## 2026-06-07 — Bundling di whisper-server nell'app (fix "Could not find EOCD" al download)

**Obiettivo:** Risolvere l'errore `invalid Zip archive: Could not find EOCD` segnalato durante il download del modello.

**Fatto:**
- Root cause: `BIN_URL` (`commands.rs`) puntava a un asset GitHub Releases di whisper.cpp **mai esistito** — placeholder mai validato (c'era persino un commento "Update these URLs..."). Verificato via API GitHub: la release `v1.7.4` ha `assets: []`, e nessuna delle ultime 100 release contiene un asset `osx`/`macos`/`darwin`/`server` — whisper.cpp non ha mai distribuito un binario `server` precompilato per macOS. L'app scaricava la pagina 404 e la interpretava come zip → errore EOCD
- Rimossi `BIN_URL`, lo step di download/estrazione zip e la dipendenza `zip` (ormai inutilizzata) da `download_local_model`/`Cargo.toml`
- `whisper-server` ora **bundled come risorsa Tauri**: `tauri.conf.json` → `bundle.resources` (`binaries/whisper-server` → `whisper-server`), risolto a runtime con `app.path().resolve(_, BaseDirectory::Resource)` (nuova `bundled_bin_path` in `commands.rs`, sostituisce `local_bin_path`)
- Compilato `whisper-server` da whisper.cpp v1.7.4: statico (`BUILD_SHARED_LIBS=OFF`, zero dylib esterne — verificato `otool -L`), Metal abilitato (`GGML_METAL=ON`), **binario universale arm64+x86_64** via `lipo -create` (coerente con `minimumSystemVersion: 13.0`)
- Aggiunto `scripts/build-whisper-server.sh` che automatizza l'intero processo (clone, build per le due architetture, lipo merge, posizionamento in `src-tauri/binaries/`, gitignored — binario grande e generato)
- `download_local_model` ora scarica solo il modello (un unico step `"model"` invece di `"binary"`+`"model"`); `SettingsPanel` aggiornato di conseguenza (label progress, testo descrittivo)
- Doc aggiornati: `docs/backend/stt.md` (nuova sezione "Binario whisper-server — bundled come risorsa app"), `docs/backend/commands.md`, `docs/frontend/ui.md`

**Decisioni:**
- **Bundlare il binario precompilato** invece di scaricarlo (ovviamente impossibile, non esiste) o richiederlo come dipendenza esterna. Vincolo guida: l'app deve essere usabile da utenti finali consumer, non da sviluppatori. Due alternative scartate:
  - *Homebrew come dipendenza*: l'utente dovrebbe installare Homebrew + `brew install` whisper.cpp prima di usare l'app. Scartata perché gli utenti consumer non hanno Homebrew — attrito enorme per un'app desktop "double-click and use", e servirebbe logica fragile di detection/guida all'installazione (path, versioni, edge case)
  - *Build da sorgente al primo avvio*: l'app scarica i sorgenti whisper.cpp e li compila in locale al primo lancio. Scartata perché richiede Xcode Command Line Tools + cmake sulla macchina dell'utente (quasi mai presenti su un Mac consumer), compilazione lenta (minuti, CPU/batteria) al primo avvio, e fragile (drift versioni toolchain, necessità di rete per i sorgenti, errori di build da gestire in UI)
  - Bundling: zero passi extra per l'utente, pattern standard per tool "local-AI" desktop, nessuna dipendenza runtime da toolchain esterni — unica opzione compatibile col vincolo "usabile da utenti finali"
- Binario **statico** (no dylib condivise): più semplice da bundlare/firmare/notarizzare (un solo file eseguibile, nessun problema di rpath dentro `.app`), a fronte di un binario leggermente più grande
- **Universal binary** (arm64+x86_64) invece di due risorse separate per architettura: un solo percorso da risolvere a runtime, niente `#[cfg(target_arch)]` lato Rust — più semplice da mantenere, e coerente con quanto il vecchio `BIN_URL` già distingueva per architettura
- Aggiornamenti futuri: whisper.cpp → ri-lanciare lo script con un tag più recente e ricompilare l'app (singolo comando documentato); modello → resta disaccoppiato, si scarica a runtime da HuggingFace, basta cambiare la costante URL/filename senza toccare il binario

---

## 2026-06-07 — "Mostra nel Finder" sempre visibile per la cartella modello

**Obiettivo:** Il bottone "Mostra nel Finder" della cartella modello compariva solo a download completato (`localReady`), inconsistente con la cartella registrazioni (sempre visibile).

**Fatto:**
- `get_local_model_path` (`commands.rs`): crea sempre la cartella padre (`models/`) se assente, prima di restituire il path
- `revealModel` (`SettingsPanel.svelte`): rivela la cartella padre di `modelPath` invece del file `.bin` — il file potrebbe non esistere ancora se il modello non è stato scaricato
- Bottone visibile appena `modelPath` è noto, non solo quando `localReady`

**Decisioni:**
- Stesso pattern già usato per `recordings_dir` (creazione cartella eager) — `revealItemInDir` richiede che il target esista su disco, rivelare la cartella invece del file evita di ripetere il bug "Mostra nel Finder non funziona" già risolto in precedenza

---

## 2026-06-07 — Adozione Tailwind CSS + design system colori (ri-skin completo)

**Obiettivo:** Sostituire il CSS plain centralizzato (`App.css`, 617 righe, classi globali) con utility Tailwind inline e introdurre un design system colori basato su una palette brand fornita dall'utente.

**Fatto:**
- Aggiunte dipendenze `tailwindcss@4` + `@tailwindcss/vite@4` (config CSS-first, niente `tailwind.config.js`)
- `vite.config.ts`: registrato plugin `tailwindcss()`
- `App.css` ridotto da 617 righe a ~70: `@import "tailwindcss"`, token `@theme` (namespace `brand-*`), reset base in `@layer base`, 3 `@keyframes` custom (`pulse-rec`, `blink`, `shimmer`)
- Tutti e 5 i componenti Svelte (`App`, `SttIndicator`, `RecordingItem`, `TranscriptView`, `SettingsPanel`) ri-skinnati con classi utility inline — rimosse tutte le classi globali `.foo {}`
- `biome.json`: abilitato `css.parser.tailwindDirectives` (altrimenti Biome non riconosce `@theme`/`@import "tailwindcss"` e fallisce su parse/format)
- Doc aggiornato: `docs/frontend/ui.md` (sezione styling + tabella token + animazioni custom)

**Decisioni:**
- Token brand namespacizzati `brand-*` (`brand-dark #290808`, `brand-darker #120404`, `brand-light #c4807f`, `brand-lighter #ab2b29`, `brand-lightest #d23434`, `brand-cream #fdf6f6`, `brand-ink #020000`) per evitare collisioni con i token base di Tailwind (`white`/`black`)
- Palette intrinsecamente scura (sfondo `#290808`, testo chiaro `#fdf6f6`) → diventa l'unico tema dell'app; rimosse tutte e 3 le media query `prefers-color-scheme: dark` esistenti (nessun bisogno di doppio tema)
- Stati semantici (successo/errore/warning/STT status) **non** seguono la palette brand — restano i default Tailwind (`green-*`/`red-*`/`amber-*`/`gray-*`) per non perdere il significato convenzionale dei colori
- `pulse` rinominato `pulse-rec` nei `@keyframes` custom per non collidere con l'utility `animate-pulse` nativa di Tailwind
- `SPEAKER_COLORS`/`speakerColor()` (`src/lib/types.ts`) lasciati invariati — palette separata per distinguere speaker, non parte del design system brand

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
