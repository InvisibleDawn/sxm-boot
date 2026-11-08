// Copyright © 2026 Laurent Honoré
// SPDX-License-Identifier: GPL-3.0-or-later
// Gestion des clés USB sous macOS (diskutil).

use crate::{run, Drive};
use plist::Value;

async fn plist(args: &[&str]) -> Result<Value, String> {
    let out = run("diskutil", args).await?;
    Value::from_reader_xml(out.as_bytes()).map_err(|e| e.to_string())
}

fn s(d: &plist::Dictionary, k: &str) -> Option<String> {
    d.get(k).and_then(|v| v.as_string()).map(String::from)
}

fn u(d: &plist::Dictionary, k: &str) -> Option<u64> {
    d.get(k).and_then(|v| v.as_unsigned_integer())
}

/// Espace utilisé / libre d'un volume monté, via `df -k`.
async fn usage(mount: &str) -> (Option<u64>, Option<u64>) {
    let Ok(out) = run("df", &["-k", mount]).await else { return (None, None) };
    let Some(line) = out.lines().nth(1) else { return (None, None) };
    let cols: Vec<&str> = line.split_whitespace().collect();
    let used = cols.get(2).and_then(|v| v.parse::<u64>().ok()).map(|v| v * 1024);
    let avail = cols.get(3).and_then(|v| v.parse::<u64>().ok()).map(|v| v * 1024);
    (used, avail)
}

/// Liste les disques externes physiques (clés USB), jamais le disque interne.
pub async fn list_drives() -> Result<Vec<Drive>, String> {
    let root = plist(&["list", "-plist", "external", "physical"]).await?;
    let disks = root.as_dictionary()
        .and_then(|d| d.get("AllDisksAndPartitions"))
        .and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut drives = Vec::new();
    for disk in disks {
        let Some(disk) = disk.as_dictionary() else { continue };
        let Some(id) = s(disk, "DeviceIdentifier") else { continue };
        let dev = format!("/dev/{id}");
        // Nom commercial de la clé
        let name = match plist(&["info", "-plist", &dev]).await {
            Ok(info) => info.as_dictionary().and_then(|i| s(i, "MediaName")).unwrap_or_default(),
            Err(_) => String::new(),
        };
        let parts = disk.get("Partitions").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let data = parts.iter().filter_map(|p| p.as_dictionary())
            .find(|p| s(p, "VolumeName").as_deref() == Some("Ventoy"));
        let mount = data.and_then(|p| s(p, "MountPoint")).filter(|m| !m.is_empty());
        let (used, avail) = match &mount { Some(m) => usage(m).await, None => (None, None) };
        drives.push(Drive {
            path: dev,
            name: if name.trim().is_empty() { "Clé USB".into() } else { name.trim().into() },
            size: u(disk, "Size").unwrap_or(0),
            ventoy: data.is_some(),
            data_part: data.and_then(|p| s(p, "DeviceIdentifier")).map(|i| format!("/dev/{i}")),
            mountpoint: mount,
            used,
            avail,
        });
    }
    Ok(drives)
}

pub async fn mount_drive(device: &str) -> Result<String, String> {
    let d = list_drives().await?.into_iter().find(|d| d.path == device).ok_or("Clé introuvable")?;
    if let Some(m) = d.mountpoint {
        return Ok(m);
    }
    let part = d.data_part.ok_or("Cette clé n'a pas encore Ventoy. Prépare-la une fois sous Linux.")?;
    run("diskutil", &["mount", &part]).await?;
    let info = plist(&["info", "-plist", &part]).await?;
    info.as_dictionary().and_then(|i| s(i, "MountPoint")).filter(|m| !m.is_empty())
        .ok_or_else(|| "Point de montage inconnu".into())
}

pub async fn eject(device: &str) -> Result<(), String> {
    if !list_drives().await?.iter().any(|d| d.path == device) {
        return Err("Clé introuvable".into());
    }
    let _ = run("sync", &[]).await;
    run("diskutil", &["eject", device]).await.map(|_| ())
}
