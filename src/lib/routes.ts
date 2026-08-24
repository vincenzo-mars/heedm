import type { RouteDefinition } from "svelte-spa-router";
import Recorder from "./Recorder.svelte";
import RecordingDetail from "./RecordingDetail.svelte";
import SettingsPanel from "./SettingsPanel.svelte";

// Hash routing: sotto Tauri il documento è servito da un protocollo custom e
// non c'è nessun server che possa riscrivere i path, quindi l'unica forma di
// URL che regge un reload è `#/...`.
// L'id del dettaglio è `entry.name`, cioè il nome della cartella della
// registrazione: già univoco dentro la recordings dir e già filesystem-safe.
// Non c'è una rotta per la lista: vive nella home, sotto il bottone REC.
export const routes: RouteDefinition = {
  "/": Recorder,
  "/detail/:id": RecordingDetail,
  "/settings": SettingsPanel,
  "*": Recorder,
};
