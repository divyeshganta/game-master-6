//! VIper mod-framework bridge (v1.7.0).
//!
//! VIper is a STATIC patcher, not a DLL injector: `viper.exe patch <game.exe>` writes
//! `<game>.viper` next to the game with every `mods/*.dll` linked into a new PE section
//! and a hook on the title's cheat-input dispatcher. The original executable is never
//! touched. Game Master runs that patched copy through the verified VIctory-NVIDIA
//! `victory.exe`, so DLSS + 2x Frame Gen stay intact (Mode A bridge).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use crate::config::Settings;

pub const VIPER_REPO: &str = "viper-community/viper";
pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/viper-community/viper/releases/latest";
pub const DEFAULT_VERSION: &str = "0.2.6";

/// Verified from VIper's `mods/examples/trainer/viper.toml` and README.
pub const TRAINER_HOTKEYS: &str = "F4 Trainer · F5 Vehicle · F6 Weapons · F7 Clear Wanted · F8 Never Wanted · F9 God Mode · F10 Companion · F11 Prop · F12 Inspect · Shift+F12 WAIL";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag: String,
    pub version: String,
    pub zip_name: String,
    pub zip_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModEntry {
    pub path: PathBuf,
    pub name: String,
    pub enabled: bool,
}

pub fn viper_root() -> PathBuf {
    Settings::app_dir().join("runners").join("viper")
}

pub fn viper_exe() -> PathBuf {
    viper_root().join("viper.exe")
}

pub fn backup_dir() -> PathBuf {
    viper_root().join("backup")
}

pub fn version_file() -> PathBuf {
    viper_root().join("version.txt")
}

pub fn installed_version() -> String {
    std::fs::read_to_string(version_file())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub fn is_installed() -> bool {
    viper_exe().is_file()
}

/// VIper keeps mods next to the game executable, in `<game_dir>/mods`.
pub fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("mods")
}

/// The modded copy VIper writes beside the game: `<stem>.viper`.
pub fn patched_exe(game_dir: &Path, game_exe: &Path) -> PathBuf {
    let stem = game_exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game".into());
    game_dir.join(format!("{stem}.viper"))
}

/// Strip a UTF-8 BOM so `VICTORY_MENU_CONSOLE` is never parsed twice / duplicated.
pub fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    }
}

pub fn parse_victory_settings_bytes(bytes: &[u8]) -> Vec<(String, String)> {
    let clean = strip_utf8_bom(bytes);
    let text = String::from_utf8_lossy(clean);
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim().trim_start_matches('\u{FEFF}');
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let key = k.trim().to_string();
            if !key.is_empty() {
                out.push((key, v.trim().to_string()));
            }
        }
    }
    out
}

/// Scan `<game_dir>/mods` for active `.dll` and disabled `.dll.disabled` mods.
pub fn scan_mods(game_dir: &Path) -> Vec<ModEntry> {
    let dir = mods_dir(game_dir);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        let (enabled, stem) = if lower.ends_with(".dll.disabled") {
            (false, name[..name.len() - ".disabled".len()].to_string())
        } else if lower.ends_with(".dll") && !lower.starts_with('.') {
            (true, name.clone())
        } else {
            continue;
        };
        out.push(ModEntry {
            path,
            name: stem,
            enabled,
        });
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out
}

pub fn active_mod_count(game_dir: &Path) -> usize {
    scan_mods(game_dir).iter().filter(|m| m.enabled).count()
}

/// Toggle a mod: `.dll` -> `.dll.disabled` and back. Never touches other files.
pub fn toggle_mod(entry: &ModEntry) -> Result<PathBuf, String> {
    let target = if entry.enabled {
        let mut s = entry.path.as_os_str().to_string_lossy().into_owned();
        s.push_str(".disabled");
        PathBuf::from(s)
    } else {
        let s = entry.path.as_os_str().to_string_lossy().into_owned();
        PathBuf::from(s.strip_suffix(".disabled").unwrap_or(&s))
    };
    std::fs::rename(&entry.path, &target)
        .map_err(|e| format!("Cannot toggle {}: {e}", entry.path.display()))?;
    Ok(target)
}

