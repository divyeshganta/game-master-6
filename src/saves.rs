use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::runners::default_victory_data_dir;
use crate::win32::local_timestamp_filename;

pub fn backups_dir() -> PathBuf {
    let dir = Settings::app_dir().join("backups");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn mods_dir() -> PathBuf {
    let dir = Settings::app_dir().join("mods");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn count_backups() -> usize {
    fs::read_dir(backups_dir())
        .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}

pub fn count_mods() -> usize {
    fs::read_dir(mods_dir())
        .map(|rd| rd.flatten().count())
        .unwrap_or(0)
}

/// Returns the most recent save backup folder (name, path) if any exists.
pub fn latest_backup() -> Option<(String, PathBuf)> {
    let mut dirs: Vec<(std::time::SystemTime, String, PathBuf)> = Vec::new();
    if let Ok(rd) = fs::read_dir(backups_dir()) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let m = e.metadata().and_then(|md| md.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
                dirs.push((m, name, p));
            }
        }
    }
    dirs.sort_by(|a, b| b.0.cmp(&a.0));
    dirs.into_iter().next().map(|(_, n, p)| (n, p))
}

/// Copies VIctory's `%LOCALAPPDATA%\victory\data` saves/profiles into `backups\saves-<timestamp>\`
/// Creates a rolling 5-slot ZIP backup of `%LOCALAPPDATA%\victory\data` in `backups\`
/// and prunes older ZIPs beyond 5 slots.
pub fn backup_saves() -> Result<(PathBuf, usize), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let Some(src) = default_victory_data_dir() else {
        return Err("LOCALAPPDATA environment variable is unavailable.".into());
    };
    if !src.is_dir() {
        return Err(format!(
            "No VIctory save data folder found yet at:\n{}",
            src.display()
        ));
    }
    let dir = backups_dir();
    let _ = fs::create_dir_all(&dir);
    let zip_path = dir.join(format!("saves-{}.zip", local_timestamp_filename()));
    let src_esc = src.to_string_lossy().replace('\'', "''");
    let dst_esc = zip_path.to_string_lossy().replace('\'', "''");
    let ps = format!("Compress-Archive -Path '{src_esc}\\*' -DestinationPath '{dst_esc}' -Force");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to create save ZIP backup: {e}"))?;
    if !out.status.success() {
        // Fallback to folder copy if Compress-Archive fails (e.g. empty dir)
        let dest = dir.join(format!("saves-{}", local_timestamp_filename()));
        fs::create_dir_all(&dest).map_err(|e| format!("Could not create backup folder: {e}"))?;
        let files = copy_dir_recursive(&src, &dest)?;
        prune_old_backups(5);
        return Ok((dest, files));
    }
    prune_old_backups(5);
    Ok((zip_path, 1))
}

/// Restores the latest save backup snapshot (ZIP or folder) back into `%LOCALAPPDATA%\victory\data`.
pub fn restore_latest_backup() -> Result<(String, usize), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let Some((name, src)) = latest_backup() else {
        return Err("No save backups found in backups\\ yet.".into());
    };
    let Some(dst) = default_victory_data_dir() else {
        return Err("LOCALAPPDATA environment variable is unavailable.".into());
    };
    fs::create_dir_all(&dst).map_err(|e| format!("Could not prepare save destination: {e}"))?;
    if src.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
        let src_esc = src.to_string_lossy().replace('\'', "''");
        let dst_esc = dst.to_string_lossy().replace('\'', "''");
        let ps = format!("Expand-Archive -Path '{src_esc}' -DestinationPath '{dst_esc}' -Force");
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to extract save ZIP: {e}"))?;
        if !out.status.success() {
            return Err(format!("Expand-Archive failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
        return Ok((name, 1));
    }
    let files = copy_dir_recursive(&src, &dst)?;
    Ok((name, files))
}

fn prune_old_backups(keep: usize) {
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    if let Ok(rd) = fs::read_dir(backups_dir()) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let m = e.metadata().and_then(|md| md.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                dirs.push((m, p));
            }
        }
    }
    dirs.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, old_dir) in dirs.into_iter().skip(keep) {
        let _ = fs::remove_dir_all(old_dir);
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<usize, String> {
    let mut copied = 0usize;
    let rd = fs::read_dir(src).map_err(|e| format!("Cannot read {}: {e}", src.display()))?;
    for entry in rd.flatten() {
        let p = entry.path();
        let target = dst.join(entry.file_name());
        if p.is_dir() {
            if entry.file_name().to_string_lossy().eq_ignore_ascii_case("temp") {
                continue;
            }
            let _ = fs::create_dir_all(&target);
            copied += copy_dir_recursive(&p, &target)?;
        } else if p.is_file() {
            if fs::copy(&p, &target).is_ok() {
                copied += 1;
            }
        }
    }
    Ok(copied)
}

#[derive(Clone, Debug)]
pub struct ModItem {
    pub name: String,
    pub enabled: bool,
    pub path: PathBuf,
}

pub fn list_mods(limit: usize) -> Vec<ModItem> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(mods_dir()) {
        for e in rd.flatten() {
            let p = e.path();
            let raw = e.file_name().to_string_lossy().into_owned();
            let (name, enabled) = if let Some(stripped) = raw.strip_suffix(".disabled") {
                (stripped.to_string(), false)
            } else {
                (raw, true)
            };
            out.push(ModItem { name, enabled, path: p });
        }
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out.truncate(limit);
    out
}

pub fn toggle_mod(path: &Path) -> Result<bool, String> {
    let fname = path.file_name().and_then(|s| s.to_str()).ok_or("Invalid mod filename")?;
    let parent = path.parent().ok_or("Invalid mod folder")?;
    let (new_name, now_enabled) = if let Some(stripped) = fname.strip_suffix(".disabled") {
        (stripped.to_string(), true)
    } else {
        (format!("{fname}.disabled"), false)
    };
    let dst = parent.join(new_name);
    fs::rename(path, &dst).map_err(|e| format!("Could not toggle mod: {e}"))?;
    Ok(now_enabled)
}

pub fn export_cache_zip(cache_dir: &Path) -> Result<PathBuf, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    if !cache_dir.is_dir() {
        return Err("Active shader cache folder does not exist yet.".into());
    }
    let dst = backups_dir().join(format!("cache-{}.zip", local_timestamp_filename()));
    let src_s = cache_dir.to_string_lossy().replace('\'', "''");
    let dst_s = dst.to_string_lossy().replace('\'', "''");
    let ps = format!(r"Compress-Archive -Path '{src_s}\*' -DestinationPath '{dst_s}' -Force");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to compress cache: {e}"))?;
    if out.status.success() {
        Ok(dst)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}
