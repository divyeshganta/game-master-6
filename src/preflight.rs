//! Universal Pre-Flight System Readiness Scorer (v15.2)
//! Evaluates GPU VRAM tier, active commit/pagefile mount status, TDR safety,
//! display VRAM overhead, shader cache readiness, and CPU capabilities.

use crate::config::Settings;
use crate::diagnostics::{AssetAudit, SystemInfo};
use crate::presets::HardwareTier;

#[derive(Clone, Debug)]
pub struct PreFlightReport {
    pub score: u32,
    pub grade: &'static str,
    pub summary: String,
    pub top_action: Option<String>,
    pub checks: Vec<(bool, String)>,
}

/// Computes a live 0–100 Pre-Flight Readiness score from real hardware probes
/// (strictly read-only; never modifies OS settings).
pub fn evaluate(s: &Settings, sys: &SystemInfo, audit: &AssetAudit) -> PreFlightReport {
    let mut score: i32 = 100;
    let mut issues: Vec<String> = Vec::new();
    let mut checks: Vec<(bool, String)> = Vec::new();

    // 1. CPU AVX2 (required by compatibility runner)
    if !sys.avx2 {
        score -= 40;
        let msg = "CPU lacks AVX2 support".to_string();
        issues.push(msg.clone());
        checks.push((false, msg));
    } else {
        checks.push((true, format!("AVX2 CPU OK ({} cores)", sys.cpu_cores)));
    }

    // 2. Core game files & runner binary
    if audit.passed < audit.total {
        score -= 30;
        let msg = format!("Missing core files ({}/{})", audit.passed, audit.total);
        issues.push(msg.clone());
        checks.push((false, msg));
    } else {
        checks.push((true, format!("Core Files {}/{} OK", audit.passed, audit.total)));
    }

    // 3. Windows commit, free RAM & unmounted pagefile detection
    if sys.pagefile_pending_reboot {
        score -= 18;
        let msg = "Restart Windows to mount D:\\pagefile.sys".to_string();
        issues.push(msg.clone());
        checks.push((false, msg));
    } else if !sys.commit_healthy() && sys.total_commit_gb > 0.0 {
        score -= 22;
        let msg = format!("Low free commit ({:.1} GB)", sys.avail_commit_gb);
        issues.push(msg.clone());
        checks.push((false, msg));
    } else {
        checks.push((true, format!("Pagefile / Commit OK ({:.1} GB free)", sys.avail_commit_gb)));
    }
    if (sys.avail_ram_gb > 0.1 && sys.avail_ram_gb < 16.0) || (sys.avail_commit_gb > 0.1 && sys.avail_commit_gb < 28.0) {
        score -= 5;
        let msg = format!(
            "RAM {:.1}G / Commit {:.1}G free (16G/28G+ ideal)",
            sys.avail_ram_gb, sys.avail_commit_gb
        );
        issues.push(msg.clone());
        checks.push((false, msg));
    }

    // 4. GPU TDR timeout protection
    if !sys.tdr_safe() {
        score -= 12;
        let msg = "Set GPU TdrDelay >= 15s".to_string();
        issues.push("Set GPU TdrDelay >= 15s to prevent driver resets".into());
        checks.push((false, msg));
    } else {
        checks.push((true, format!("GPU TDR Safe ({}s)", sys.tdr_delay_secs.unwrap_or(15))));
    }

    // 5. Shader cache readiness
    let needs_refresh = s.cache_needs_refresh(audit.shader_cache_ready);
    if !audit.shader_cache_ready {
        score -= 15;
        let msg = "Run PREPARE CACHE before launch".to_string();
        issues.push(msg.clone());
        checks.push((false, msg));
    } else if needs_refresh {
        score -= 8;
        let msg = "Refresh shader cache for runner".to_string();
        issues.push(msg.clone());
        checks.push((false, msg));
    } else {
        checks.push((true, "Cache Prepared & Verified".into()));
    }

    // 5b. Dedicated GPU preference
    if s.prefer_dgpu {
        checks.push((true, "Dedicated GPU Active".into()));
    } else {
        checks.push((false, "Dedicated GPU pref OFF".into()));
    }

    // 6. Background GPU/VRAM-heavy apps on <= 12 GB cards
    if sys.vram_tax_mb >= 450 && matches!(HardwareTier::from_vram_gb(sys.vram_gb), HardwareTier::Tight8 | HardwareTier::Mid12) {
        score -= 8;
        let msg = format!("Close background GPU apps (~{} MB)", sys.vram_tax_mb);
        issues.push(format!("Close background GPU apps (~{} MB VRAM tax)", sys.vram_tax_mb));
        checks.push((false, msg));
    }

    // 7. Multi-monitor VRAM budget pressure on <= 12 GB cards
    let tier = HardwareTier::from_vram_gb(sys.vram_gb);
    if sys.display_count > 1 && matches!(tier, HardwareTier::Tight8 | HardwareTier::Mid12) {
        score -= 6;
        let msg = "Multi-monitor (-6 pts · Win+P)".to_string();
        issues.push("Win+P (Second screen only) frees ~1 GB VRAM".into());
        checks.push((false, msg));
    }

    let final_score = score.clamp(0, 100) as u32;
    let grade = match final_score {
        92..=100 => "S",
        80..=91 => "A",
        65..=79 => "B",
        50..=64 => "C",
        _ => "D",
    };

    let top_action = issues.first().cloned();
    let summary = format!("PRE-FLIGHT {} · {}/100", grade, final_score);

    PreFlightReport {
        score: final_score,
        grade,
        summary,
        top_action,
        checks,
    }
}
