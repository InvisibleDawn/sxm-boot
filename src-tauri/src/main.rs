// Copyright © 2026 Laurent Honoré
// SPDX-License-Identifier: GPL-3.0-or-later
// SXM Boot — créateur de clé USB multiboot (moteur Ventoy)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

#[cfg(target_os = "macos")]
mod macos;

static CANCEL: AtomicBool = AtomicBool::new(false);
const CHUNK: usize = 4 * 1024 * 1024;

#[derive(Serialize, Clone)]
pub(crate) struct Drive {
    path: String,
    name: String,
    size: u64,
    ventoy: bool,
    data_part: Option<String>,
    mountpoint: Option<String>,
    used: Option<u64>,
    avail: Option<u64>,
}

#[derive(Serialize, Clone)]
struct Progress {
    id: String,
    phase: String, // "download" | "copy" | "verify" | "done" | "error"
    done: u64,
    total: u64,
    message: Option<String>,
}

#[derive(Serialize)]
struct IsoFile {
    name: String,
    size: u64,
}

fn as_u64(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

fn first_mount(v: &Value) -> Option<String> {
    if let Some(arr) = v.get("mountpoints").and_then(|m| m.as_array()) {
        return arr.iter().find_map(|m| m.as_str().map(String::from));
    }
    v.get("mountpoint").and_then(|m| m.as_str()).map(String::from)
}

pub(crate) async fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("{cmd} introuvable : {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Liste uniquement les disques USB (jamais les disques internes).
#[tauri::command]
async fn list_drives() -> Result<Vec<Drive>, String> {
    #[cfg(target_os = "macos")]
    return macos::list_drives().await;
    #[allow(unreachable_code)]
    let json = run(
        "lsblk",
        &["-J", "-b", "-o", "PATH,NAME,SIZE,MODEL,VENDOR,TRAN,TYPE,LABEL,FSUSED,FSAVAIL,MOUNTPOINTS"],
    )
    .await?;
    let root: Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let mut drives = Vec::new();
    for dev in root["blockdevices"].as_array().cloned().unwrap_or_default() {
        if dev["type"].as_str() != Some("disk") || dev["tran"].as_str() != Some("usb") {
            continue;
        }
        let model = dev["model"].as_str().unwrap_or("").trim();
        let vendor = dev["vendor"].as_str().unwrap_or("").trim();
        let name = format!("{vendor} {model}").trim().to_string();
        let children = dev["children"].as_array().cloned().unwrap_or_default();
        let data = children.iter().find(|c| c["label"].as_str() == Some("Ventoy"));
        drives.push(Drive {
            path: dev["path"].as_str().unwrap_or("").to_string(),
            name: if name.is_empty() { "Clé USB".into() } else { name },
            size: as_u64(&dev["size"]).unwrap_or(0),
            ventoy: data.is_some(),
            data_part: data.and_then(|d| d["path"].as_str().map(String::from)),
            mountpoint: data.and_then(first_mount),
            used: data.and_then(|d| as_u64(&d["fsused"])),
            avail: data.and_then(|d| as_u64(&d["fsavail"])),
        });
    }
    Ok(drives)
}

fn ventoy_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("sxm-boot/ventoy")
}

fn find_ventoy() -> Option<PathBuf> {
    let base = ventoy_dir();
    if let Ok(entries) = std::fs::read_dir(&base) {
        for e in entries.flatten() {
            let p = e.path().join("Ventoy2Disk.sh");
            if p.exists() {
                return Some(p);
            }
        }
    }
    let opt = PathBuf::from("/opt/ventoy/Ventoy2Disk.sh");
    opt.exists().then_some(opt)
}

#[tauri::command]
fn ventoy_status() -> Option<String> {
    find_ventoy().map(|p| p.display().to_string())
}

/// Télécharge la dernière version de Ventoy depuis GitHub.
#[tauri::command]
async fn fetch_ventoy(app: AppHandle) -> Result<String, String> {
    if cfg!(target_os = "macos") {
        return Err("Ventoy ne peut pas être installé depuis macOS. Prépare la clé une fois sous Linux.".into());
    }
    let client = http()?;
    let rel: Value = client
        .get("https://api.github.com/repos/ventoy/Ventoy/releases/latest")
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    let asset = rel["assets"].as_array().and_then(|a| {
        a.iter().find(|x| x["name"].as_str().map_or(false, |n| n.ends_with("-linux.tar.gz")))
    }).ok_or("Archive Linux de Ventoy introuvable")?;
    let url = asset["browser_download_url"].as_str().ok_or("URL manquante")?;
    let dir = ventoy_dir();
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let tar = dir.join("ventoy.tar.gz");
    download_to(&app, "ventoy", url, &tar, None).await?;
    run("tar", &["-xzf", tar.to_str().unwrap(), "-C", dir.to_str().unwrap()]).await?;
    let _ = tokio::fs::remove_file(&tar).await;
    find_ventoy().map(|p| p.display().to_string()).ok_or("Extraction échouée".into())
}

/// Installe Ventoy sur la clé (efface tout). Demande le mot de passe via polkit.
#[tauri::command]
async fn install_ventoy(device: String) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        return Err("Ventoy ne peut pas être installé depuis macOS. Prépare la clé une fois sous Linux.".into());
    }
    let drives = list_drives().await?;
    if !drives.iter().any(|d| d.path == device) {
        return Err("Ce périphérique n'est pas une clé USB détectée.".into());
    }
    let script = find_ventoy().ok_or("Ventoy n'est pas encore téléchargé.")?;
    let dir = script.parent().unwrap().display().to_string();
    // Démonter les partitions avant d'écrire
    let _ = Command::new("sh").arg("-c")
        .arg(format!("for p in {device}?*; do udisksctl unmount -b \"$p\" --no-user-interaction; done"))
        .output().await;
    let mut child = Command::new("pkexec")
        .args(["sh", "-c", &format!("cd '{dir}' && ./Ventoy2Disk.sh -I '{device}'")])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"y\ny\n").await; // Ventoy demande deux confirmations
    }
    let out = child.wait_with_output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        let log = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
        return Err(format!("Échec de l'installation de Ventoy :\n{}", log.trim()));
    }
    Ok(())
}

