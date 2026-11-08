// Copyright © 2026 Laurent Honoré
// SPDX-License-Identifier: GPL-3.0-or-later
// Gestion des clés USB sous Windows 10 / 11 (PowerShell + Ventoy2Disk.exe).

use crate::{run, Drive};
use serde_json::Value;
use std::path::PathBuf;

async fn ps(script: &str) -> Result<String, String> {
    run("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]).await
}

/// Les clés sont identifiées par "PhysicalDriveN" (numéro de disque Windows).
fn disk_number(device: &str) -> Result<u32, String> {
    device.trim_start_matches("PhysicalDrive").parse().map_err(|_| "Clé invalide".to_string())
}

/// Liste uniquement les disques USB, jamais les disques internes.
pub async fn list_drives() -> Result<Vec<Drive>, String> {
    let script = r#"
$out = @(Get-Disk | Where-Object { $_.BusType -eq 'USB' } | ForEach-Object {
  $d = $_
  $v = Get-Partition -DiskNumber $d.Number -ErrorAction SilentlyContinue |
       Get-Volume -ErrorAction SilentlyContinue |
       Where-Object { $_.FileSystemLabel -eq 'Ventoy' } | Select-Object -First 1
  [pscustomobject]@{
    Number = $d.Number; Name = $d.FriendlyName; Size = $d.Size
    Ventoy = [bool]$v
    Letter = if ($v -and $v.DriveLetter) { [string]$v.DriveLetter } else { $null }
    Total = if ($v) { $v.Size } else { $null }
    Free = if ($v) { $v.SizeRemaining } else { $null }
  }
})
ConvertTo-Json -InputObject $out -Compress
"#;
    let json = ps(script).await?;
    let v: Value = serde_json::from_str(json.trim()).map_err(|e| e.to_string())?;
    let list = match v { Value::Array(a) => a, Value::Null => vec![], other => vec![other] };
    Ok(list.iter().map(|d| {
        let mount = d["Letter"].as_str().filter(|l| !l.is_empty()).map(|l| format!("{l}:\\"));
        let total = d["Total"].as_u64();
        let free = d["Free"].as_u64();
        Drive {
            path: format!("PhysicalDrive{}", d["Number"].as_u64().unwrap_or(0)),
            name: d["Name"].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Clé USB".into()),
            size: d["Size"].as_u64().unwrap_or(0),
            ventoy: d["Ventoy"].as_bool().unwrap_or(false),
            data_part: None,
            mountpoint: mount,
            used: match (total, free) { (Some(t), Some(f)) => Some(t.saturating_sub(f)), _ => None },
            avail: free,
        }
    }).collect())
}

/// Windows monte la clé tout seul : on renvoie sa lettre (ex. E:\).
pub async fn mount_drive(device: &str) -> Result<String, String> {
    let d = list_drives().await?.into_iter().find(|d| d.path == device).ok_or("Clé introuvable")?;
    if !d.ventoy {
        return Err("Cette clé n'a pas encore Ventoy.".into());
    }
    d.mountpoint.ok_or_else(|| "La partition Ventoy n'a pas de lettre de lecteur. Rebranche la clé.".into())
}

/// Installe Ventoy avec l'outil officiel (demande l'autorisation administrateur).
pub async fn install_ventoy(device: &str, exe: PathBuf) -> Result<(), String> {
    let n = disk_number(device)?;
    if !list_drives().await?.iter().any(|d| d.path == device) {
        return Err("Ce périphérique n'est pas une clé USB détectée.".into());
    }
    let exe = exe.display().to_string().replace('\'', "''");
    let script = format!(
        "$p = Start-Process -FilePath '{exe}' -ArgumentList 'VTOYCLI','/I','/PhyDrive:{n}' -Verb RunAs -Wait -PassThru; exit $p.ExitCode"
    );
    ps(&script).await.map(|_| ()).map_err(|e| {
        if e.is_empty() { "Échec de l'installation de Ventoy (autorisation refusée ?)".into() } else { format!("Échec de l'installation de Ventoy : {e}") }
    })
}

pub async fn eject(device: &str) -> Result<(), String> {
    let d = list_drives().await?.into_iter().find(|d| d.path == device).ok_or("Clé introuvable")?;
    let Some(m) = d.mountpoint else { return Ok(()) };
    let letter = m.trim_end_matches('\\');
    let script = format!(
        "(New-Object -ComObject Shell.Application).Namespace(17).ParseName('{letter}').InvokeVerb('Eject')"
    );
    ps(&script).await.map(|_| ())
}