pub fn version_tuple(v: &str) -> (u32, u32, u32) {
    let nums: Vec<u32> = v
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split('.')
        .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
        .take(3)
        .collect();
    (
        *nums.get(0).unwrap_or(&0),
        *nums.get(1).unwrap_or(&0),
        *nums.get(2).unwrap_or(&0),
    )
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    version_tuple(candidate) > version_tuple(current)
}

/// Minimal GitHub `releases/latest` parser (no serde dependency).
pub fn parse_latest_release_json(text: &str) -> Result<ReleaseInfo, String> {
    let tag = json_string_field(text, "tag_name").ok_or("release has no tag_name")?;
    let version = tag.trim_start_matches('v').to_string();
    // Prefer a windows zip asset; fall back to the first .zip asset.
    let mut zip_name = String::new();
    let mut zip_url = String::new();
    let mut cursor = 0usize;
    while let Some(pos) = text[cursor..].find("\"name\"") {
        let i = cursor + pos;
        let Some(name) = json_string_field_at(text, i) else {
            cursor = i + 6;
            continue;
        };
        if name.to_ascii_lowercase().ends_with(".zip") {
            let tail = &text[i..];
            zip_name = name.clone();
            zip_url = json_string_field(tail, "browser_download_url").unwrap_or_default();
            if name.to_ascii_lowercase().contains("windows") {
                break;
            }
        }
        cursor = i + 6;
    }
    if zip_name.is_empty() || zip_url.is_empty() {
        return Err("release has no downloadable zip asset".into());
    }
    Ok(ReleaseInfo {
        tag,
        version,
        zip_name,
        zip_url,
    })
}

fn json_string_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = text.find(&needle)?;
    json_string_field_at(text, i + needle.len())
}

fn json_string_field_at(text: &str, from: usize) -> Option<String> {
    let rest = &text[from..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let quote = after.find('"')?;
    let body = &after[quote + 1..];
    let end = body.find('"')?;
    Some(body[..end].replace("\\/", "/").replace("\\\"", "\""))
}

/// Non-blocking update check: returns a receiver for the parsed latest release.
pub fn check_latest_async() -> Receiver<Result<ReleaseInfo, String>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let result = (|| -> Result<ReleaseInfo, String> {
            let output = Command::new("curl")
                .args([
                    "-sSL",
                    "--max-time",
                    "20",
                    "-H",
                    "User-Agent: GameMaster6",
                    LATEST_RELEASE_API,
                ])
                .output()
                .map_err(|e| format!("curl failed: {e}"))?;
            let text = String::from_utf8_lossy(&output.stdout);
            parse_latest_release_json(&text)
        })();
        let _ = tx.send(result);
    });
    rx
}

/// True when `<game>.viper` is newer than viper.exe and every mods/*.dll + *.toml.
/// Avoids re-running the patcher on every launch (the main cause of intermittent
/// "PLAY does nothing": a patcher process racing the launcher over a locked file).
pub fn patch_is_current(game_exe: &Path) -> bool {
    let game_dir = game_exe.parent().unwrap_or(Path::new("."));
    let out = patched_exe(game_dir, game_exe);
    let Ok(out_meta) = std::fs::metadata(&out) else { return false };
    let Ok(out_time) = out_meta.modified() else { return false };
    if let Ok(m) = std::fs::metadata(viper_exe()) {
        if m.modified().map_or(false, |t| t > out_time) {
            return false;
        }
    }
    let Ok(rd) = std::fs::read_dir(mods_dir(game_dir)) else {
        return true; // no mods folder: the built-in trainer was linked
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue };
        let lower = name.to_ascii_lowercase();
        if lower.starts_with('.') || !(lower.ends_with(".dll") || lower.ends_with(".toml")) {
            continue;
        }
        if let Ok(m) = entry.metadata() {
            if m.modified().map_or(false, |t| t > out_time) {
                return false;
            }
        }
    }
    true
}

