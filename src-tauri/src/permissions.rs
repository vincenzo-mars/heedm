//! Stato e gestione dei permessi OS richiesti da Heedm (microfono, registrazione
//! schermo). Vive fuori da `recorder/`: non è cattura audio, è integrazione OS.

#[cfg(target_os = "macos")]
mod macos {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    pub fn screen_recording_granted() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    /// Innesca il prompt di sistema se lo stato non è ancora deciso; no-op se
    /// già concesso/negato. Ritorna lo stato risultante.
    pub fn request_screen_recording() -> bool {
        unsafe { CGRequestScreenCaptureAccess() }
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    pub fn screen_recording_granted() -> bool {
        true
    }

    pub fn request_screen_recording() -> bool {
        true
    }
}

pub fn screen_recording_granted() -> bool {
    macos::screen_recording_granted()
}

pub fn request_screen_recording() -> bool {
    macos::request_screen_recording()
}

pub enum PermissionPane {
    Microphone,
    ScreenRecording,
}

/// Apre il pannello di Privacy & Security pertinente in System Settings.
/// `pane` è un enum chiuso: nessuna stringa esterna finisce nel comando.
pub fn open_settings_pane(pane: PermissionPane) -> Result<(), String> {
    let url = match pane {
        PermissionPane::Microphone => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        PermissionPane::ScreenRecording => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }
    };

    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|e| format!("Impossibile aprire Impostazioni di Sistema: {e}"))?;

    Ok(())
}
