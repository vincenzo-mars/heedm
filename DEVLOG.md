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
