use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const RELEASES_API: &str =
    "https://api.github.com/repos/AlejandroEspinosaLeal/agenda/releases/latest";
const INSTALLER_NAME: &str = "Mi_Agenda_Setup.exe";

const ENV_RELEASES_URL: &str = "MIAGENDA_RELEASES_URL";
const ENV_UPDATE_URL: &str = "MIAGENDA_UPDATE_URL";
const ENV_UPDATE_DEST: &str = "MIAGENDA_UPDATE_DEST";

/// Reads the latest release metadata. All user-influenced values are passed via
/// environment variables so nothing is interpolated into the PowerShell script.
const FETCH_SCRIPT: &str = r#"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12;
$r = Invoke-RestMethod -Uri $env:MIAGENDA_RELEASES_URL -Headers @{'User-Agent'='MiAgenda-Updater'} -TimeoutSec 10;
$r | ConvertTo-Json -Depth 4 -Compress
"#;

/// Downloads a single file to the staging directory. URL and destination are
/// read from `$env:` variables to avoid any command-string interpolation.
const DOWNLOAD_SCRIPT: &str = r#"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12;
Invoke-WebRequest -Uri $env:MIAGENDA_UPDATE_URL -OutFile $env:MIAGENDA_UPDATE_DEST -Headers @{'User-Agent'='MiAgenda-Updater'} -TimeoutSec 60
"#;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub download_url: String,
    /// Optional `.sha256` asset published next to the installer.
    pub sha256_url: Option<String>,
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

    /// Downloads the installer into the app staging folder and signals completion.
    pub fn start_download(&mut self, release: ReleaseInfo) {
        if matches!(self.status, UpdateStatus::Downloading(_)) {
            return;
        }
        self.status = UpdateStatus::Downloading(release.version.clone());
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let res = download_installer(&release.download_url, release.sha256_url.as_deref());
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
pub fn launch_installer(path: &Path) -> Result<(), String> {
    Command::new(path)
        .args(["/SILENT", "/FORCECLOSEAPPLICATIONS"])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("No se pudo iniciar el instalador: {e}"))
}

/// Absolute path to Windows PowerShell so a planted binary in the working
/// directory cannot hijack the call.
fn powershell_exe() -> PathBuf {
    let root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    root.join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
}

/// Accepts only HTTPS GitHub Releases downloads and GitHub's asset CDN.
fn is_allowed_github_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("https://") else {
        return false;
    };
    if rest.starts_with("github.com/") {
        return rest.contains("/releases/download/");
    }
    rest.starts_with("objects.githubusercontent.com/")
}

fn fetch_latest_release() -> Result<Option<ReleaseInfo>, String> {
    let output = Command::new(powershell_exe())
        .args(["-NoProfile", "-NonInteractive", "-Command", FETCH_SCRIPT])
        .env(ENV_RELEASES_URL, RELEASES_API)
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
        .find(|a| a.name.eq_ignore_ascii_case(INSTALLER_NAME))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")))
        .ok_or_else(|| "La release no contiene un instalador .exe".to_owned())?;

    if !is_allowed_github_url(&asset.browser_download_url) {
        return Err("La release apunta a una URL de descarga no permitida".to_owned());
    }

    let sha_name = format!("{}.sha256", asset.name);
    let sha256_url = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(&sha_name))
        .map(|a| a.browser_download_url.clone())
        .filter(|url| is_allowed_github_url(url));

    Ok(Some(ReleaseInfo {
        version: release.tag_name,
        download_url: asset.browser_download_url.clone(),
        sha256_url,
    }))
}

/// `%LOCALAPPDATA%\organizador_tareas\updates`, falling back to `%TEMP%`.
fn updates_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("organizador_tareas")
        .join("updates")
}

/// Unique installer name built from the process id and the current time.
fn unique_installer_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    format!("Mi_Agenda_Setup_update_{nanos:032x}_{pid:08x}.exe")
}

