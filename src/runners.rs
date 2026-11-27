use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::diagnostics::{folder_stats, format_bytes};
use crate::win32::hardlink_or_copy;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunnerKind {
    Baseline,
    LocalZtrx,
    VictoryFork,
    Victory,
    VictoryDiag,
    Xcl,
    Xbox2Windows,
    Generic,
}

impl RunnerKind {
    pub fn id(self) -> &'static str {
        match self {
            RunnerKind::Baseline => "baseline",
            RunnerKind::LocalZtrx => "local-ztrx",
            RunnerKind::VictoryFork => "victory-fork",
            RunnerKind::Victory => "victory",
            RunnerKind::VictoryDiag => "victory-diag",
            RunnerKind::Xcl => "xcl",
            RunnerKind::Xbox2Windows => "xbox2windows",
            RunnerKind::Generic => "generic",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            RunnerKind::Baseline => "BASELINE · BASELINE",
            RunnerKind::LocalZtrx => "LOCAL ZTRX",
            RunnerKind::VictoryFork => "VICTORY-FORK · RT",
            RunnerKind::Victory => "VICTORY UPSTREAM",
            RunnerKind::VictoryDiag => "VICTORY DIAG",
            RunnerKind::Xcl => "XCL LAYER",
            RunnerKind::Xbox2Windows => "XBOX2WINDOWS",
            RunnerKind::Generic => "CUSTOM RUNNER",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "baseline" | "cool_fox" | "cool-fox" => Some(RunnerKind::Baseline),
            "local-ztrx" | "local_ztrx" | "ztrx" | "zettrox" => Some(RunnerKind::LocalZtrx),
            "victory-fork" | "victory_fork" | "hyphennn" => Some(RunnerKind::VictoryFork),
            "victory" => Some(RunnerKind::Victory),
            "victory-diag" | "victory_diag" | "diag" => Some(RunnerKind::VictoryDiag),
            "xcl" => Some(RunnerKind::Xcl),
            "xbox2windows" | "x2w" => Some(RunnerKind::Xbox2Windows),
            "generic" | "custom" => Some(RunnerKind::Generic),
            _ => None,
        }
    }

    pub fn is_victory_family(self) -> bool {
        matches!(
            self,
            RunnerKind::Baseline | RunnerKind::LocalZtrx | RunnerKind::VictoryFork | RunnerKind::Victory | RunnerKind::VictoryDiag
        )
    }

    pub fn supports_prepare(self) -> bool {
        self.is_victory_family()
    }

    pub fn detect_from_path(path: &Path) -> Self {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if lower.contains("baseline") || lower.contains("cool_fox") || lower.contains("overnight-tuned") {
            RunnerKind::Baseline
        } else if lower.contains("diag") {
            RunnerKind::VictoryDiag
        } else if lower.contains("ztrx") || lower.contains("zettrox") {
            RunnerKind::LocalZtrx
        } else if lower.contains("victory-fork")
            || lower.contains("victory_fork")
            || lower.contains("hyphen")
            || lower.contains("fxceless")
            || lower.contains("uilian")
        {
            RunnerKind::VictoryFork
        } else if lower.contains("xcl") {
            RunnerKind::Xcl
        } else if lower.contains("xbox2windows") || lower.contains("x2w") {
            RunnerKind::Xbox2Windows
        } else if lower.contains("victory") || lower.contains("gamerunner") {
            RunnerKind::Victory
        } else {
            RunnerKind::Generic
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            RunnerKind::Baseline => RunnerKind::VictoryFork,
            RunnerKind::LocalZtrx => RunnerKind::VictoryFork,
            RunnerKind::VictoryFork => RunnerKind::Victory,
            RunnerKind::Victory => RunnerKind::VictoryDiag,
            RunnerKind::VictoryDiag => RunnerKind::Xcl,
            RunnerKind::Xcl => RunnerKind::Xbox2Windows,
            RunnerKind::Xbox2Windows => RunnerKind::Generic,
            RunnerKind::Generic => RunnerKind::Baseline,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunnerMeta {
    pub repo: String,
    pub commit: String,
    pub built_at: String,
}

#[derive(Clone, Debug)]
pub struct DiscoveredRunner {
    pub name: String,
    pub path: PathBuf,
    pub kind: RunnerKind,
    pub size_label: String,
    pub cache_label: String,
    pub has_backup: bool,
    pub repo: String,
    pub commit: String,
    pub built_at: String,
}

pub fn import_runner_exe(src: &Path) -> Result<(String, PathBuf), String> {
    if !src.is_file() {
        return Err(format!("Selected runner does not exist: {}", src.display()));
    }
    let parent_name = src
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "custom".into());
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "runner".into());
    let mut folder = if stem.eq_ignore_ascii_case("victory") || stem.eq_ignore_ascii_case("gamerunner") {
        parent_name
    } else {
        stem
    };
    let clean: String = folder
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    folder = clean.trim_matches('-').to_string();
    if folder.is_empty() || folder.eq_ignore_ascii_case("ztrx") {
        folder = format!(
            "imported-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() % 100_000)
                .unwrap_or(1)
        );
    }
    let dir = runners_dir().join(&folder);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    let dst = dir.join("victory.exe");
    fs::copy(src, &dst).map_err(|e| format!("Failed to copy runner: {e}"))?;
    Ok((folder, dst))
}

