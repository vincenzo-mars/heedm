//! Primitive condivise di processo/porta per i server locali (whisper, llm).
//! Estratte da `stt.rs` quando è arrivato un secondo server con lo stesso
//! ciclo di vita: la logica di stop porta due invarianti non ovvi (guard
//! rilasciato prima di ogni `.await`, mai kill-by-porta) che una copia-incolla
//! avrebbe finito per rompere in uno dei due punti.

use std::path::Path;
use std::sync::Mutex;

pub(crate) async fn port_is_open(port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .is_ok()
}

/// Poll a 250ms invece che a 1s: il server apre la porta in frazioni di
/// secondo dopo il load, e la granularità grossa aggiungeva fino a 1s di
/// latenza percepita sull'avvio.
pub(crate) async fn wait_for_port(port: u16, timeout_secs: u64) -> Result<(), String> {
    for _ in 0..timeout_secs * 4 {
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        if port_is_open(port).await {
            return Ok(());
        }
    }
    Err(format!("Server locale non pronto dopo {timeout_secs}s"))
}

/// Thread da passare ai server locali: tutti i core meno due (uno per l'app,
/// uno per l'OS), mai sotto 1.
pub(crate) fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(2).max(1))
        .unwrap_or(4)
}

/// Spawn di un server locale + registrazione del child nello slot tracciato,
/// così `stop_tracked_server` e il cleanup alla chiusura possono terminarlo.
///
/// `kill_on_drop(true)` è la rete di sicurezza per i percorsi in cui il `Child`
/// viene droppato senza un kill esplicito (unwind da panic, o un errore a metà
/// di `stop_tracked_server`): senza, il processo resterebbe vivo tenendo la
/// porta occupata e non tracciato da nessuno. Non copre la morte violenta
/// dell'app (SIGKILL sul padre): lì nessun codice del padre gira più.
pub(crate) fn spawn_tracked(
    bin: &Path,
    args: &[&str],
    slot: &Mutex<Option<tokio::process::Child>>,
) -> Result<(), String> {
    let child = tokio::process::Command::new(bin)
        .args(args)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    match slot.lock() {
        Ok(mut guard) => {
            *guard = Some(child);
            Ok(())
        }
        Err(e) => Err(format!("Stato del server corrotto: {e}")),
    }
}

/// Kill best-effort del child tracciato, usato alla chiusura dell'app: niente
/// `wait()` (il processo muore da solo, l'app sta uscendo).
pub(crate) fn kill_tracked(slot: &Mutex<Option<tokio::process::Child>>) {
    if let Some(mut child) = slot.lock().ok().and_then(|mut guard| guard.take()) {
        let _ = child.start_kill();
    }
}

/// Secondi di attesa dopo il kill prima di arrendersi: il processo può tenere
/// il socket ancora un istante dopo che `wait()` è tornato.
const PORT_RELEASE_TIMEOUT_SECS: u64 = 5;

/// Ferma il processo tracciato in `slot`, se presente. Non termina mai un
/// processo per porta: se lo slot è vuoto ma la porta è occupata, è un
/// processo che Heedm non ha avviato e va lasciato stare.
pub(crate) async fn stop_tracked_server(
    slot: &Mutex<Option<tokio::process::Child>>,
    port: u16,
) -> Result<(), String> {
    // Il MutexGuard vive solo dentro il match: va tolto prima di qualsiasi
    // `.await` sul child (kill/wait), altrimenti il lock resterebbe tenuto
    // attraverso un punto di sospensione.
    let child = match slot.lock() {
        Ok(mut guard) => guard.take(),
        Err(e) => return Err(format!("Stato del server corrotto: {e}")),
    };

    match child {
        Some(mut child) => {
            // `take()` sopra ha già tolto il child dallo slot: se il segnale non
            // parte, il processo è ancora vivo e va rimesso dov'era, altrimenti
            // resterebbe senza nessuno che lo traccia (e `kill_tracked` alla
            // chiusura dell'app troverebbe lo slot vuoto). Un `wait` fallito
            // invece non si recupera: lì il SIGKILL è già partito.
            if let Err(e) = child.start_kill() {
                if let Ok(mut guard) = slot.lock() {
                    *guard = Some(child);
                }
                return Err(e.to_string());
            }
            child.wait().await.map_err(|e| e.to_string())?;
        }
        None => {
            if port_is_open(port).await {
                return Err(format!(
                    "La porta {port} è occupata da un processo che Heedm non ha avviato: chiudilo manualmente prima di riprovare"
                ));
            }
            return Ok(());
        }
    }

    if !port_is_open(port).await {
        return Ok(());
    }
    for _ in 0..PORT_RELEASE_TIMEOUT_SECS {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if !port_is_open(port).await {
            return Ok(());
        }
    }

    Err(format!(
        "La porta {port} resta occupata {PORT_RELEASE_TIMEOUT_SECS}s dopo lo stop del server"
    ))
}
