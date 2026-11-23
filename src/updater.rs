use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use serde::Deserialize;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const RELEASES_API: &str =
    "https://api.github.com/repos/AlejandroEspinosaLeal/agenda/releases/latest";

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Downloading(String),
    Error(String),
}

enum WorkerMsg {
    CheckDone(Result<Option<ReleaseInfo>, String>),
    DownloadDone(Result<PathBuf, String>),
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub struct Updater {
    pub status: UpdateStatus,
    rx: Option<Receiver<WorkerMsg>>,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            status: UpdateStatus::Idle,
            rx: None,
        }
    }

    /// Starts an asynchronous check against GitHub Releases.
    pub fn check_for_updates(&mut self) {
        if matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading(_)
        ) {
            return;
        }
        self.status = UpdateStatus::Checking;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let res = fetch_latest_release();
            let _ = tx.send(WorkerMsg::CheckDone(res));
        });
    }

    /// Downloads the installer for `release` to `%TEMP%` and signals completion.
    pub fn start_download(&mut self, release: ReleaseInfo) {
        if matches!(self.status, UpdateStatus::Downloading(_)) {
            return;
        }
        self.status = UpdateStatus::Downloading(release.version.clone());
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let res = download_installer(&release.download_url);
            let _ = tx.send(WorkerMsg::DownloadDone(res));
        });
    }

    /// Polls background update worker; returns `Some(installer_path)` when ready to launch.
    pub fn poll(&mut self) -> Option<PathBuf> {
        let rx = self.rx.as_ref()?;
        let msg = rx.try_recv().ok()?;
        self.rx = None;

        match msg {
            WorkerMsg::CheckDone(Ok(Some(info))) => {
                // Auto-download as soon as a newer GitHub release is detected
                self.start_download(info);
            }
            WorkerMsg::CheckDone(Ok(None)) => {
                self.status = UpdateStatus::UpToDate;
            }
            WorkerMsg::CheckDone(Err(err)) => {
                self.status = UpdateStatus::Error(err);
            }
            WorkerMsg::DownloadDone(Ok(path)) => {
                return Some(path);
            }
            WorkerMsg::DownloadDone(Err(err)) => {
                self.status = UpdateStatus::Error(err);
            }
        }
        None
    }
}

/// Launches the downloaded Inno Setup installer silently and returns whether it started.
pub fn launch_installer(path: &std::path::Path) -> Result<(), String> {
    Command::new(path)
        .args(["/SILENT", "/FORCECLOSEAPPLICATIONS"])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("No se pudo iniciar el instalador: {e}"))
}

fn fetch_latest_release() -> Result<Option<ReleaseInfo>, String> {
    let script = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         $r = Invoke-RestMethod -Uri '{RELEASES_API}' -Headers @{{'User-Agent'='MiAgenda-Updater'}} -TimeoutSec 10; \
         $r | ConvertTo-Json -Depth 4 -Compress"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Error al comprobar actualizaciones: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if err.contains("404") {
            return Ok(None);
        }
        return Err("No se pudo conectar con GitHub Releases".to_owned());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let release: GitHubRelease = serde_json::from_str(raw.trim())
        .map_err(|e| format!("Respuesta inválida de GitHub: {e}"))?;

    if !is_newer_version(&release.tag_name, CURRENT_VERSION) {
        return Ok(None);
    }

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("Mi_Agenda_Setup.exe"))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")))
        .ok_or_else(|| "La release no contiene un instalador .exe".to_owned())?;

    Ok(Some(ReleaseInfo {
        version: release.tag_name,
        download_url: asset.browser_download_url.clone(),
    }))
}

fn download_installer(url: &str) -> Result<PathBuf, String> {
    let dest = std::env::temp_dir().join("Mi_Agenda_Setup_Update.exe");
    let dest_str = dest.to_string_lossy().replace('\'', "''");
    let url_safe = url.replace('\'', "''");

    let script = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
         Invoke-WebRequest -Uri '{url_safe}' -OutFile '{dest_str}' -Headers @{{'User-Agent'='MiAgenda-Updater'}} -TimeoutSec 60"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Error al descargar actualización: {e}"))?;

    if !output.status.success() {
        return Err("Falló la descarga de la actualización".to_owned());
    }
    Ok(dest)
}

/// Parses `"v1.2.3"` or `"1.2.3"` into `(major, minor, patch)`.
pub fn parse_semver(tag: &str) -> Option<(u32, u32, u32)> {
    let clean = tag.trim().strip_prefix('v').unwrap_or(tag.trim());
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch_raw = parts.next().unwrap_or("0").split('-').next().unwrap_or("0");
    let patch = patch_raw.parse().ok()?;
    Some((major, minor, patch))
}

/// Returns `true` if `remote` is strictly newer than `current`.
pub fn is_newer_version(remote: &str, current: &str) -> bool {
    match (parse_semver(remote), parse_semver(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_should_detect_newer_releases() {
        assert!(is_newer_version("v1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.9"));
        assert!(!is_newer_version("v1.0.0", "1.0.0"));
        assert!(!is_newer_version("v0.9.9", "1.0.0"));
    }
}