pub fn runners_dir() -> PathBuf {
    let dir = Settings::app_dir().join("runners");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn caches_root() -> PathBuf {
    let dir = Settings::app_dir().join("caches");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn read_runner_meta(exe_path: &Path) -> RunnerMeta {
    let mut meta = RunnerMeta::default();
    let Some(parent) = exe_path.parent() else {
        return meta;
    };
    let json_path = parent.join("runner-meta.json");
    if let Ok(jtxt) = fs::read_to_string(&json_path) {
        if let Some(r) = extract_json_str(&jtxt, "repo") {
            meta.repo = r;
        }
        if let Some(c) = extract_json_str(&jtxt, "commit") {
            meta.commit = c;
        }
        if let Some(b) = extract_json_str(&jtxt, "built_at") {
            meta.built_at = b;
        }
        if !meta.repo.is_empty() || !meta.commit.is_empty() {
            return meta;
        }
    }
    let meta_path = parent.join(".gm-meta");
    let Ok(text) = fs::read_to_string(&meta_path) else {
        return meta;
    };
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "repo" => meta.repo = v.trim().to_string(),
                "commit" => meta.commit = v.trim().to_string(),
                "built_at" => meta.built_at = v.trim().to_string(),
                _ => {}
            }
        }
    }
    meta
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut esc = false;
    for ch in after[1..].chars() {
        if esc {
            out.push(ch);
            esc = false;
        } else if ch == '\\' {
            esc = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if after.starts_with("true") {
        Some(true)
    } else if after.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

pub fn runner_slug(path: &Path, kind: RunnerKind) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("runner")
        .to_ascii_lowercase();
    let parent_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let raw = if stem == "victory" || stem == "gamerunner" {
        if !parent_name.is_empty() && parent_name != "runners" && !parent_name.contains("game master") {
            parent_name
        } else {
            kind.id().to_string()
        }
    } else {
        stem
    };

    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

pub fn cache_dir_for(path: &Path, kind: RunnerKind, isolated: bool) -> PathBuf {
    if isolated {
        let dir = caches_root().join(runner_slug(path, kind));
        let _ = fs::create_dir_all(&dir);
        dir
    } else {
        default_victory_cache_dir()
    }
}

pub fn default_victory_cache_dir() -> PathBuf {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(local).join("victory").join("cache")
    } else {
        Settings::app_dir().join("caches").join("default")
    }
}

pub fn default_victory_data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("victory").join("data"))
}

pub fn quick_cache_summary(cache_dir: &Path) -> (bool, String) {
    let shaders = cache_dir.join("shaders");
    let pipelines = cache_dir.join("pipelines");
    let (sb, sc) = if shaders.is_dir() { folder_stats(&shaders) } else { (0, 0) };
    let (pb, pc) = if pipelines.is_dir() { folder_stats(&pipelines) } else { (0, 0) };
    let total_b = sb + pb;
    let total_c = sc + pc;
    if total_c > 0 && total_b > 0 {
        (true, format!("{} ({total_c} files)", format_bytes(total_b)))
    } else {
        (false, "Not built yet".into())
    }
}

/// Returns true for real runner binaries, filtering out Rust/CMake build helper tools like `xtask.exe`.
pub fn is_junk_helper_exe(p: &Path) -> bool {
    let fname = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    fname == "xtask.exe"
        || fname.starts_with("build-script")
        || fname == "rustc.exe"
        || fname == "cargo.exe"
        || fname == "clippy-driver.exe"
}

