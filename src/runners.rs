use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::diagnostics::{folder_stats, format_bytes};
use crate::win32::hardlink_or_copy;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunnerKind {
    Cgbj,
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
            RunnerKind::Cgbj => "cgbj",
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
            RunnerKind::Cgbj => "CGBJ · BASELINE",
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
            "cgbj" | "cool_fox" | "cool-fox" => Some(RunnerKind::Cgbj),
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
            RunnerKind::Cgbj | RunnerKind::LocalZtrx | RunnerKind::VictoryFork | RunnerKind::Victory | RunnerKind::VictoryDiag
        )
    }

    pub fn supports_prepare(self) -> bool {
        self.is_victory_family()
    }

    pub fn detect_from_path(path: &Path) -> Self {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if lower.contains("cgbj") || lower.contains("cool_fox") {
            RunnerKind::Cgbj
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
            RunnerKind::Cgbj => RunnerKind::VictoryFork,
            RunnerKind::LocalZtrx => RunnerKind::VictoryFork,
            RunnerKind::VictoryFork => RunnerKind::Victory,
            RunnerKind::Victory => RunnerKind::VictoryDiag,
            RunnerKind::VictoryDiag => RunnerKind::Xcl,
            RunnerKind::Xcl => RunnerKind::Xbox2Windows,
            RunnerKind::Xbox2Windows => RunnerKind::Generic,
            RunnerKind::Generic => RunnerKind::Cgbj,
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
        "runnerx.exe",
        "openrunner.exe",
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
    if !bak.is_file() {
        return Err(format!("No backup found at {}", bak.display()));
    }
    fs::copy(&bak, exe_path).map_err(|e| format!("Could not restore backup: {e}"))?;
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
