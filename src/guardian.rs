//! Automated Crash-Recovery & Last-Known-Good Guardian (v15.2)
//! Snapshots proven-stable settings after >=90s of clean runtime, diagnoses early crashes
//! from victory logs, and enables 1-click Safe Boot rollback without losing user paths.

use std::fs;
use std::path::PathBuf;

use crate::config::Settings;

#[derive(Clone, Debug)]
pub struct CrashVerdict {
    pub category: &'static str,
    pub cause: String,
    pub recommended_fix: String,
    pub auto_safe_boot_available: bool,
}

pub fn last_known_good_path() -> PathBuf {
    Settings::app_dir().join("GameMaster.last_good.settings")
}

pub fn has_last_known_good() -> bool {
    last_known_good_path().is_file()
}

/// Saves a clean snapshot of current settings once a session proves stable (>= 90s).
pub fn record_known_good_if_stable(duration_secs: u64, exit_code: Option<i32>) {
    let clean_exit = matches!(exit_code, Some(0) | None);
    if duration_secs >= 90 && clean_exit {
        let src = Settings::path();
        if src.is_file() {
            let _ = fs::copy(src, last_known_good_path());
        }
    }
}

/// Restores `GameMaster.last_good.settings` over `GameMaster.settings` and reloads.
pub fn restore_last_known_good() -> Result<Settings, String> {
    let lkg = last_known_good_path();
    if !lkg.is_file() {
        return Err("No Last-Known-Good snapshot recorded yet (requires one 90s+ stable run).".into());
    }
    fs::copy(&lkg, Settings::path()).map_err(|e| format!("Failed to restore Last-Known-Good: {e}"))?;
    Ok(Settings::load())
}

/// Analyzes a failed or short (<45s) session log to pinpoint the exact victory failure mode.
pub fn diagnose_crash(exit_code: Option<i32>, duration_secs: u64, log_lines: &[String]) -> Option<CrashVerdict> {
    let is_crash = match exit_code {
        Some(0) | None => false,
        Some(_) => true,
    };
    if !is_crash && duration_secs >= 20 {
        return None;
    }

    let mut device_removed = false;
    let mut commit_oom = false;
    let mut vram_exhausted = false;
    let mut bvh_fault = false;

    for line in log_lines {
        let lo = line.to_ascii_lowercase();
        if lo.contains("device_removed") || lo.contains("0x887a0005") || lo.contains("0x887a0006") || lo.contains("device hung") {
            device_removed = true;
        }
        if lo.contains("host memory pressure") || lo.contains("1455") || lo.contains("out of memory") {
            commit_oom = true;
        }
        if lo.contains("the card has no room for more") {
            vram_exhausted = true;
        }
        if lo.contains("bvh") && (lo.contains("panic") || lo.contains("fault") || lo.contains("error")) {
            bvh_fault = true;
        }
    }

    let lkg = has_last_known_good();
    if bvh_fault {
        Some(CrashVerdict {
            category: "EXPERIMENTAL BVH FAULT",
            cause: "Experimental BVH ray-tracing builder faulted during pipeline setup.".into(),
            recommended_fix: "Disable VICTORY_BVH_BUILDER in ADVANCED or apply BALANCED preset.".into(),
            auto_safe_boot_available: lkg,
        })
    } else if device_removed {
        Some(CrashVerdict {
            category: "GPU DEVICE REMOVED (TDR)",
            cause: "Windows reset the GPU driver during a heavy frame or VRAM spill.".into(),
            recommended_fix: "Verify TdrDelay >= 15s, switch to LOW/BALANCED, or close background GPU apps.".into(),
            auto_safe_boot_available: lkg,
        })
    } else if commit_oom {
        Some(CrashVerdict {
            category: "WINDOWS COMMIT EXHAUSTED",
            cause: "Total Windows commit limit ran dry while streaming textures.".into(),
            recommended_fix: "Ensure D:\\pagefile.sys (32-48 GB) is active (full Windows Restart required).".into(),
            auto_safe_boot_available: lkg,
        })
    } else if vram_exhausted {
        Some(CrashVerdict {
            category: "VRAM BUDGET EXHAUSTED",
            cause: "Dedicated VRAM filled completely; textures stalled waiting for eviction.".into(),
            recommended_fix: "Use series-s, set VICTORY_TEXTURE_ROOM_PERCENT=82, or use single-display (Win+P).".into(),
            auto_safe_boot_available: lkg,
        })
    } else if is_crash {
        Some(CrashVerdict {
            category: "EARLY RUNNER EXIT",
            cause: format!("Runner exited abnormally after {duration_secs}s."),
            recommended_fix: if lkg {
                "Click SAFE BOOT to roll back to your Last-Known-Good settings.".into()
            } else {
                "Inspect the session log or switch to the LOW stability preset.".into()
            },
            auto_safe_boot_available: lkg,
        })
    } else {
        None
    }
}
