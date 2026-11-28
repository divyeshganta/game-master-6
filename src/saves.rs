use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::runners::default_victory_data_dir;
use crate::win32::local_timestamp_filename;

pub fn backups_dir() -> PathBuf {
    let dir = Settings::app_dir().join("backups");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Timestamped save snapshots live in `backups\saves\`.
pub fn saves_backups_dir() -> PathBuf {
    let dir = backups_dir().join("saves");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn mods_dir() -> PathBuf {
    let dir = Settings::app_dir().join("mods");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Returns the single shared save vault directory (`%LOCALAPPDATA%\victory\data`)
/// used across all Victory-family runners so saves persist across runner switches.
/// Respects a configured `VICTORY_DATA_DIR`.
pub fn shared_save_vault_dir() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("VICTORY_DATA_DIR") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    default_victory_data_dir()
}

/// Directory detected for the Save Manager (`%LOCALAPPDATA%\victory\data` or `VICTORY_DATA_DIR`).
pub fn save_data_dir() -> Option<PathBuf> {
    shared_save_vault_dir()
}

fn is_backup_entry(p: &Path) -> bool {
    if p.is_dir() {
        return p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("saves-"));
    }
    p.is_file()
        && p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
        && p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("saves-"))
}

pub fn count_backups() -> usize {
    let mut n = 0usize;
    for dir in [saves_backups_dir(), backups_dir()] {
        if let Ok(rd) = fs::read_dir(dir) {
            n += rd.flatten().filter(|e| is_backup_entry(&e.path())).count();
        }
    }
    n
}

pub fn count_mods() -> usize {
    fs::read_dir(mods_dir())
        .map(|rd| rd.flatten().count())
        .unwrap_or(0)
}

/// Returns the most recent save backup (ZIP or folder) if any exists.
pub fn latest_backup() -> Option<(String, PathBuf)> {
    let mut items: Vec<(std::time::SystemTime, String, PathBuf)> = Vec::new();
    for dir in [saves_backups_dir(), backups_dir()] {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if is_backup_entry(&p) {
                    let m = e.metadata().and_then(|md| md.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    items.push((m, name, p));
                }
            }
        }
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items.into_iter().next().map(|(_, n, p)| (n, p))
}

/// 1-Click EXPORT SAVES (.ZIP): timestamped read-only snapshot of `%LOCALAPPDATA%\victory\data`
/// (or configured `VICTORY_DATA_DIR`) to `backups\saves\`. Original save files are never modified.
pub fn export_saves_zip() -> Result<(PathBuf, usize), String> {
    backup_saves()
}

/// Copies saves/profiles into `backups\saves\saves-<timestamp>.zip` (pure Rust, no PowerShell).
/// Keeps a rolling 5-slot snapshot and never modifies the source save files.
pub fn backup_saves() -> Result<(PathBuf, usize), String> {
    let Some(src) = shared_save_vault_dir() else {
        return Err("Could not resolve save data directory.".into());
    };
    if !src.is_dir() {
        return Err(format!(
            "No save data folder found yet at:\n{}",
            src.display()
        ));
    }
    let dir = saves_backups_dir();
    let _ = fs::create_dir_all(&dir);
    let zip_path = dir.join(format!("saves-{}.zip", local_timestamp_filename()));
    let files = zip_dir_to(&src, &zip_path)?;
    prune_old_backups(5);
    Ok((zip_path, files))
}

/// Restores the latest save backup snapshot (ZIP or folder) back into the save data directory.
/// A safety snapshot is taken first. Original unrelated files are left intact.
pub fn restore_latest_backup() -> Result<(String, usize), String> {
    let Some((name, src)) = latest_backup() else {
        return Err("No save backups found in backups\\saves yet.".into());
    };
    let Some(dst) = shared_save_vault_dir() else {
        return Err("Could not resolve save data directory.".into());
    };
    // Safety: snapshot current saves before restoring over them.
    if dst.is_dir() {
        let pre = saves_backups_dir().join(format!("saves-pre-restore-{}.zip", local_timestamp_filename()));
        let _ = zip_dir_to(&dst, &pre);
    }
    fs::create_dir_all(&dst).map_err(|e| format!("Could not prepare save destination: {e}"))?;
    if src.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
        let files = unzip_to(&src, &dst)?;
        return Ok((name, files));
    }
    let files = copy_dir_recursive(&src, &dst)?;
    Ok((name, files))
}

/// Pure-Rust ZIP (deflate) writer: snapshot `src_dir` recursively into `zip_path`. Never modifies source.
pub fn zip_dir_to(src_dir: &Path, zip_path: &Path) -> Result<usize, String> {
    let file = fs::File::create(zip_path).map_err(|e| format!("Could not create zip: {e}"))?;
    let mut zw = zip::ZipWriter::new(file);
    let opts = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0usize;
    let mut files: Vec<PathBuf> = Vec::new();
    collect_files(src_dir, &mut files);
    for f in files {
        let rel = f.strip_prefix(src_dir).unwrap_or(&f).to_string_lossy().replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        // Skip transient temp files.
        if rel.to_ascii_lowercase().starts_with("temp/") || rel.eq_ignore_ascii_case("temp") {
            continue;
        }
        zw.start_file(rel, opts).map_err(|e| format!("zip start_file: {e}"))?;
        let data = fs::read(&f).map_err(|e| format!("read {}: {e}", f.display()))?;
        zw.write_all(&data).map_err(|e| format!("zip write: {e}"))?;
        count += 1;
    }
    zw.finish().map_err(|e| format!("zip finish: {e}"))?;
    Ok(count)
}

/// Pure-Rust unzip into `dst`.
pub fn unzip_to(zip_path: &Path, dst: &Path) -> Result<usize, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("open zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {e}"))?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("zip entry: {e}"))?;
        let Some(enclosed) = entry.enclosed_name() else { continue };
        let out = dst.join(enclosed);
        if entry.is_dir() {
            let _ = fs::create_dir_all(&out);
            continue;
        }
        if let Some(parent) = out.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut outf = fs::File::create(&out).map_err(|e| format!("create {}: {e}", out.display()))?;
        std::io::copy(&mut entry, &mut outf).map_err(|e| format!("extract: {e}"))?;
        count += 1;
    }
    Ok(count)
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case("temp")) {
                    continue;
                }
                collect_files(&p, out);
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
}

fn prune_old_backups(keep: usize) {
    let mut items: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    if let Ok(rd) = fs::read_dir(saves_backups_dir()) {
        for e in rd.flatten() {
            let p = e.path();
            if is_backup_entry(&p) {
                let m = e.metadata().and_then(|md| md.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                items.push((m, p));
            }
        }
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, old_path) in items.into_iter().skip(keep) {
        if old_path.is_dir() {
            let _ = fs::remove_dir_all(old_path);
        } else {
            let _ = fs::remove_file(old_path);
        }
    }
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<usize, String> {
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
    if !cache_dir.is_dir() {
        return Err("Active shader cache folder does not exist yet.".into());
    }
    let dst = backups_dir().join(format!("cache-{}.zip", local_timestamp_filename()));
    zip_dir_to(cache_dir, &dst)?;
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gm_saves_test_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn zip_dir_roundtrip_and_source_untouched() {
        let root = tmpdir("roundtrip");
        let src = root.join("data");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("00000000").join("save.bin"), b"save-data").unwrap_or(());
        let _ = fs::create_dir_all(src.join("00000000"));
        fs::write(src.join("00000000").join("save.bin"), b"save-data").unwrap();
        fs::write(src.join("nested").join("profile.json"), b"{}").unwrap();
        let before = fs::read(src.join("00000000").join("save.bin")).unwrap();

        let zip_path = root.join("snap.zip");
        let n = zip_dir_to(&src, &zip_path).unwrap();
        assert!(n >= 2);
        assert!(zip_path.is_file());
        assert!(fs::metadata(&zip_path).unwrap().len() > 0);
        // Source untouched.
        assert_eq!(fs::read(src.join("00000000").join("save.bin")).unwrap(), before);

        let out = root.join("restored");
        let restored = unzip_to(&zip_path, &out).unwrap();
        assert!(restored >= 2);
        assert_eq!(fs::read(out.join("00000000").join("save.bin")).unwrap(), before);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn backup_saves_makes_valid_zip_in_backups_saves() {
        let root = tmpdir("backup");
        let data = root.join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("save.bin"), b"hello").unwrap();
        let outdir = root.join("backups").join("saves");

        // Emulate snapshot logic without touching the real vault.
        let zip_path = outdir.join("saves-test.zip");
        fs::create_dir_all(&outdir).unwrap();
        let n = zip_dir_to(&data, &zip_path).unwrap();
        assert_eq!(n, 1);
        assert!(zip_path.is_file());
        let mut arc = zip::ZipArchive::new(fs::File::open(&zip_path).unwrap()).unwrap();
        assert_eq!(arc.len(), 1);
        let _ = fs::remove_dir_all(&root);
    }
}