fn is_exe(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

pub fn discover_runners(s: &Settings) -> Vec<DiscoveredRunner> {
    let mut found: Vec<DiscoveredRunner> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    let mut push_candidate = |p: PathBuf, forced_kind: Option<RunnerKind>| {
        if !p.is_file() || is_junk_helper_exe(&p) {
            return;
        }
        let key = p.to_string_lossy().to_ascii_lowercase();
        if seen.iter().any(|k| k == &key) {
            return;
        }
        seen.push(key);
        let kind = forced_kind.unwrap_or_else(|| RunnerKind::detect_from_path(&p));
        let size = fs::metadata(&p).map(|m| format_bytes(m.len())).unwrap_or_else(|_| "?".into());
        let name = display_name(&p);
        let cdir = cache_dir_for(&p, kind, s.isolated_cache);
        let (_, cache_label) = if kind.is_victory_family() {
            quick_cache_summary(&cdir)
        } else {
            (false, "Managed by runner".into())
        };
        let has_backup = backup_path_for(&p).is_file();
        let meta = read_runner_meta(&p);
        found.push(DiscoveredRunner {
            name,
            path: p,
            kind,
            size_label: size,
            cache_label,
            has_backup,
            repo: meta.repo,
            commit: meta.commit,
            built_at: meta.built_at,
        });
    };

    // 1. Currently configured runner always first
    let active = s.resolved_runner();
    let active_kind = RunnerKind::from_id(&s.runner_kind);
    push_candidate(active, active_kind);

    // 2. App root known runner binaries
    let app = Settings::app_dir();
    for fname in [
        "GameRunner.exe",
        "victory-fork.exe",
        "victory.exe",
        "victory-diag.exe",
        "xcl.exe",
        "xbox2windows.exe",
        "tuxbox.exe",
        "console-runner.exe",
        "open-console.exe",
    ] {
        push_candidate(app.join(fname), None);
    }

    // 3. runners\ folder (direct .exe files and 1-level subfolders like runners\ztrx\victory.exe)
    let rdir = runners_dir();
    if let Ok(rd) = fs::read_dir(&rdir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() && is_exe(&p) {
                push_candidate(p, None);
            } else if p.is_dir() {
                if let Ok(sub) = fs::read_dir(&p) {
                    for se in sub.flatten() {
                        let sp = se.path();
                        if sp.is_file() && is_exe(&sp) {
                            push_candidate(sp, None);
                        }
                    }
                }
            }
        }
    }

    found
}

/// Removes leftover `xtask.exe` / build-helper binaries from `runners\` and their `prev\` backups.
pub fn clean_junk_binaries() -> usize {
    let rdir = runners_dir();
    let mut removed = 0usize;
    let mut scan_dir = |dir: &Path| {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() && is_junk_helper_exe(&p) {
                    let bak = backup_path_for(&p);
                    if bak.is_file() {
                        let _ = fs::remove_file(&bak);
                    }
                    if fs::remove_file(&p).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
    };
    scan_dir(&rdir);
    if let Ok(rd) = fs::read_dir(&rdir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                scan_dir(&p);
            }
        }
    }
    removed
}

fn display_name(p: &Path) -> String {
    let fname = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("runner.exe");
    if let Some(parent) = p.parent().and_then(|d| d.file_name()).and_then(|s| s.to_str()) {
        let pl = parent.to_ascii_lowercase();
        if pl != "runners"
            && !pl.contains("game master")
            && (fname.eq_ignore_ascii_case("victory.exe")
                || fname.eq_ignore_ascii_case("victory-diag.exe")
                || fname.eq_ignore_ascii_case("xcl.exe"))
        {
            return format!("{parent} ({fname})");
        }
    }
    fname.to_string()
}

pub fn backup_path_for(exe_path: &Path) -> PathBuf {
    let fname = exe_path.file_name().and_then(|s| s.to_str()).unwrap_or("runner.exe");
    exe_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("prev")
        .join(format!("{fname}.bak"))
}

pub fn rollback_runner(exe_path: &Path) -> Result<(), String> {
    let bak = backup_path_for(exe_path);
    let side_bak = exe_path.with_extension("exe.bak");
    let src = if bak.is_file() {
        bak
    } else if side_bak.is_file() {
        side_bak
    } else {
        return Err(format!("No backup found at {} or {}", bak.display(), side_bak.display()));
    };
    fs::copy(&src, exe_path).map_err(|e| format!("Could not restore backup: {e}"))?;
    Ok(())
}