fn download_with_powershell(url: &str, dest: &Path) -> Result<(), String> {
    if !is_allowed_github_url(url) {
        return Err("URL de descarga de actualización no permitida".to_owned());
    }
    let output = Command::new(powershell_exe())
        .args(["-NoProfile", "-NonInteractive", "-Command", DOWNLOAD_SCRIPT])
        .env(ENV_UPDATE_URL, url)
        .env(ENV_UPDATE_DEST, dest.as_os_str())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Error al descargar actualización: {e}"))?;

    if !output.status.success() {
        return Err("Falló la descarga de la actualización".to_owned());
    }
    Ok(())
}

fn download_installer(url: &str, sha256_url: Option<&str>) -> Result<PathBuf, String> {
    let dir = updates_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("No se pudo preparar la carpeta de actualizaciones: {e}"))?;

    let file_name = unique_installer_name();
    let dest = dir.join(&file_name);

    download_with_powershell(url, &dest)?;

    if let Some(sha_url) = sha256_url {
        let sha_path = dir.join(format!("{file_name}.sha256"));
        let verify = download_with_powershell(sha_url, &sha_path)
            .and_then(|_| verify_sha256(&dest, &sha_path));
        let _ = std::fs::remove_file(&sha_path);
        if let Err(err) = verify {
            let _ = std::fs::remove_file(&dest);
            return Err(err);
        }
    } else {
        eprintln!(
            "[updater] La release no publica {INSTALLER_NAME}.sha256; no se pudo verificar la integridad del instalador."
        );
    }

    Ok(dest)
}

/// Compares the installer bytes against the published `.sha256` file.
fn verify_sha256(file: &Path, sha_file: &Path) -> Result<(), String> {
    let listing = std::fs::read_to_string(sha_file)
        .map_err(|e| format!("No se pudo leer el archivo .sha256: {e}"))?;
    let expected = parse_sha256(&listing)
        .ok_or_else(|| "El archivo .sha256 no contiene un hash válido".to_owned())?;
    let data = std::fs::read(file).map_err(|e| format!("No se pudo leer el instalador: {e}"))?;
    let actual = hex_encode(&sha256(&data));
    if actual.eq_ignore_ascii_case(&expected) {
        Ok(())
    } else {
        Err(
            "El SHA-256 del instalador no coincide con el publicado; se ha descartado la descarga."
                .to_owned(),
        )
    }
}

/// Extracts the first 64-hex-digit token from `sha256sum`/`Get-FileHash` output.
fn parse_sha256(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_ascii_hexdigit()))
        .find(|token| token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|token| token.to_owned())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Pure-Rust SHA-256 (FIPS 180-4) so no extra dependency is required.
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
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

    #[test]
    fn sha256_should_match_known_vectors() {
        assert_eq!(
            hex_encode(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex_encode(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex_encode(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn allowed_urls_should_be_limited_to_github_releases() {
        assert!(is_allowed_github_url(
            "https://github.com/AlejandroEspinosaLeal/agenda/releases/download/v1.2.3/Mi_Agenda_Setup.exe"
        ));
        assert!(is_allowed_github_url(
            "https://objects.githubusercontent.com/github-production-release-asset/x"
        ));
        assert!(!is_allowed_github_url(
            "http://github.com/AlejandroEspinosaLeal/agenda/releases/download/v1/Mi_Agenda_Setup.exe"
        ));
        assert!(!is_allowed_github_url(
            "https://evil.example/github.com/releases/download/x.exe"
        ));
        assert!(!is_allowed_github_url(
            "https://github.com/AlejandroEspinosaLeal/agenda/archive/refs/heads/main.zip"
        ));
    }

    #[test]
    fn sha256_listing_should_parse_common_formats() {
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(
            parse_sha256(&format!("{hash}  Mi_Agenda_Setup.exe")).as_deref(),
            Some(hash)
        );
        assert_eq!(
            parse_sha256(&format!("Hash : {hash}\n")).as_deref(),
            Some(hash)
        );
        assert_eq!(parse_sha256("no hash here"), None);
    }
}
