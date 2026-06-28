# Frontend UI

File: `src/App.svelte`, `src/lib/*.svelte`, `src/lib/types.ts`, `src/App.css`

UI in Svelte 5 (runes: `$state`, `$derived`, `$effect`, `$props`), un componente per file sotto `src/lib/`. Tipi e helper condivisi in `src/lib/types.ts`. Nessuna libreria di state management esterna.

## Styling — Tailwind CSS v4 + design system colori

Stili applicati con utility Tailwind inline nei template (nessun componente ha `<style>` proprio, nessuna classe globale `.foo`). `src/App.css` contiene solo: `@import "tailwindcss"`, token `@theme`, reset base (`@layer base`) e `@keyframes` custom.

Setup (config CSS-first, Tailwind v4 — niente `tailwind.config.js`):
- `vite.config.ts`: plugin `@tailwindcss/vite` registrato in `plugins`
- `src/App.css`: `@import "tailwindcss";` in testa

### shadcn-svelte

Componenti UI primitivi (`Button`, ecc.) in `src/lib/components/ui/`, generati via CLI (`npx shadcn-svelte@latest add <componente>`) — non modificare a mano lo stile interno dei componenti `ui/*`, sono codice generato e re-installabile.

- `components.json` — config CLI (alias, base color `neutral`, style `vega`, icon library `lucide`)
- Alias `$lib` → `src/lib` (`vite.config.ts` → `resolve.alias`, `tsconfig.json` → `compilerOptions.paths`); non è SvelteKit quindi l'alias non è automatico
- `src/lib/utils.ts` — helper `cn()` (clsx + tailwind-merge) e tipi utility (`WithElementRef`, ecc.), usati dai componenti `ui/*`
- `src/App.css`: `@import "tw-animate-css"`, `@import "shadcn-svelte/tailwind.css"`, `@import "@fontsource-variable/inter"`, `@custom-variant dark (&:is(.dark *))`

**Token semantici shadcn → palette brand**: Heedm ha un solo tema (dark, fisso, niente toggle) — i token `--background`/`--foreground`/`--primary`/`--card`/ecc. definiti in `:root` (`App.css`) puntano direttamente alle variabili `--color-brand-*` invece dei grigi oklch generati di default. Niente blocco `.dark{}`: le utility `dark:*` dei componenti `ui/*` restano inerti (nessun elemento ha mai classe `.dark`).

| Token shadcn | Mappato a |
|---|---|
| `background` / `foreground` | `brand-dark` / `brand-cream` |
| `card`, `popover`, `secondary`, `muted`, `sidebar` | `brand-darker` |
| `primary`, `ring`, `sidebar-primary` | `brand-lightest` |
| `accent`, `destructive`, `sidebar-accent` | `brand-lighter` |
| `muted-foreground` | `brand-light` |
| `border`, `input`, `sidebar-border` | `brand-cream` con alpha (12%/16%) |

### Design tokens (`@theme` in `App.css`)

Palette brand, namespace `brand-*` per non collidere con i token base di Tailwind (`white`/`black`):

| Token | Hex | Uso |
|---|---|---|
| `brand-dark` | `#290808` | sfondo app |
| `brand-darker` | `#120404` | superfici/card/pannelli, barre progresso |
| `brand-light` | `#c4807f` | accenti tenui, bordi/testo bottoni outline, badge "trascrizione" |
| `brand-lighter` | `#ab2b29` | bottone REC (idle), bottone download, riempimento progress bar |
| `brand-lightest` | `#d23434` | bottone REC (hover/recording), timer, glow pulse |
| `brand-cream` | `#fdf6f6` | testo su sfondo scuro |
| `brand-ink` | `#020000` | testo su superfici chiare (es. bottone "Salva") |

Genera utility `bg-brand-*`, `text-brand-*`, `border-brand-*`, ecc. Usate sempre con opacità frazionaria (`/10`, `/40`, `/85`...) per superfici e testo secondario invece di colori grigi separati.

