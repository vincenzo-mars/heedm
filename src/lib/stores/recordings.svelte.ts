import { invoke } from "@tauri-apps/api/core";
import type { RecordingEntry } from "../types";
import { session } from "./session.svelte";

let entries = $state<RecordingEntry[]>([]);
let loading = $state(false);
let loaded = false;

// Fonte unica per lista e dettaglio: senza, il dettaglio raggiunto per URL
// dovrebbe rifare `list_recordings` per conto suo e le due copie andrebbero
// tenute allineate a mano dopo ogni rilancio.
async function load(force = false): Promise<RecordingEntry[]> {
  if (loaded && !force) return entries;
  loading = true;
  try {
    entries = await invoke<RecordingEntry[]>("list_recordings");
    loaded = true;
    return entries;
  } finally {
    loading = false;
  }
}

function byName(name: string): RecordingEntry | undefined {
  return entries.find((e) => e.name === name);
}

// Rilancio della trascrizione da lista o dettaglio: riusa il lock di
// `session` (blocca REC e ogni altra riga), poi ricarica dal disco come unica
// fonte di verità. L'esito, riuscito o fallito, è già persistito da
// `transcribe_recording` (transcript.json o transcript_error.json), quindi la
// ricarica lo riflette da sola senza propagare l'errore.
async function retryTranscription(
  folderPath: string,
): Promise<RecordingEntry[]> {
  if (session.locked) return entries;
  try {
    await session.runTranscription(`${folderPath}/recording.wav`);
  } catch {
    // Esito già sul disco: la load() qui sotto lo rilegge.
  }
  return load(true);
}

export const recordings = {
  get all() {
    return entries;
  },
  get loading() {
    return loading;
  },
  byName,
  load,
  retryTranscription,
};
