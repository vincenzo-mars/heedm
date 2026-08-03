//! Primitive condivise di processo/porta per i server locali (whisper, llm).
//! Estratte da `stt.rs` quando è arrivato un secondo server con lo stesso
//! ciclo di vita: la logica di stop porta due invarianti non ovvi (guard
//! rilasciato prima di ogni `.await`, mai kill-by-porta) che una copia-incolla
//! avrebbe finito per rompere in uno dei due punti.

use std::sync::Mutex;

pub(crate) async fn port_is_open(port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .is_ok()
}

pub(crate) async fn wait_for_port(port: u16, timeout_secs: u64) -> Result<(), String> {
    for _ in 0..timeout_secs {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if port_is_open(port).await {
            return Ok(());
        }
    }
    Err(format!("Server locale non pronto dopo {timeout_secs}s"))
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
            child.start_kill().map_err(|e| e.to_string())?;
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