Stati semantici **non** seguono la palette brand — restano i default Tailwind per chiarezza UX:
- successo (`localReady`, badge "fatto") → `green-400`/`green-500`/`green-950`
- errore (banner errore, badge "errore", stato STT `error`) → `red-400`/`red-800`/`red-950`
- avvio server STT → `amber-500`; stato "checking" → `gray-400`

`SPEAKER_INFO`/`speakerInfo()` in `src/lib/types.ts` restano una mappa colori/etichette separata (3 valori fissi: mic/sys/both, vedi `TranscriptView` sotto), non fa parte del design system brand.

### Animazioni custom (`@keyframes` in `App.css`)

Tailwind non ha keyframes con questi nomi/timing — definiti in plain CSS e applicati via arbitrary value (`animate-[nome_durata_easing_infinite]`):
- `pulse-rec` — glow pulsante del bottone REC durante registrazione (nome custom per non collidere con `animate-pulse` di Tailwind)
- `blink` — dot di stato STT, puntini "trascrizione..."
- `shimmer` — skeleton loading durante trascrizione

## Lint & format

Biome (`biome.json`) — `npm run lint` (check) / `npm run lint:fix` (write). Regole `noUnusedImports`/`noUnusedVariables` disattivate per `*.svelte`: il parser Biome non legge il markup, quindi flagga falsi positivi su import/var usati solo in template.

## Componenti

### `App` (root) — `src/App.svelte`

State (`$state`):
| Stato | Tipo | Descrizione |
|---|---|---|
| `isRecording` | `boolean` | Flag registrazione attiva |
| `durationMs` | `number` | Durata corrente in ms (polling ogni 500ms) |
| `error` | `string \| null` | Errore da mostrare |
| `recordings` | `Recording[]` | Lista registrazioni della sessione |
| `sttStatus` | `SttStatus` | Stato server STT |
| `showSettings` | `boolean` | Visibilità modal impostazioni |

Flusso principale:
1. Mount → `get_stt_settings` → se non configurato apre SettingsPanel → `ensureServer()`
2. REC click → `start_recording` → polling `get_recording_status` ogni 500ms
3. STOP click → `stop_recording` → riceve path WAV → aggiunge Recording con status `"transcribing"` → `transcribe_recording` → status `"done"` o `"error"`

### `SttIndicator` — `src/lib/SttIndicator.svelte`

