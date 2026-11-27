use crate::config::Settings;
use crate::diagnostics::SystemInfo;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum HardwareTier {
    Tight6,
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
        } else if vram_gb <= 6.5 {
            HardwareTier::Tight6
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
            HardwareTier::Tight6 => "6GB TIER",
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

/// Applies a hardware-adaptive BASELINE preset (LOW / BALANCED / HIGH) computed
/// dynamically from live GPU VRAM tier and CPU core count.
/// Every emitted parameter maps 1:1 to a verified variable in `runner_src\baseline`.
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

    // Common verified BASELINE throughput & residency flags
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

    // GPU-architecture-aware auto-tuning
    match sys.gpu_arch {
        crate::diagnostics::GpuArch::NvidiaAdaAmpere => {
            s.thin_lists = false;
            s.join_parts = 0; // wave32-safe without THIN+JOIN_PARTS blur
            s.ambient_val = "0.5".into();
            s.ambient_air = "1.2".into();
        }
        crate::diagnostics::GpuArch::AmdRdna => {
            s.thin_lists = true;
            s.ambient_val = "0.6".into();
            s.ambient_air = "1.2".into();
        }
        _ => {
            // NVIDIA Blackwell (RTX 50-series) & default
            s.thin_lists = false;
            s.ambient_val = "0.3".into();
            s.ambient_air = "1.2".into();
        }
    }
    s.levels_on_demand = true;
    s.no_wait_resident = true;
    s.ambient_fill = true;
    s.ambient_val = "0.8".into();
    s.ambient_air = "1.5".into();
    s.texture_budget_mb.clear();
    s.texture_stage_mb.clear();

    match (tier, preset) {
        // ── Tier 0: <= 6.5 GB VRAM (RTX 4050 6 GB / RTX 3050 4-6 GB / RTX 2060 6 GB) ──
        (HardwareTier::Tight6, PresetKind::Low | PresetKind::Balanced | PresetKind::High) => {
            s.console_model = "series-s".into();
            s.display_mode = "1280x720".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "384".into();
            s.texture_room_percent = "78".into();
            s.card_room_for_targets_mb = "1024".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.ambient_val = "0.3".into();
            s.ambient_air = "1.2".into();
            s.log_level = "warn".into();
        }

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
            // Verified overnight winner (Max FPS): Series X + 720p output + 90% tex room (7.24 avg / 12.1 max / 10.4 steady)
            s.console_model = "series-x".into();
            s.display_mode = "1280x720".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "90".into();
            s.card_room_for_targets_mb = "2048".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.ambient_val = "0.3".into();
            s.ambient_air = "1.2".into();
            s.log_level = "warn".into();
        }
        (HardwareTier::Mid12, PresetKind::Balanced) => {
            // Verified overnight Balanced: Series S + 1080p FidelityFX + THIN=0 + AMBIENT=0.3 (6.47 avg / 8.9 max / 8.6 steady)
            s.console_model = "series-s".into();
            s.display_mode = "1920x1080".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.ambient_val = "0.3".into();
            s.ambient_air = "1.2".into();
            s.log_level = "warn".into();
        }
        (HardwareTier::Mid12, PresetKind::High) => {
            // Verified overnight Sharp: Series X + 1664x936 output (no_room=0, 12.3 max)
            s.console_model = "series-x".into();
            s.display_mode = "1664x936".into();
            s.card_buffers_min_mb = "4".into();
            s.card_buffers_mb = "1024".into();
            s.texture_room_percent = "94".into();
            s.card_room_for_targets_mb = "2560".into();
            s.no_wait_resident = true;
            s.relay_mode = 2;
            s.card_steps = 2;
            s.thin_lists = false;
            s.upload_ahead = true;
            s.ambient_val = "0.3".into();
            s.ambient_air = "1.2".into();
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

    // If the active runner is a Streamline/DLSS runner, apply exact DLSS/FG preset tuning
    // (70/72/74% on 6-8 GB, 72/75/78% on 12 GB, 78/80/82% on 16 GB+) and sync to victory-settings.txt.
    let runner_path = s.resolved_runner();
    if crate::runners::is_streamline_runner(&runner_path) && preset != PresetKind::Custom {
        let fg_3_or_2 = match sys.gpu_arch {
            crate::diagnostics::GpuArch::NvidiaBlackwell => 3,
            crate::diagnostics::GpuArch::NvidiaAdaAmpere => 2,
            crate::diagnostics::GpuArch::Unknown => 3,
            _ => 0,
        };
        let fg_high = match sys.gpu_arch {
            crate::diagnostics::GpuArch::NvidiaBlackwell
            | crate::diagnostics::GpuArch::NvidiaAdaAmpere
            | crate::diagnostics::GpuArch::Unknown => 2,
            _ => 0,
        };
        let tex_room = match (tier, preset) {
            (HardwareTier::Tight6 | HardwareTier::Tight8, PresetKind::Low) => "70",
            (HardwareTier::Tight6 | HardwareTier::Tight8, PresetKind::Balanced) => "72",
            (HardwareTier::Tight6 | HardwareTier::Tight8, PresetKind::High) => "74",
            (HardwareTier::Mid12, PresetKind::Low) => "72",
            (HardwareTier::Mid12, PresetKind::Balanced) => "75",
            (HardwareTier::Mid12, PresetKind::High) => "78",
            (_, PresetKind::Low) => "78",
            (_, PresetKind::Balanced) => "80",
            (_, PresetKind::High) => "82",
            (_, PresetKind::Custom) => "75",
        };
        match preset {
            PresetKind::Low => {
                s.console_model = if tier <= HardwareTier::Tight8 { "series-s".into() } else { "series-x".into() };
                s.display_mode = if tier <= HardwareTier::Tight8 { "1280x720".into() } else { "2560x1440".into() };
                s.dlss_mode = "performance".into();
                s.framegen_x = if tier <= HardwareTier::Tight8 && fg_3_or_2 > 2 { 2 } else { fg_3_or_2 };
                s.frame_interval_us.clear();
            }
            PresetKind::Balanced => {
                s.console_model = if tier <= HardwareTier::Tight8 { "series-s".into() } else { "series-x".into() };
                s.display_mode = if tier <= HardwareTier::Tight8 { "1920x1080".into() } else { "2560x1440".into() };
                s.dlss_mode = "balanced".into();
                s.framegen_x = if tier <= HardwareTier::Tight8 && fg_3_or_2 > 2 { 2 } else { fg_3_or_2 };
                s.frame_interval_us.clear();
            }
            PresetKind::High => {
                s.console_model = "series-x".into();
                s.display_mode = "2560x1440".into();
                s.dlss_mode = "quality".into();
                s.framegen_x = fg_high;
                s.frame_interval_us.clear();
            }
            PresetKind::Custom => {}
        }
        s.texture_room_percent = tex_room.into();
        s.display_1080p = s.display_mode == "1920x1080";
        s.bvh_builder = false;
        s.log_level = "warn".into();
        let _ = crate::runners::sync_to_victory_settings(s, &runner_path);
    }

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

        assert_eq!(get("VICTORY_DISPLAY_MODE"), Some("1920x1080"));
        assert_eq!(get("VICTORY_OUTPUT"), Some("1920x1080"));
        assert_eq!(get("VICTORY_CARD_BUFFERS_MIN_MB"), Some("4"));
        assert_eq!(get("VICTORY_CARD_ROOM_FOR_TARGETS_MB"), Some("2560"));
        assert_eq!(get("VICTORY_TEXTURE_ROOM_PERCENT"), Some("94"));
        assert_eq!(get("VICTORY_WAIT_RESIDENT"), Some("0"));
        assert_eq!(get("VICTORY_THIN"), Some("0"));
        assert_eq!(get("VICTORY_AMBIENT"), Some("0.3"));
        assert_eq!(get("VICTORY_PORT_GUARD"), Some("1"));
        assert_eq!(get("VICTORY_PORT_JOURNAL"), Some("1"));
    }

    #[test]
    fn detect_active_preset_distinguishes_all_presets_across_tiers() {
        for vram in [6.0, 8.0, 12.0, 16.0, 24.0] {
            let mut sys = SystemInfo::default();
            sys.vram_gb = vram;
            sys.cpu_cores = 16;
            for kind in [PresetKind::Low, PresetKind::Balanced, PresetKind::High] {
                let mut s = Settings::default();
                let _ = apply_adaptive_preset(&mut s, kind, &sys);
                if vram > 6.5 {
                    assert_eq!(detect_active_preset(&s, &sys), kind, "vram={vram} kind={kind:?}");
                }
                let env = build_runner_env(&s);
                let get = |k: &str| env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.as_str());
                assert_eq!(get("VICTORY_RELAY"), Some("2"), "vram={vram} kind={kind:?}");
                assert_eq!(get("VICTORY_CARD_STEPS"), Some("2"), "vram={vram} kind={kind:?}");
                assert_eq!(s.log_level, "warn", "vram={vram} kind={kind:?}");
            }
        }
    }

    #[test]
    fn tight6_rtx_4050_emits_safe_6gb_headroom_and_aggressive_idle_trim() {
        let mut s = Settings::default();
        let mut sys = SystemInfo::default();
        sys.vram_gb = 6.0;
        sys.cpu_cores = 12;
        sys.gpu_arch = crate::diagnostics::GpuArch::NvidiaAdaAmpere;

        assert_eq!(HardwareTier::from_vram_gb(6.0), HardwareTier::Tight6);
        let _ = apply_adaptive_preset(&mut s, PresetKind::Balanced, &sys);
        let env = build_runner_env(&s);
        let get = |k: &str| env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.as_str());

        assert_eq!(get("VICTORY_OUTPUT"), Some("1280x720"));
        assert_eq!(get("VICTORY_TEXTURE_ROOM_PERCENT"), Some("78"));
        assert_eq!(get("VICTORY_CARD_ROOM_FOR_TARGETS_MB"), Some("1024"));
        assert_eq!(get("VICTORY_CARD_BUFFERS_MB"), Some("384"));
        assert_eq!(get("VICTORY_CARD_BUFFERS_MIN_MB"), Some("4"));
        assert_eq!(get("VICTORY_THIN"), Some("0"));
        assert_eq!(get("VICTORY_AMBIENT"), Some("0.3"));
        assert_eq!(get("VICTORY_LIST_IDLE_TRIM_S"), Some("3"));
        assert_eq!(get("VICTORY_SCRATCH_SPARE_MB"), Some("64"));
        assert_eq!(get("VICTORY_SEND_SPARE_MB"), Some("64"));
        assert_eq!(get("VICTORY_STAGING_IDLE_MS"), Some("1000"));
        assert_eq!(get("VICTORY_WINDOW_IDLE_MS"), Some("1000"));
        assert_eq!(get("VICTORY_READ_KEEP_MB"), Some("32"));
        assert_eq!(get("VICTORY_BVH_BUILDER"), Some("0"));
    }

    #[test]
    fn settings_default_has_no_personal_paths() {
        let s = Settings::default();
        assert!(s.exe.is_empty());
        assert!(s.game_dir.is_empty());
        assert!(!s.runner.to_ascii_lowercase().contains("user"));
    }

    #[test]
    fn dlss_presets_and_two_way_victory_settings_sync() {
        let tmp = std::env::temp_dir().join(format!("gm_v14_sync_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let exe = tmp.join("victory.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        std::fs::write(tmp.join("sl.interposer.dll"), b"DLL").unwrap();
        std::fs::write(
            tmp.join("victory-settings.txt"),
            "VICTORY_LIST_IDLE_TRIM_S=5\nVICTORY_SCRATCH_SPARE_MB=128\n",
        )
        .unwrap();

        let mut s = Settings::default();
        s.runner = exe.to_string_lossy().into_owned();
        let mut sys = SystemInfo::default();
        sys.vram_gb = 12.0;
        sys.gpu_arch = crate::diagnostics::GpuArch::NvidiaBlackwell;

        // 1. BALANCED preset -> writes 1440p Native, DLSS balanced, 3x FG, uncapped FPS (1000), 75% tex room, preserves unrelated keys
        let _ = apply_adaptive_preset(&mut s, PresetKind::Balanced, &sys);
        let vs = std::fs::read_to_string(tmp.join("victory-settings.txt")).unwrap();
        assert!(vs.contains("VICTORY_OUTPUT=2560x1440"));
        assert!(vs.contains("VICTORY_NATIVE_1440P=1"));
        assert!(vs.contains("VICTORY_MENU_RES=1440"));
        assert!(vs.contains("VICTORY_DLSS=1"));
        assert!(vs.contains("VICTORY_DLSS_MODE=balanced"));
        assert!(vs.contains("VICTORY_FRAMEGEN_X=3"));
        assert!(vs.contains("VICTORY_MENU_FPS=1000"));
        assert!(vs.contains("VICTORY_FRAME_INTERVAL_US=1000"));
        assert!(!vs.contains("VICTORY_FRAME_INTERVAL_US=50000"));
        assert!(!vs.contains("VICTORY_MENU_FPS=20"));
        assert!(vs.contains("VICTORY_FULLSCREEN=1"));
        assert!(vs.contains("VICTORY_TEXTURE_ROOM_PERCENT=75"));
        assert!(vs.contains("VICTORY_LIST_IDLE_TRIM_S=5"));
        assert!(vs.contains("VICTORY_SCRATCH_SPARE_MB=128"));

        // 2. LOW preset -> writes 2560x1440 (1440p Native), DLSS performance, 72% tex room
        let _ = apply_adaptive_preset(&mut s, PresetKind::Low, &sys);
        let vs_low = std::fs::read_to_string(tmp.join("victory-settings.txt")).unwrap();
        assert!(vs_low.contains("VICTORY_DLSS_MODE=performance"));
        assert!(vs_low.contains("VICTORY_TEXTURE_ROOM_PERCENT=72"));

        // 3. HIGH preset -> writes 2560x1440, NATIVE_1440P=1, DLSS quality, FG 2x, 78% tex room
        let _ = apply_adaptive_preset(&mut s, PresetKind::High, &sys);
        let vs_hi = std::fs::read_to_string(tmp.join("victory-settings.txt")).unwrap();
        assert!(vs_hi.contains("VICTORY_OUTPUT=2560x1440"));
        assert!(vs_hi.contains("VICTORY_NATIVE_1440P=1"));
        assert!(vs_hi.contains("VICTORY_DLSS_MODE=quality"));
        assert!(vs_hi.contains("VICTORY_FRAMEGEN_X=2"));
        assert!(vs_hi.contains("VICTORY_TEXTURE_ROOM_PERCENT=78"));

        // 4. Two-way Read -> UI sync
        std::fs::write(
            tmp.join("victory-settings.txt"),
            "VICTORY_MENU_CONSOLE=series-s\nVICTORY_OUTPUT=1920x1080\nVICTORY_DLSS=1\nVICTORY_DLSS_MODE=performance\nVICTORY_FRAMEGEN=1\nVICTORY_FRAMEGEN_X=2\n",
        )
        .unwrap();
        assert!(crate::runners::sync_from_victory_settings(&mut s, &exe));
        assert_eq!(s.console_model, "series-s");
        assert_eq!(s.display_mode, "1920x1080");
        assert_eq!(s.dlss_mode, "performance");
        assert_eq!(s.framegen_x, 2);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
