// Copyright © 2026 Laurent Honoré
// SPDX-License-Identifier: GPL-3.0-or-later
// Mises à jour automatiques à partir des versions publiées sur GitHub.

use crate::{download_to, http};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use tauri::AppHandle;

const RELEASES_API: &str = "https://api.github.com/repos/InvisibleDawn/sxm-boot/releases/latest";

#[derive(Serialize)]
pub struct UpdateInfo {
    current: String,
    version: String,
    notes: String,
    published: String,
    page: String,
    asset_url: String,
    asset_name: String,
    digest: Option<String>,
    size: u64,
}

fn parse_version(v: &str) -> Vec<u64> {
    v.trim().trim_start_matches('v').split(['.', '-'])
        .map(|p| p.parse().unwrap_or(0)).take(3).collect()
}

/// Fichier d'installation adapté au système en cours.
fn wanted_asset(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    if cfg!(windows) { n.ends_with("-setup.exe") }
    else if cfg!(target_os = "macos") { n.ends_with(".dmg") }
    else { n.ends_with(".rpm") }
}

/// Seuls les fichiers publiés sur le dépôt officiel sont acceptés.
fn official_url(url: &str) -> bool {
    url.starts_with("https://github.com/InvisibleDawn/sxm-boot/releases/download/")
}

/// Renvoie la nouvelle version si elle est plus récente que celle installée.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let current = app.package_info().version.to_string();
    let resp = http()?.get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Vérification impossible ({})", resp.status()));
    }
    let rel: Value = resp.json().await.map_err(|e| e.to_string())?;
    let version = rel["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    if version.is_empty() || parse_version(&version) <= parse_version(&current) {
        return Ok(None);
    }
    let asset = rel["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"].as_str().map_or(false, wanted_asset)))
        .ok_or("Aucun installateur pour ce système dans la nouvelle version.")?;
    let asset_url = asset["browser_download_url"].as_str().unwrap_or_default().to_string();
    if !official_url(&asset_url) {
        return Err("Adresse de mise à jour non officielle.".into());
    }
    Ok(Some(UpdateInfo {
        current,
        version,
        notes: rel["body"].as_str().unwrap_or_default().to_string(),
        published: rel["published_at"].as_str().unwrap_or_default().to_string(),
        page: rel["html_url"].as_str().unwrap_or_default().to_string(),
        asset_name: asset["name"].as_str().unwrap_or_default().to_string(),
        digest: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(String::from),
        size: asset["size"].as_u64().unwrap_or(0),
        asset_url,
    }))
}

/// Télécharge l'installateur (vérifié en SHA256) puis lance l'installation.
#[tauri::command]
pub async fn install_update(app: AppHandle, url: String, name: String, digest: Option<String>) -> Result<String, String> {
    if !official_url(&url) || name.contains('/') || name.contains('\\') || !wanted_asset(&name) {
        return Err("Mise à jour refusée : fichier non officiel.".into());
    }
    let dir: PathBuf = std::env::temp_dir().join("sxm-boot-update");
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let file = dir.join(&name);
    let _ = tokio::fs::remove_file(&file).await;
    download_to(&app, "update", &url, &file, digest.as_deref()).await?;
    let path = file.display().to_string();

    #[cfg(windows)]
    {
        // Installation discrète (/P), puis relance de SXM Boot (/R). On quitte pour libérer les fichiers.
        std::process::Command::new(&path).args(["/P", "/R"]).spawn().map_err(|e| e.to_string())?;
        app.exit(0);
        return Ok("restart".into());
    }
    #[cfg(target_os = "macos")]
    {
        crate::run("open", &[&path]).await?;
        return Ok("dmg".into());
    }
    #[allow(unreachable_code)]
    {
        // Fedora : dnf installe le paquet par-dessus l'ancien (mot de passe demandé par polkit).
        let out = tokio::process::Command::new("pkexec")
            .args(["dnf", "install", "-y", &path])
            .output().await.map_err(|e| e.to_string())?;
        if !out.status.success() {
            let log = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if log.is_empty() { "Installation annulée.".into() } else { format!("Échec de l'installation : {log}") });
        }
        let _ = tokio::fs::remove_file(&file).await;
        Ok("installed".into())
    }
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