Due elementi flottanti separati in basso (non più un'unica pillola):
- **Pulsante impostazioni**: rotondo a sé, `right-4 bottom-4`, icona `Settings` (`@lucide/svelte`), `bg-brand-darker/85`, hover invertito (`hover:bg-brand-cream hover:text-brand-darker`)
- **Pillola di stato STT**: `left-4 bottom-4`, dot + label, niente bottone incorporato

| Stato | Colore dot | Colore testo | Label |
|---|---|---|---|
| `checking` | grigio (blink) | `text-brand-cream/50` | "Controllo..." |
| `starting` | giallo (blink) | `text-brand-cream/50` | "Avvio server..." |
| `running` | verde | `text-brand-cream` | "Server attivo" |
| `error` | rosso | `text-brand-cream` | "Server non disponibile" |

Il colore semantico (verde/rosso) vive solo sul dot — il testo resta sempre `brand-cream` per uniformità visiva con la pillola.

### `SettingsPanel` — `src/lib/SettingsPanel.svelte`

Modal "Impostazioni": permessi OS, setup/download modello Whisper, cartelle. (Il binario `whisper-server` non si scarica più — è bundled nell'app, vedi `docs/backend/stt.md`.) Contenitore `max-h-[85vh]`, corpo centrale `overflow-y-auto`/`min-h-0` — header e bottone "Salva" restano fissi, le sezioni scrollano (sempre più sezioni di quante ne entrino in 90vw×640px). Chiusura con icona `X` (`@lucide/svelte`), non più glifo testuale.

**Sezione Permessi** (prima — i permessi bloccano tutto il resto), voci elenco (icona/stato + titolo + descrizione su riga sotto), intera voce cliccabile (no bottoni interni) → `open_permission_settings({ pane })`:
- **Microfono**: icona `Mic`, descrizione, click → pane `"microphone"`. Niente stato live: l'unica API di sistema per lo stato di autorizzazione è `AVCaptureDevice.authorizationStatus`, Objective-C — bridging `objc2` solo per un pallino non vale la complessità (vedi `docs/backend/commands.md` → Permessi OS)
- **Cattura audio sistema**: icona di stato (`MonitorCheck` verde se concesso, `MonitorX` grigio se da concedere — stesso significato del dot di `SttIndicator`, ma come icona invece di pallino), popolata da `check_screen_recording_permission` al mount, click → pane `"screen-recording"`

Logica:
- Mount → `get_stt_settings` + `get_local_model_path` + `get_recordings_dir` + `check_screen_recording_permission` + `listen("download-progress")`
- Se `localReady`: mostra messaggio "Modello installato e pronto"
- Altrimenti: mostra warning dimensioni

Due sezioni "percorso", sola visualizzazione + reveal (niente più cambio cartella dalla UI — `pick_directory`/`changeModelDir`/`changeRecordingsDir` rimossi):
- **Modello locale**: titolo + icona `Folder` (mostrata solo se `modelPath` noto) → `revealModel`; sotto: messaggio stato (`localReady` → "Modello installato e pronto" verde, altrimenti warning dimensioni) → riquadro path `bg-brand-dark/50` con micro-label "Percorso" (`uppercase`/`tracking-wider`/`text-brand-cream/40`) sopra il path monospace → progress bar (durante download) → `[Scarica / Scarica di nuovo]` (bottone pieno `bg-brand-lighter`, label cambia in base a `localReady`, `disabled` durante download)
- **Cartella registrazioni**: stesso pattern titolo + icona `Folder` → `revealRecordings`, riquadro path sotto

Logica:
- "Mostra nel Finder" (icona `Folder`, visibile solo a path noto) → `revealItemInDir` (plugin opener). Per le registrazioni rivela `recordingsDir` (sempre creata da `get_recordings_dir`); per il modello rivela la cartella padre di `modelPath` invece del file — il file `.bin` potrebbe non esistere ancora se il modello non è stato scaricato, mentre `get_local_model_path` garantisce che la cartella esista
- Durante download: progress bar singolo step (modello), label percentuale; errore propagato (`throw`, non più `alert`) — risale a chi chiama `startDownload`
- Save → `save_stt_settings` → `onSaved(settings)` → chiude modal → `ensureServer()`

### `RecordingItem` — `src/lib/RecordingItem.svelte`

Card per una singola registrazione.

| Status | UI |
|---|---|
| `transcribing` | Badge animato + skeleton lines |
| `done` | Badge verde + `TranscriptView` |
| `error` | Badge rosso + messaggio errore |

### `TranscriptView` — `src/lib/TranscriptView.svelte`

Rendering trascrizione con diarizzazione "a 2 vie" (tu = mic vs. audio di sistema —
vedi `docs/backend/stt.md` → Diarizzazione per come viene stimata lato backend).

- `groupSegments()` aggrega segmenti consecutivi con la stessa etichetta `speaker`
  grezza (`"mic"|"sys"|"both"|"unknown"` — chiave di raggruppamento, non
  testo da mostrare)
- `speakerInfo(speaker)` mappa la chiave grezza a `{ label, color }` per la UI:
  `mic` → "YOU" (blu), `sys` → "THEM" (verde), `both` → "Sovrapposti" (ambra),
  sconosciuto → "Sconosciuto" (grigio)
- Ogni gruppo: label colorata + timestamp + bubble testo

## Tipi chiave

Definiti in `src/lib/types.ts`.

```typescript
interface SttSettings {
  localReady: boolean;
  configured: boolean;
  modelDir: string | null;      // null = default (app_data_dir)
  recordingsDir: string | null; // null = default (document_dir/Heedm)
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
| `speakerInfo(speaker)` | Mappa chiave grezza speaker → `{ label, color }` (etichetta italiana + colore) |
| `groupSegments(segments)` | Aggrega segmenti consecutivi con la stessa chiave speaker grezza |
