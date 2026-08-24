import { invoke } from "@tauri-apps/api/core";
import type { ServerStatus, SttSettings, SttStatus } from "../types";

let sttStatus = $state<SttStatus>("checking");
let llmStatus = $state<ServerStatus>("checking");
let modelReady = $state(false);
let llmReady = $state(false);

let llmPollId: ReturnType<typeof setInterval> | undefined;

// llama-server, a differenza di whisper-server, può restare "loading" a lungo
// (caricamento di 1-5GB in RAM/VRAM): questo poll è l'unico modo per la UI di
// accorgersi quando diventa "running" da solo, senza un secondo click.
// Passare sempre da qui invece di assegnare `llmStatus` direttamente: è il
// punto che accende e spegne il poll.
function setLlmStatus(next: ServerStatus) {
  llmStatus = next;
  if (next === "loading" || next === "starting") startLlmPoll();
  else stopLlmPoll();
}

function startLlmPoll() {
  if (llmPollId) return;
  llmPollId = setInterval(async () => {
    try {
      const status = await invoke<string>("check_llm_server");
      if (status === "running") setLlmStatus("running");
      else if (status === "stopped") setLlmStatus("stopped");
      else setLlmStatus("loading");
    } catch {
      setLlmStatus("error");
    }
  }, 2000);
}

function stopLlmPoll() {
  if (!llmPollId) return;
  clearInterval(llmPollId);
  llmPollId = undefined;
}

// Punto unico di riallineamento fra stato reale (whisper-server + modello sul
// disco) e stato mostrato in UI. Chiamata al mount e da chi cambia qualcosa
// (salvataggio impostazioni, fine onboarding, stop/restart del server): i
// chiamanti non devono duplicare la logica di check/avvio.
// `attemptStart: false` serve a chi ha appena fermato il server di proposito:
// senza, il check troverebbe "stopped" e lo riavvierebbe subito.
async function refreshStt(
  opts: { attemptStart?: boolean } = {},
): Promise<SttSettings | null> {
  const attemptStart = opts.attemptStart ?? true;
  sttStatus = "checking";
  try {
    const settings = await invoke<SttSettings>("get_stt_settings");
    modelReady = settings.localReady;
    const status = await invoke<string>("check_stt_server");
    if (status === "running") {
      sttStatus = "running";
      return settings;
    }
    if (!attemptStart) {
      sttStatus = "stopped";
      return settings;
    }
    if (!modelReady) {
      // Senza modello whisper-server fallirebbe comunque all'avvio: risparmia
      // il giro e riflette subito lo stato reale.
      sttStatus = "error";
      return settings;
    }
    sttStatus = "starting";
    await invoke("start_stt_server");
    sttStatus = "running";
    return settings;
  } catch {
    sttStatus = "error";
    return null;
  }
}

// Mirror di refreshStt per il server LLM (riassunto/chat), con un default
// invertito: `attemptStart` è `false` qui (STT ce l'ha `true`). Al mount
// osserviamo soltanto: non ha senso caricare 1-5GB di modello per una feature
// che molte sessioni non toccano mai. Diventa `true` solo quando l'utente
// preme esplicitamente "Avvia" (impostazioni o pannello note).
async function refreshLlm(
  opts: { attemptStart?: boolean } = {},
): Promise<SttSettings | null> {
  const attemptStart = opts.attemptStart ?? false;
  setLlmStatus("checking");
  try {
    const settings = await invoke<SttSettings>("get_stt_settings");
    llmReady = settings.llmReady;
    const status = await invoke<string>("check_llm_server");
    if (status === "running") {
      setLlmStatus("running");
      return settings;
    }
    if (status === "loading") {
      setLlmStatus("loading");
      return settings;
    }
    if (!attemptStart) {
      setLlmStatus("stopped");
      return settings;
    }
    setLlmStatus("starting");
    await invoke("start_llm_server");
    setLlmStatus("loading");
    return settings;
  } catch {
    setLlmStatus("error");
    return null;
  }
}

export const servers = {
  get sttStatus() {
    return sttStatus;
  },
  get llmStatus() {
    return llmStatus;
  },
  get modelReady() {
    return modelReady;
  },
  get llmReady() {
    return llmReady;
  },
  refreshStt,
  refreshLlm,
};
