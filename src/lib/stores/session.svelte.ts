import { invoke } from "@tauri-apps/api/core";
import { servers } from "./servers.svelte";

let isRecording = $state(false);
let isTranscribing = $state(false);
let durationMs = $state(0);
let transcribeMs = $state(0);
let error = $state<string | null>(null);

let durationTimer: ReturnType<typeof setInterval> | undefined;
let transcribeTimer: ReturnType<typeof setInterval> | undefined;

// Il frontend è l'unico a poter avviare registrazione e trascrizione, quindi
// l'istante di inizio lo conosce già: nessun bisogno di chiedere al backend
// via IPC ogni mezzo secondo.
function startTimer(
  tick: (elapsed: number) => void,
  everyMs: number,
): ReturnType<typeof setInterval> {
  const start = Date.now();
  tick(0);
  return setInterval(() => tick(Date.now() - start), everyMs);
}

function stopTimer(id: ReturnType<typeof setInterval> | undefined) {
  if (id) clearInterval(id);
  return undefined;
}

// Gate per registrazione e import: senza server whisper attivo e modello sul
// disco, `transcribe_recording` non può funzionare. Lo stesso controllo è
// applicato anche lato backend (start_recording/import_audio_file), qui serve
// solo a riflettersi in UI.
const canRecord = () => servers.sttStatus === "running" && servers.modelReady;

const recordGateReason = () => {
  if (canRecord()) return null;
  if (!servers.modelReady)
    return "Scarica il modello dalle impostazioni per registrare.";
  return "Avvia il server whisper dalle impostazioni per registrare.";
};

// Lock condiviso da REC, import e rilancio: finché è attivo nessun altro
// flusso di trascrizione può partire. Esposto anche a chi mostra righe
// disabilitate (lista e dettaglio).
const locked = () => isRecording || isTranscribing;

// Primitiva usata sia dal flusso REC/import sia dal rilancio in
// `recordings.svelte.ts`: tiene il lock e il cronometro in un posto solo.
// Gli errori non vengono ingoiati: `transcribe_recording` li persiste su
// disco (transcript_error.json) e il chiamante decide se mostrarli.
async function runTranscription(path: string): Promise<void> {
  isTranscribing = true;
  transcribeTimer = startTimer((ms) => {
    transcribeMs = ms;
  }, 100);
  try {
    await invoke("transcribe_recording", { path });
  } finally {
    isTranscribing = false;
    transcribeTimer = stopTimer(transcribeTimer);
  }
}

async function startRecording(): Promise<void> {
  error = null;
  if (locked()) return;
  if (!canRecord()) {
    error = recordGateReason();
    return;
  }
  try {
    await invoke("start_recording");
    isRecording = true;
    durationTimer = startTimer((ms) => {
      durationMs = ms;
    }, 500);
  } catch (e) {
    error = String(e);
  }
}

async function stopRecording(): Promise<void> {
  if (!isRecording) return;
  try {
    const path = await invoke<string>("stop_recording");
    isRecording = false;
    durationTimer = stopTimer(durationTimer);
    if (!path) return;
    await runTranscription(path);
  } catch (e) {
    isRecording = false;
    durationTimer = stopTimer(durationTimer);
    error = String(e);
  }
}

// Ritorna true se un file è stato davvero importato e trascritto: il
// chiamante lo usa per decidere se navigare alla lista. False copre anche
// l'annullamento del dialog, che non è un errore.
async function importFile(): Promise<boolean> {
  error = null;
  if (locked()) return false;
  if (!canRecord()) {
    error = recordGateReason();
    return false;
  }
  try {
    const path = await invoke<string | null>("import_audio_file");
    if (!path) return false;
    await runTranscription(path);
    return true;
  } catch (e) {
    error = String(e);
    return false;
  }
}

export const session = {
  get isRecording() {
    return isRecording;
  },
  get isTranscribing() {
    return isTranscribing;
  },
  get durationMs() {
    return durationMs;
  },
  get transcribeMs() {
    return transcribeMs;
  },
  get error() {
    return error;
  },
  get locked() {
    return locked();
  },
  get canRecord() {
    return canRecord();
  },
  get recordGateReason() {
    return recordGateReason();
  },
  startRecording,
  stopRecording,
  importFile,
  runTranscription,
};