/// Kill a lingering VIper patcher before launching so Windows file locks cannot
/// abort the patch. Pre-launch only — never called while a game session is active.
pub fn kill_stray_viper() -> usize {
    match Command::new("taskkill")
        .args(["/F", "/T", "/IM", "viper.exe"])
        .output()
    {
        Ok(o) if o.status.success() => 1,
        _ => 0,
    }
}

/// Write the full patcher failure (stderr + exit code) to logs/ so a failed PLAY is
/// never silent.
pub fn log_launch_failure(message: &str) {
    let dir = crate::diagnostics::logs_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("viper-launch.log");
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{} {}", crate::win32::local_timestamp_human(), message);
    }
}

/// Always re-link mods (used by the RE-PATCH button and when the cache is stale).
pub fn repatch(game_exe: &Path) -> Result<PathBuf, String> {
    let exe = viper_exe();
    if !exe.is_file() {
        let msg = "Install VIper first (runners\\viper\\viper.exe missing)".to_string();
        log_launch_failure(&msg);
        return Err(msg);
    }
    let game_dir = game_exe.parent().unwrap_or(Path::new("."));
    let out = Command::new(&exe)
        .arg("patch")
        .arg(game_exe)
        .current_dir(game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| {
            let msg = format!("Cannot start VIper patcher: {e}");
            log_launch_failure(&msg);
            msg
        })?;
    if !out.status.success() {
        let msg = format!(
            "VIper patch failed (exit {}): {} | stdout: {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).trim(),
            String::from_utf8_lossy(&out.stdout).trim()
        );
        log_launch_failure(&msg);
        return Err(msg);
    }
    let path = patched_exe(game_dir, game_exe);
    if !path.is_file() {
        let msg = format!("VIper patch reported success but {}", path.display());
        log_launch_failure(&msg);
        return Err(msg);
    }
    Ok(path)
}

/// Ask VIper to link all mods into `<game>.viper`, skipping the patcher when the
/// cached copy is still current. Original game files are never changed.
pub fn ensure_patched(game_exe: &Path) -> Result<PathBuf, String> {
    if patch_is_current(game_exe) {
        return Ok(patched_exe(game_exe.parent().unwrap_or(Path::new(".")), game_exe));
    }
    repatch(game_exe)
}

/// Install/update VIper from a downloaded release zip into `runners/viper/`.
/// The previous install is copied to `runners/viper/backup/` for one-click rollback.
pub fn install_from_zip(zip_path: &Path, version: &str) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("bad zip: {e}"))?;
    let root = viper_root();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let backup = backup_dir();
    if root.join("viper.exe").is_file() {
        let _ = std::fs::create_dir_all(&backup);
        let stamp = format!("viper-{}.exe", installed_version().replace('.', "_"));
        let _ = std::fs::copy(root.join("viper.exe"), backup.join(stamp));
    }
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else { continue };
        let file_name = name.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !file_name.eq_ignore_ascii_case("viper.exe")
            && !file_name.eq_ignore_ascii_case("default_emulator.dll")
        {
            continue;
        }
        let target = root.join(file_name);
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    if !root.join("viper.exe").is_file() {
        return Err("release zip did not contain viper.exe".into());
    }
    std::fs::write(version_file(), version).map_err(|e| e.to_string())?;
    Ok(())
}

/// Download a release asset with curl (pre-launch / user action only, never in-game).
pub fn download_release(info: &ReleaseInfo) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("gm-viper-{}", info.version));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&info.zip_name);
    let status = Command::new("curl")
        .args([
            "-sSL",
            "--max-time",
            "180",
            "-o",
            &dest.to_string_lossy(),
            &info.zip_url,
        ])
        .status()
        .map_err(|e| format!("download failed: {e}"))?;
    if !status.success() {
        return Err("download failed".into());
    }
    Ok(dest)
}