/// Safely deletes a runner inside `runners\`.
/// If other valid runner binaries exist in the same subfolder, deletes only that binary (and its backup).
/// Otherwise deletes the entire runner subfolder cleanly.
pub fn delete_runner(exe_path: &Path) -> Result<(), String> {
    let root = fs::canonicalize(runners_dir()).map_err(|e| format!("Cannot resolve runners folder: {e}"))?;
    let target = fs::canonicalize(exe_path).map_err(|e| format!("Cannot resolve runner path: {e}"))?;
    let name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("runner.exe");
    if name.eq_ignore_ascii_case("gamerunner.exe") {
        return Err("Refusing to delete the root GameRunner.exe".into());
    }
    if !target.starts_with(&root) {
        return Err(format!(
            "Refusing to delete a runner outside {}: {}",
            root.display(),
            target.display()
        ));
    }
    let parent = target
        .parent()
        .ok_or_else(|| "Runner has no parent folder".to_string())?;
    if parent == root {
        let bak = backup_path_for(&target);
        if bak.is_file() {
            let _ = fs::remove_file(&bak);
        }
        fs::remove_file(&target).map_err(|e| format!("Could not delete runner: {e}"))?;
    } else if parent.starts_with(&root) {
        // Count sibling non-junk runner .exes in the same subfolder
        let sibling_count = fs::read_dir(parent)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| {
                        let p = e.path();
                        p.is_file() && is_exe(&p) && !is_junk_helper_exe(&p)
                    })
                    .count()
            })
            .unwrap_or(1);
        if sibling_count > 1 {
            let bak = backup_path_for(&target);
            if bak.is_file() {
                let _ = fs::remove_file(&bak);
            }
            fs::remove_file(&target).map_err(|e| format!("Could not delete runner: {e}"))?;
        } else {
            fs::remove_dir_all(parent).map_err(|e| format!("Could not delete runner folder: {e}"))?;
        }
    } else {
        return Err(format!("Refusing to delete path outside runners folder: {}", target.display()));
    }
    Ok(())
}

/// Hardlinks (0 extra bytes on same drive) or copies the global shader/pipeline cache into an isolated runner cache.
pub fn seed_isolated_cache_from_global(dst_cache: &Path) -> Result<usize, String> {
    let src_cache = default_victory_cache_dir();
    if !src_cache.is_dir() {
        return Err(format!("Global cache not found at {}", src_cache.display()));
    }
    if src_cache == dst_cache {
        return Err("Active cache is already using the global cache.".into());
    }
    let mut linked = 0usize;
    for sub in ["shaders", "pipelines"] {
        let s = src_cache.join(sub);
        let d = dst_cache.join(sub);
        if s.is_dir() {
            linked += link_tree(&s, &d)?;
        }
    }
    Ok(linked)
}

