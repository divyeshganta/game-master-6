use crate::config::Settings;
use crate::diagnostics::SystemInfo;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HardwareTier {
    Tight8,
    Mid12,
    High16,
    Ultra20,
}

impl HardwareTier {
    /// Classifies the host GPU from live probed dedicated VRAM (GB).
    /// Falls back safely to Mid12 if VRAM probing is still in progress.
    pub fn from_vram_gb(vram_gb: f64) -> Self {
        if vram_gb <= 0.5 {
            HardwareTier::Mid12
        } else if vram_gb <= 8.5 {
            HardwareTier::Tight8
        } else if vram_gb <= 13.5 {
            HardwareTier::Mid12
        } else if vram_gb <= 18.5 {
            HardwareTier::High16
        } else {
            HardwareTier::Ultra20
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HardwareTier::Tight8 => "8GB TIER",
            HardwareTier::Mid12 => "12GB TIER",
            HardwareTier::High16 => "16GB TIER",
            HardwareTier::Ultra20 => "20GB+ TIER",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresetKind {
    Low,
    Balanced,
    High,
    Custom,
}

impl PresetKind {
    pub fn id(self) -> &'static str {
        match self {
            PresetKind::Low => "low",
            PresetKind::Balanced => "balanced",
            PresetKind::High => "high",
            PresetKind::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PresetKind::Low => "LOW",
            PresetKind::Balanced => "BALANCED",
            PresetKind::High => "HIGH",
            PresetKind::Custom => "CUSTOM",
        }
    }

    #[allow(dead_code)]
    pub fn from_id(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" | "safe" => PresetKind::Low,
            "balanced" => PresetKind::Balanced,
            "high" | "max" => PresetKind::High,
            _ => PresetKind::Custom,
        }
    }
}

/// Applies a hardware-adaptive runner preset (LOW / BALANCED / HIGH) computed
/// dynamically from live GPU VRAM tier and CPU core count.
/// Every emitted parameter maps 1:1 to a verified runtime variable.
pub fn apply_adaptive_preset(s: &mut Settings, preset: PresetKind, sys: &SystemInfo) -> String {
    let tier = HardwareTier::from_vram_gb(sys.vram_gb);

    // Common verified host & stability defaults
    s.prefer_dgpu = true;
    s.zero_overhead = true;
    s.high_priority = true;
    s.timer_1ms = true;
    s.high_perf_power = false;
    s.gpu_timing = false;
    s.dred = false;
    s.capture = false;
    s.warp = false;
    s.bvh_builder = false;
    s.shader_loop_limit.clear();
    s.upscaler_enabled = false;
    s.kbm_crosshair = false;
    s.mouse_aim = false;
    s.input_mode = "pad".into();

    // Common verified throughput & residency flags
    s.texture_no_wait = true;
    s.relay_copy = true;
    s.wide_back = true;
    s.card_rehash_runs = true;
    s.join_copies = true;
    s.join_parts = 8;
    s.indirect_window = true;
    s.indirect_batch = 2;
    s.map_threads = true;
    s.card_awake = true;
    s.show_after = true;
    s.async_submit = true;
    s.wide_refresh = true;
    s.wide_merge = true;
    s.fast_cores = sys.cpu_cores >= 8;
    s.levels_on_demand = true;
    s.no_wait_resident = true;
    s.ambient_fill = true;
    s.ambient_val = "0.8".into();
    s.ambient_air = "1.5".into();
    s.texture_budget_mb.clear();
    s.texture_stage_mb.clear();

    match (tier, preset) {
        // ── Tier 1: <= 8 GB VRAM ───────────────────────────────────────────
        (HardwareTier::Tight8, PresetKind::Low) => {
            s.console_model = "series-s".into();
            s.display_mode = "1280x720".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "512".into();
            s.texture_room_percent = "85".into();
            s.card_room_for_targets_mb = "1536".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::Tight8, PresetKind::Balanced) => {
            s.console_model = "series-s".into();
            s.display_mode = "1280x720".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "512".into();
            s.texture_room_percent = "85".into();
            s.card_room_for_targets_mb = "1536".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::Tight8, PresetKind::High) => {
            s.console_model = "series-s".into();
            s.display_mode = "1600x900".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "768".into();
            s.texture_room_percent = "90".into();
            s.card_room_for_targets_mb = "2048".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }

        // ── Tier 2: 10–12 GB VRAM (e.g. RTX 5070 Ti 12 GB) ─────────────────
        (HardwareTier::Mid12, PresetKind::Low) => {
            s.console_model = "series-s".into();
            s.display_mode = "1280x720".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "768".into();
            s.texture_room_percent = "90".into();
            s.card_room_for_targets_mb = "2048".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::Mid12, PresetKind::Balanced) => {
            // Verified 12 GB 180213.log baseline: Series S + 900p + 4 MB min buf + 2560 MB targets + 94% tex room
            s.console_model = "series-s".into();
            s.display_mode = "1600x900".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::Mid12, PresetKind::High) => {
            s.console_model = "series-s".into();
            s.display_mode = "1920x1080".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }

        // ── Tier 3: 16 GB VRAM ─────────────────────────────────────────────
        (HardwareTier::High16, PresetKind::Low) => {
            s.console_model = "series-x".into();
            s.display_mode = "1920x1080".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::High16, PresetKind::Balanced) => {
            s.console_model = "series-x".into();
            s.display_mode = "1920x1080".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::High16, PresetKind::High) => {
            s.console_model = "series-x".into();
            s.display_mode = "2560x1440".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "2048".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }

        // ── Tier 4: >= 20 GB VRAM ──────────────────────────────────────────
        (HardwareTier::Ultra20, PresetKind::Low) => {
            s.console_model = "series-x".into();
            s.display_mode = "1920x1080".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::Ultra20, PresetKind::Balanced) => {
            s.console_model = "series-x".into();
            s.display_mode = "1920x1080".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }
        (HardwareTier::Ultra20, PresetKind::High) => {
            s.console_model = "series-x".into();
            s.display_mode = "auto".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "4096".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "3072".into();
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = true;
            s.upload_ahead = true;
            s.log_level = "warn".into();
        }

        // Custom does not overwrite user overrides
        (_, PresetKind::Custom) => {
            s.active_preset = "custom".into();
            return "Active preset: CUSTOM".into();
        }
    }

    s.display_1080p = s.display_mode == "1920x1080";
    s.active_preset = preset.id().into();

    format!(
        "Applied {} preset for {} ({}, {}, {}% tex room, {} MB min buf)",
        preset.label(),
        tier.label(),
        s.console_model,
        s.display_mode,
        s.texture_room_percent,
        s.card_buffers_min_mb
    )
}

/// Detects whether current Settings still match one of the adaptive presets
/// for the detected hardware tier, or have been customized.
pub fn detect_active_preset(s: &Settings, sys: &SystemInfo) -> PresetKind {
    for candidate in [PresetKind::Low, PresetKind::Balanced, PresetKind::High] {
        let mut probe = s.clone();
        let _ = apply_adaptive_preset(&mut probe, candidate, sys);
        if s.console_model == probe.console_model
            && s.display_mode == probe.display_mode
            && s.card_buffers_min_mb.trim() == probe.card_buffers_min_mb.trim()
            && s.card_buffers_mb.trim() == probe.card_buffers_mb.trim()
            && s.texture_room_percent.trim() == probe.texture_room_percent.trim()
            && s.card_room_for_targets_mb.trim() == probe.card_room_for_targets_mb.trim()
            && s.relay_mode == probe.relay_mode
            && s.card_steps == probe.card_steps
            && s.thin_lists == probe.thin_lists
            && s.ambient_val == probe.ambient_val
            && s.ambient_air == probe.ambient_air
            && s.log_level == probe.log_level
        {
            return candidate;
        }
    }
    PresetKind::Custom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::build_runner_env;

    #[test]
    fn mid12_balanced_emits_verified_v15_2_env() {
        let mut s = Settings::default();
        let mut sys = SystemInfo::default();
        sys.vram_gb = 12.0;
        sys.cpu_cores = 16;

        let _ = apply_adaptive_preset(&mut s, PresetKind::Balanced, &sys);
        let env = build_runner_env(&s);
        let get = |k: &str| env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.as_str());

        assert_eq!(get("VICTORY_DISPLAY_MODE"), Some("1600x900"));
        assert_eq!(get("VICTORY_CARD_BUFFERS_MIN_MB"), Some("4"));
        assert_eq!(get("VICTORY_CARD_ROOM_FOR_TARGETS_MB"), Some("2560"));
        assert_eq!(get("VICTORY_TEXTURE_ROOM_PERCENT"), Some("94"));
        assert_eq!(get("VICTORY_WAIT_RESIDENT"), Some("0"));
    }

    #[test]
    fn detect_active_preset_distinguishes_all_presets_across_tiers() {
        for vram in [8.0, 12.0, 16.0, 24.0] {
            let mut sys = SystemInfo::default();
            sys.vram_gb = vram;
            sys.cpu_cores = 16;
            for kind in [PresetKind::Low, PresetKind::Balanced, PresetKind::High] {
                let mut s = Settings::default();
                let _ = apply_adaptive_preset(&mut s, kind, &sys);
                assert_eq!(detect_active_preset(&s, &sys), kind, "vram={vram} kind={kind:?}");
                let env = build_runner_env(&s);
                let get = |k: &str| env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.as_str());
                assert_eq!(get("VICTORY_RELAY"), Some("2"), "vram={vram} kind={kind:?}");
                assert_eq!(get("VICTORY_CARD_STEPS"), Some("2"), "vram={vram} kind={kind:?}");
                assert_eq!(s.log_level, "warn", "vram={vram} kind={kind:?}");
            }
        }
    }

    #[test]
    fn settings_default_has_no_personal_paths() {
        let s = Settings::default();
        assert!(s.exe.is_empty());
        assert!(s.game_dir.is_empty());
    }
}