/// Monte la partition de données Ventoy et renvoie son point de montage.
#[tauri::command]
async fn mount_drive(device: String) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    return macos::mount_drive(&device).await;
    #[allow(unreachable_code)]
    let d = list_drives().await?.into_iter().find(|d| d.path == device).ok_or("Clé introuvable")?;
    if let Some(m) = d.mountpoint {
        return Ok(m);
    }
    let part = d.data_part.ok_or("Cette clé n'a pas encore Ventoy.")?;
    let out = run("udisksctl", &["mount", "-b", &part, "--no-user-interaction"]).await?;
    // Sortie : « Mounted /dev/sdb1 at /run/media/laurent/Ventoy »
    out.split(" at ").nth(1).map(|s| s.trim().trim_end_matches('.').to_string())
        .ok_or_else(|| format!("Point de montage inconnu : {out}"))
}

#[tauri::command]
async fn list_isos(mountpoint: String) -> Result<Vec<IsoFile>, String> {
    let mut v = Vec::new();
    let mut rd = tokio::fs::read_dir(&mountpoint).await.map_err(|e| e.to_string())?;
    while let Ok(Some(e)) = rd.next_entry().await {
        let name = e.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        if lower.ends_with(".iso") || lower.ends_with(".img") {
            let size = e.metadata().await.map(|m| m.len()).unwrap_or(0);
            v.push(IsoFile { name, size });
        }
    }
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

#[tauri::command]
async fn delete_iso(mountpoint: String, name: String) -> Result<(), String> {
    if name.contains('/') { return Err("Nom invalide".into()); }
    tokio::fs::remove_file(Path::new(&mountpoint).join(name)).await.map_err(|e| e.to_string())
}

fn emit(app: &AppHandle, id: &str, phase: &str, done: u64, total: u64, msg: Option<String>) {
    let _ = app.emit("progress", Progress { id: id.into(), phase: phase.into(), done, total, message: msg });
}

/// Copie une ISO locale sur la clé, avec progression.
#[tauri::command]
async fn copy_iso(app: AppHandle, id: String, src: String, mountpoint: String) -> Result<(), String> {
    CANCEL.store(false, Ordering::SeqCst);
    let name = Path::new(&src).file_name().ok_or("Fichier invalide")?.to_owned();
    let dest = Path::new(&mountpoint).join(&name);
    let part = dest.with_extension("part");
    let total = tokio::fs::metadata(&src).await.map_err(|e| e.to_string())?.len();
    let mut input = tokio::fs::File::open(&src).await.map_err(|e| e.to_string())?;
    let mut out = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; CHUNK];
    let mut done = 0u64;
    loop {
        if CANCEL.load(Ordering::SeqCst) {
            drop(out);
            let _ = tokio::fs::remove_file(&part).await;
            return Err("Annulé".into());
        }
        let n = input.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 { break; }
        out.write_all(&buf[..n]).await.map_err(|e| e.to_string())?;
        done += n as u64;
        emit(&app, &id, "copy", done, total, None);
    }
    out.sync_all().await.map_err(|e| e.to_string())?;
    tokio::fs::rename(&part, &dest).await.map_err(|e| e.to_string())?;
    emit(&app, &id, "done", total, total, None);
    Ok(())
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent("SXM-Boot/0.1").build().map_err(|e| e.to_string())
}