fn link_tree(src: &Path, dst: &Path) -> Result<usize, String> {
    fs::create_dir_all(dst).map_err(|e| format!("Could not create {}: {e}", dst.display()))?;
    let mut count = 0usize;
    let rd = fs::read_dir(src).map_err(|e| format!("Cannot read {}: {e}", src.display()))?;
    for entry in rd.flatten() {
        let sp = entry.path();
        let dp = dst.join(entry.file_name());
        if sp.is_dir() {
            count += link_tree(&sp, &dp)?;
        } else if sp.is_file() && !dp.exists() {
            if hardlink_or_copy(&sp, &dp).is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Computes lowercase SHA-256 hex digest of a file using PowerShell `Get-FileHash` (no console flash).
pub fn sha256_file_hex(path: &Path) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    if !path.is_file() {
        return None;
    }
    let esc = path.to_string_lossy().replace('\'', "''");
    let ps = format!("(Get-FileHash -LiteralPath '{esc}' -Algorithm SHA256).Hash");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let hash = String::from_utf8_lossy(&out.stdout).trim().to_ascii_lowercase();
    if hash.len() == 64 { Some(hash) } else { None }
}

/// Checks `.runner_hash` inside `cache_dir` against the active runner binary's SHA-256.
/// Returns `Some((expected, actual))` if a mismatch is detected.
pub fn check_cache_hash_mismatch(runner_path: &Path, cache_dir: &Path) -> Option<(String, String)> {
    let marker = cache_dir.join(".runner_hash");
    let recorded = fs::read_to_string(&marker).ok()?.trim().to_ascii_lowercase();
    if recorded.len() != 64 {
        return None;
    }
    let actual = sha256_file_hex(runner_path)?;
    if recorded != actual {
        Some((recorded, actual))
    } else {
        None
    }
}

/// Records the active runner binary's SHA-256 into `cache_dir/.runner_hash` after shader prep.
pub fn write_cache_hash(runner_path: &Path, cache_dir: &Path) {
    if let Some(hash) = sha256_file_hex(runner_path) {
        let _ = fs::create_dir_all(cache_dir);
        let _ = fs::write(cache_dir.join(".runner_hash"), hash);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerProfile {
    pub name: String,
    pub kind: String,
    pub extra_args: String,
    pub env: Vec<(String, String)>,
    pub supports_shaders: Option<bool>,
    pub supports_warm: Option<bool>,
}

pub fn load_runner_profile(exe_path: &Path) -> Option<RunnerProfile> {
    let parent = exe_path.parent()?;
    let path = parent.join("runner-profile.json");
    let text = fs::read_to_string(path).ok()?;
    Some(parse_runner_profile_json(&text))
}

pub fn parse_runner_profile_json(json: &str) -> RunnerProfile {
    let mut prof = RunnerProfile {
        name: extract_json_str(json, "name").unwrap_or_default(),
        kind: extract_json_str(json, "kind").unwrap_or_default(),
        extra_args: extract_json_str(json, "extra_args").unwrap_or_default(),
        env: Vec::new(),
        supports_shaders: extract_json_bool(json, "supports_shaders"),
        supports_warm: extract_json_bool(json, "supports_warm"),
    };
    if let Some(idx) = json.find("\"env\"") {
        let rest = &json[idx + 5..];
        if let Some(open) = rest.find('{') {
            if let Some(close) = rest[open..].find('}') {
                let block = &rest[open + 1..open + close];
                for pair in block.split(',') {
                    if let Some((k_raw, v_raw)) = pair.split_once(':') {
                        let k = k_raw.trim().trim_matches('"').trim();
                        let v = v_raw.trim().trim_matches('"').trim();
                        if !k.is_empty() {
                            prof.env.push((k.to_string(), v.to_string()));
                        }
                    }
                }
            }
        }
    }
    prof
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunnerCapabilities {
    pub has_shaders: bool,
    pub has_warm: bool,
    pub has_translate: bool,
    pub has_upscaler_check: bool,
    pub has_dlss_dlls: bool,
    pub has_victory_settings: bool,
    pub has_sdk_fetcher: bool,
    pub profile: Option<RunnerProfile>,
}

pub fn parse_help_capabilities(help_text: &str) -> RunnerCapabilities {
    let mut caps = RunnerCapabilities::default();
    for line in help_text.lines() {
        let trimmed = line.trim();
        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        match first_word {
            "shaders" => caps.has_shaders = true,
            "warm" => caps.has_warm = true,
            "translate" => caps.has_translate = true,
            "upscaler-check" => caps.has_upscaler_check = true,
            _ => {}
        }
    }
    caps
}

pub fn has_dlss_dlls_beside(exe_path: &Path) -> bool {
    let Some(dir) = exe_path.parent() else {
        return false;
    };
    dir.join("sl.interposer.dll").is_file() || dir.join("nvngx_dlss.dll").is_file()
}

pub fn parse_victory_settings(path: &Path) -> Vec<(String, String)> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim();
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

/// Reads `victory-settings.txt` beside `runner_path` and syncs Play UI fields
/// (`console_model`, `display_mode`, `dlss_mode`, `framegen_x`, `texture_room_percent`,
/// `bvh_builder`, `log_level`, `frame_interval_us`) so the UI always reflects what will launch.
pub fn sync_from_victory_settings(s: &mut Settings, runner_path: &Path) -> bool {
    let Some(parent) = runner_path.parent() else {
        return false;
    };
    let vs_path = parent.join("victory-settings.txt");
    if !vs_path.is_file() {
        return false;
    }
    let pairs = parse_victory_settings(&vs_path);
    if pairs.is_empty() {
        return false;
    }
    let get = |k: &str| pairs.iter().rev().find(|(ek, _)| ek == k).map(|(_, v)| v.as_str());

    if let Some(c) = get("VICTORY_MENU_CONSOLE") {
        if c.eq_ignore_ascii_case("series-s") || c.eq_ignore_ascii_case("series-x") {
            s.console_model = c.to_ascii_lowercase();
        }
    }
    if let Some(out) = get("VICTORY_OUTPUT") {
        if !out.trim().is_empty() {
            s.display_mode = out.trim().to_string();
            s.display_1080p = s.display_mode == "1920x1080";
        }
    } else if let Some(n1440) = get("VICTORY_NATIVE_1440P") {
        if n1440 == "1" {
            s.display_mode = "2560x1440".into();
            s.display_1080p = false;
        }
    }
    if let Some(dlss_on) = get("VICTORY_DLSS") {
        if dlss_on == "0" {
            s.dlss_mode = "off".into();
        } else if let Some(m) = get("VICTORY_DLSS_MODE").or_else(|| get("VICTORY_MENU_DLSS")) {
            let ml = m.trim().to_ascii_lowercase();
            if matches!(ml.as_str(), "off" | "performance" | "balanced" | "quality" | "dlaa") {
                s.dlss_mode = ml;
            }
        }
    } else if let Some(m) = get("VICTORY_MENU_DLSS") {
        let ml = m.trim().to_ascii_lowercase();
        if matches!(ml.as_str(), "off" | "performance" | "balanced" | "quality" | "dlaa") {
            s.dlss_mode = ml;
        }
    }
    if let Some(fg_on) = get("VICTORY_FRAMEGEN") {
        if fg_on == "0" {
            s.framegen_x = 0;
        } else if let Some(fx) = get("VICTORY_FRAMEGEN_X").or_else(|| get("VICTORY_MENU_FRAMEGEN")) {
            s.framegen_x = fx.trim().parse::<u32>().unwrap_or(0).clamp(0, 6);
        }
    } else if let Some(fx) = get("VICTORY_MENU_FRAMEGEN") {
        s.framegen_x = fx.trim().parse::<u32>().unwrap_or(0).clamp(0, 6);
    }
    if let Some(tr) = get("VICTORY_TEXTURE_ROOM_PERCENT") {
        if !tr.trim().is_empty() {
            s.texture_room_percent = tr.trim().to_string();
        }
    }
    if let Some(rt) = get("VICTORY_BVH_BUILDER") {
        s.bvh_builder = rt.trim() == "1";
    }
    if let Some(lg) = get("VICTORY_LOG") {
        if !lg.trim().is_empty() {
            s.log_level = lg.trim().to_ascii_lowercase();
        }
    }
    if let Some(fi) = get("VICTORY_FRAME_INTERVAL_US") {
        s.frame_interval_us = fi.trim().to_string();
    }
    true
}

/// Writes Play UI settings into `victory-settings.txt` beside `runner_path`,
/// preserving all unrelated keys already present in the file.
pub fn sync_to_victory_settings(s: &Settings, runner_path: &Path) -> Result<(), String> {
    let Some(parent) = runner_path.parent() else {
        return Err("Invalid runner path".into());
    };
    if !has_dlss_dlls_beside(runner_path) && !parent.join("victory-settings.txt").is_file() {
        return Ok(());
    }
    let vs_path = parent.join("victory-settings.txt");
    let mut pairs = parse_victory_settings(&vs_path);
    let mut set_kv = |k: &str, v: String| {
        if let Some(entry) = pairs.iter_mut().find(|(ek, _)| ek == k) {
            entry.1 = v;
        } else {
            pairs.push((k.to_string(), v));
        }
    };

    let dlss_m = s.dlss_mode.trim().to_ascii_lowercase();
    let dlss_active = !dlss_m.is_empty() && dlss_m != "off";
    let fg_x = if dlss_active { s.framegen_x.clamp(0, 6) } else { 0 };

    set_kv("VICTORY_MENU_CONSOLE", s.console_model.clone());
    set_kv("VICTORY_MENU_RES", if s.display_mode == "1280x720" { "720".into() } else if s.display_mode == "1920x1080" { "1080".into() } else { "1440".into() });
    set_kv("VICTORY_NATIVE_1440P", if s.display_mode == "2560x1440" { "1" } else { "0" }.into());
    if matches!(s.display_mode.as_str(), "1280x720" | "1664x936" | "1920x1080" | "2560x1440" | "3840x2160") {
        set_kv("VICTORY_OUTPUT", s.display_mode.clone());
    }
    set_kv("VICTORY_DLSS", if dlss_active { "1" } else { "0" }.into());
    set_kv("VICTORY_DLSS_MODE", if dlss_active { dlss_m.clone() } else { "off".into() });
    set_kv("VICTORY_MENU_DLSS", if dlss_active { dlss_m } else { "off".into() });
    set_kv("VICTORY_FRAMEGEN", if fg_x > 0 { "1" } else { "0" }.into());
    set_kv("VICTORY_FRAMEGEN_X", fg_x.to_string());
    set_kv("VICTORY_MENU_FRAMEGEN", fg_x.to_string());
    if !s.texture_room_percent.trim().is_empty() {
        set_kv("VICTORY_TEXTURE_ROOM_PERCENT", s.texture_room_percent.trim().to_string());
    }
    set_kv("VICTORY_BVH_BUILDER", if s.bvh_builder { "1" } else { "0" }.into());
    set_kv("VICTORY_LOG", s.effective_victory_log().to_string());
    if !s.frame_interval_us.trim().is_empty() {
        set_kv("VICTORY_FRAME_INTERVAL_US", s.frame_interval_us.trim().to_string());
        if let Ok(us) = s.frame_interval_us.trim().parse::<u32>() {
            if us > 0 {
                set_kv("VICTORY_MENU_FPS", (1_000_000 / us).max(1).to_string());
            }
        }
    }

    let mut out = String::new();
    for (k, v) in pairs {
        out.push_str(&format!("{k}={v}\n"));
    }
    let tmp = parent.join("victory-settings.txt.tmp");
    fs::write(&tmp, &out).map_err(|e| format!("Cannot write victory-settings.txt: {e}"))?;
    fs::rename(&tmp, &vs_path).map_err(|e| format!("Cannot replace victory-settings.txt: {e}"))?;
    Ok(())
}

pub fn probe_runner_capabilities(exe_path: &Path) -> RunnerCapabilities {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut caps = if let Ok(out) = std::process::Command::new(exe_path)
        .arg("--help")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        parse_help_capabilities(&combined)
    } else {
        RunnerCapabilities::default()
    };
    caps.has_dlss_dlls = has_dlss_dlls_beside(exe_path);
    if let Some(parent) = exe_path.parent() {
        caps.has_victory_settings = parent.join("victory-settings.txt").is_file();
        if let Some(slug) = parent.file_name().and_then(|s| s.to_str()) {
            let sdk_script = Settings::app_dir()
                .join("runners_src")
                .join(slug)
                .join("scripts")
                .join("get-nvidia-sdks.ps1");
            caps.has_sdk_fetcher = sdk_script.is_file();
        }
    }
    if let Some(prof) = load_runner_profile(exe_path) {
        if let Some(s) = prof.supports_shaders {
            caps.has_shaders = s;
        }
        if let Some(w) = prof.supports_warm {
            caps.has_warm = w;
        }
        caps.profile = Some(prof);
    }
    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_probe_detects_shaders_warm_translate_and_upscaler_check() {
        let sample = "\
Commands:
  translate       Run every shader found
  shaders         Translate every shader found into the shader cache
  warm            Make the host pipelines of every pipeline the title ever created
  upscaler-check  Check NVIDIA Streamline, DLSS and DLSS Frame Generation
  run             Prepare, map and bind a title executable
";
        let caps = parse_help_capabilities(sample);
        assert!(caps.has_shaders);
        assert!(caps.has_warm);
        assert!(caps.has_translate);
        assert!(caps.has_upscaler_check);
    }

    #[test]
    fn runner_profile_json_parses_fields_and_env() {
        let json = r#"{
  "name": "Kelvaroq Experimental",
  "kind": "victory",
  "extra_args": "--fast-boot",
  "supports_shaders": true,
  "supports_warm": false,
  "env": {
    "VICTORY_HOST_WAVE": "32",
    "VICTORY_LOG": "warn"
  }
}"#;
        let prof = parse_runner_profile_json(json);
        assert_eq!(prof.name, "Kelvaroq Experimental");
        assert_eq!(prof.kind, "victory");
        assert_eq!(prof.extra_args, "--fast-boot");
        assert_eq!(prof.supports_shaders, Some(true));
        assert_eq!(prof.supports_warm, Some(false));
        assert!(prof.env.contains(&("VICTORY_HOST_WAVE".into(), "32".into())));
        assert!(prof.env.contains(&("VICTORY_LOG".into(), "warn".into())));
    }
}
