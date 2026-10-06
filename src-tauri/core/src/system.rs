//! Actions système : extinction planifiée de Windows, ouverture de dossiers.

use std::path::Path;
use std::process::Command;

#[cfg(windows)]
fn quiet(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000)
}

#[cfg(not(windows))]
fn quiet(cmd: &mut Command) -> &mut Command {
    cmd
}

/// Programme l'extinction de Windows dans `seconds` secondes (annulable avec `cancel_os_shutdown`).
pub fn schedule_os_shutdown(seconds: u64, comment: &str) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Extinction planifiée : Windows uniquement".into());
    }
    let comment: String = comment.chars().take(500).collect();
    let out = quiet(Command::new("shutdown").args(["/s", "/t", &seconds.to_string(), "/c", &comment]))
        .output()
        .map_err(|e| format!("shutdown : {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn cancel_os_shutdown() -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Extinction planifiée : Windows uniquement".into());
    }
    let out = quiet(Command::new("shutdown").arg("/a")).output().map_err(|e| format!("shutdown : {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn open_in_file_manager(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Introuvable : {}", path.display()));
    }
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("explorer");
        c.arg(path);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(path);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(path);
        c
    };
    quiet(&mut cmd).spawn().map(|_| ()).map_err(|e| format!("Ouverture : {e}"))
}