pub fn rollback() -> Result<(), String> {
    let backup = backup_dir();
    let rd = std::fs::read_dir(&backup).map_err(|e| e.to_string())?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in rd.flatten() {
        let p = entry.path();
        if p.file_name().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("viper.exe")) {
            if let Ok(t) = entry.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().is_none_or(|(old, _)| t > *old) {
                    newest = Some((t, p));
                }
            }
        }
    }
    let (_, src) = newest.ok_or("no backup found")?;
    std::fs::copy(&src, viper_exe()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_is_stripped_before_settings_parse() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"VICTORY_MENU_CONSOLE=series-x\nVICTORY_OUTPUT=2560x1440\n");
        let pairs = parse_victory_settings_bytes(&bytes);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("VICTORY_MENU_CONSOLE".into(), "series-x".into()));
        assert_eq!(pairs[1], ("VICTORY_OUTPUT".into(), "2560x1440".into()));
        assert_eq!(strip_utf8_bom(&bytes).len(), bytes.len() - 3);
    }

    #[test]
    fn mod_toggle_state_machine() {
        let game = std::env::temp_dir().join(format!("gm_viper_game_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&game);
        let mods = game.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("a.dll"), b"A").unwrap();
        std::fs::write(mods.join("b.dll.disabled"), b"B").unwrap();
        let found = scan_mods(&game);
        assert_eq!(found.len(), 2);
        assert!(found[0].enabled && found[0].name == "a.dll");
        assert!(!found[1].enabled);
        let off = toggle_mod(&found[0]).unwrap();
        assert!(off.to_string_lossy().ends_with("a.dll.disabled"));
        let on = toggle_mod(&ModEntry { path: off, name: "a.dll".into(), enabled: false }).unwrap();
        assert!(on.to_string_lossy().ends_with("a.dll"));
        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn release_json_parser_and_version_compare() {
        let json = r#"{"tag_name":"v0.2.6","assets":[{"name":"update-v1.toml","browser_download_url":"https://x/update"},{"name":"viper-0.2.6-windows.zip","browser_download_url":"https://x/viper.zip"}]}"#;
        let info = parse_latest_release_json(json).unwrap();
        assert_eq!(info.tag, "v0.2.6");
        assert_eq!(info.version, "0.2.6");
        assert_eq!(info.zip_name, "viper-0.2.6-windows.zip");
        assert_eq!(info.zip_url, "https://x/viper.zip");
        assert!(is_newer("0.2.6", "0.2.5"));
        assert!(!is_newer("0.2.5", "0.2.6"));
        assert_eq!(version_tuple("v1.7.0"), (1, 7, 0));
    }

    #[test]
    fn patch_cache_skips_when_current_and_detects_changed_mod() {
        let game = std::env::temp_dir().join(format!("gm_viper_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&game);
        let mods = game.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        let exe = game.join("title.exe");
        std::fs::write(&exe, b"EXE").unwrap();
        let patched = patched_exe(&game, &exe);
        std::fs::write(&patched, b"VIPER").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(patch_is_current(&exe), "fresh patched copy should skip re-patching");
        std::fs::write(mods.join("mod.dll"), b"DLL").unwrap();
        assert!(!patch_is_current(&exe), "newer mod must force a re-patch");
        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn patched_copy_name_and_hotkeys() {
        let dir = Path::new(r"C:\games\title");
        assert_eq!(patched_exe(dir, &dir.join("title.exe")), dir.join("title.viper"));
        assert!(TRAINER_HOTKEYS.contains("F4 Trainer"));
        assert!(TRAINER_HOTKEYS.contains("Shift+F12"));
    }
}