async fn download_to(app: &AppHandle, id: &str, url: &str, dest: &Path, sha256: Option<&str>) -> Result<(), String> {
    let resp = http()?.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Téléchargement refusé ({})", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let part = dest.with_extension("part");
    let mut out = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    let mut stream = resp.bytes_stream();
    let mut last = 0u64;
    while let Some(chunk) = stream.next().await {
        if CANCEL.load(Ordering::SeqCst) {
            drop(out);
            let _ = tokio::fs::remove_file(&part).await;
            return Err("Annulé".into());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        out.write_all(&chunk).await.map_err(|e| e.to_string())?;
        if sha256.is_some() { hasher.update(&chunk); }
        done += chunk.len() as u64;
        if done - last > 1_000_000 || done == total {
            last = done;
            emit(app, id, "download", done, total, None);
        }
    }
    out.sync_all().await.map_err(|e| e.to_string())?;
    drop(out);
    if let Some(expected) = sha256 {
        emit(app, id, "verify", done, done, None);
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected.trim()) {
            let _ = tokio::fs::remove_file(&part).await;
            return Err("Somme SHA256 incorrecte : fichier supprimé.".into());
        }
    }
    tokio::fs::rename(&part, dest).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Télécharge une ISO directement sur la clé.
#[tauri::command]
async fn download_iso(app: AppHandle, id: String, url: String, filename: String, mountpoint: String, sha256: Option<String>) -> Result<(), String> {
    CANCEL.store(false, Ordering::SeqCst);
    if filename.contains('/') || filename.is_empty() { return Err("Nom de fichier invalide".into()); }
    let dest = Path::new(&mountpoint).join(&filename);
    download_to(&app, &id, &url, &dest, sha256.as_deref().filter(|s| !s.is_empty())).await?;
    emit(&app, &id, "done", 1, 1, None);
    Ok(())
}

#[derive(Serialize)]
struct Latest {
    version: String,
    url: String,
    filename: String,
    sums: Option<String>,
}

fn hrefs(html: &str) -> Vec<String> {
    html.split("href=\"").skip(1)
        .filter_map(|s| s.split('"').next().map(String::from))
        .collect()
}

fn ver_key(v: &str) -> Vec<u32> {
    v.trim_end_matches('/').split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// Trouve la dernière version publiée dans un dossier de miroir (ex. Fedora 45, Ubuntu MATE 26.10).
#[tauri::command]
async fn resolve_latest(base: String, subpath: String, prefix: String, suffix: String) -> Result<Latest, String> {
    let client = http()?;
    let index = client.get(&base).send().await.map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    let mut versions: Vec<String> = hrefs(&index).into_iter()
        .map(|h| h.trim_end_matches('/').to_string())
        .filter(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_digit() || c == '.') && h.chars().next().unwrap().is_ascii_digit())
        .collect();
    versions.sort_by(|a, b| ver_key(b).cmp(&ver_key(a)));
    versions.dedup();
    for v in versions.iter().take(4) {
        let dir = format!("{}{}/{}", base, v, subpath);
        let Ok(resp) = client.get(&dir).send().await else { continue };
        if !resp.status().is_success() { continue; }
        let Ok(page) = resp.text().await else { continue };
        let links = hrefs(&page);
        let mut isos: Vec<&String> = links.iter()
            .filter(|h| h.starts_with(&prefix) && h.ends_with(&suffix)).collect();
        isos.sort();
        if let Some(iso) = isos.last() {
            let sums = links.iter().find(|h| h.contains("CHECKSUM") || h.as_str() == "SHA256SUMS")
                .map(|s| format!("{dir}{s}"));
            return Ok(Latest { version: v.clone(), url: format!("{dir}{iso}"), filename: iso.to_string(), sums });
        }
    }
    Err("Aucune version trouvée".into())
}

/// Récupère la somme SHA256 d'un fichier dans un fichier de sommes publié (formats Fedora, Debian, Ubuntu, Mint).
#[tauri::command]
async fn fetch_sha(sums_url: String, filename: String) -> Result<Option<String>, String> {
    let text = http()?.get(&sums_url).send().await.map_err(|e| e.to_string())?
        .text().await.map_err(|e| e.to_string())?;
    for line in text.lines().filter(|l| l.contains(&filename)) {
        let hash = line.split(|c: char| !c.is_ascii_hexdigit())
            .find(|t| t.len() == 64);
        if let Some(h) = hash { return Ok(Some(h.to_lowercase())); }
    }
    Ok(None)
}

#[tauri::command]
async fn file_size(path: String) -> Result<u64, String> {
    tokio::fs::metadata(&path).await.map(|m| m.len()).map_err(|e| e.to_string())
}

#[tauri::command]
fn platform() -> &'static str {
    std::env::consts::OS
}

#[tauri::command]
fn cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}

#[tauri::command]
async fn eject(device: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::eject(&device).await;
    #[allow(unreachable_code)]
    if !list_drives().await?.iter().any(|d| d.path == device) {
        return Err("Clé introuvable".into());
    }
    let _ =Command::new("sh").arg("-c")
        .arg(format!("sync; for p in {device}?*; do udisksctl unmount -b \"$p\" --no-user-interaction; done"))
        .output().await;
    run("udisksctl", &["power-off", "-b", &device, "--no-user-interaction"]).await.map(|_| ())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_drives, ventoy_status, fetch_ventoy, install_ventoy, mount_drive,
            list_isos, delete_iso, copy_iso, download_iso, fetch_sha, resolve_latest, file_size, platform, cancel, eject
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de SXM Boot");
}
