//! Shader Pipeline Auto-Healer (v15.2)
//! Scans recent runner logs for severe shader/pipeline compile or draw spikes (>= 80ms),
//! ranks top rogue hashes, and manages clean quarantine into `VICTORY_SKIP_PIPELINES`.

use crate::config::Settings;
use crate::diagnostics::SlowShader;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ShaderHealCandidate {
    pub hash: String,
    pub spike_ms: f64,
    pub already_skipped: bool,
}

/// Identifies severe shader offenders (>= 80ms) from parsed log timings.
#[allow(dead_code)]
pub fn find_severe_shaders(s: &Settings, slow: &[SlowShader]) -> Vec<ShaderHealCandidate> {
    let mut out = Vec::new();
    for item in slow {
        let ms = item.ms.unwrap_or(0.0);
        if ms >= 80.0 && !item.hash.trim().is_empty() {
            out.push(ShaderHealCandidate {
                hash: item.hash.clone(),
                spike_ms: ms,
                already_skipped: s.is_hash_skipped(&item.hash),
            });
        }
    }
    out.sort_by(|a, b| b.spike_ms.partial_cmp(&a.spike_ms).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(8);
    out
}

/// Quarantines the single worst unskipped shader hash (>= 150ms) into `skip_pipelines`.
#[allow(dead_code)]
pub fn quarantine_worst_shader(s: &mut Settings, slow: &[SlowShader]) -> Option<(String, f64)> {
    let candidates = find_severe_shaders(s, slow);
    let worst = candidates.into_iter().find(|c| !c.already_skipped && c.spike_ms >= 150.0)?;
    s.toggle_skip_hash(&worst.hash);
    Some((worst.hash, worst.spike_ms))
}
