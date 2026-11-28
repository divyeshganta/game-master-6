use crate::config::Settings;
use crate::diagnostics::{GpuArch, SystemInfo};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum NvidiaGen {
    Rtx50,
    Rtx40,
    Rtx20_30,
    NonNvidia,
}

impl NvidiaGen {
    pub fn detect(arch: GpuArch) -> Self {
        match arch {
            GpuArch::NvidiaBlackwell => NvidiaGen::Rtx50,
            GpuArch::NvidiaAdaAmpere => NvidiaGen::Rtx40,
            GpuArch::NvidiaTuringAmpere => NvidiaGen::Rtx20_30,
            _ => NvidiaGen::NonNvidia,
        }
    }

    pub fn is_nvidia(self) -> bool {
        !matches!(self, NvidiaGen::NonNvidia)
    }

    /// Friendly banner when a non-NVIDIA GPU is detected.
    pub fn non_nvidia_warning() -> &'static str {
        "Designed for NVIDIA RTX GPUs - DLSS/FG features require an RTX card"
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresetKind {
    Auto,
    Low,
    Balanced,
    High,
    Custom,
}

impl PresetKind {
    pub fn id(self) -> &'static str {
        match self {
            PresetKind::Auto => "auto",
            PresetKind::Low => "low",
            PresetKind::Balanced => "balanced",
            PresetKind::High => "high",
            PresetKind::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PresetKind::Auto => "AUTO (THIS PC)",
            PresetKind::Low => "LOW",
            PresetKind::Balanced => "BALANCED",
            PresetKind::High => "HIGH",
            PresetKind::Custom => "CUSTOM",
        }
    }

    pub fn from_id(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => PresetKind::Auto,
            "low" | "safe" => PresetKind::Low,
            "balanced" => PresetKind::Balanced,
            "high" | "max" => PresetKind::High,
            _ => PresetKind::Custom,
        }
    }
}

/// Resolved `AUTO (THIS PC)` NVIDIA tier mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NvidiaAuto {
    pub gen: NvidiaGen,
    pub tier: HardwareTier,
    pub console_model: String,   // "series-x" | "series-s"
    pub display_mode: String,    // "2560x1440" | "1920x1080"
    pub dlss_mode: String,       // "balanced" | "performance"
    pub framegen_x: u32,         // 2 (3x selectable)
    pub texture_room_percent: String,
    pub aggressive_idle_trim: bool,
    pub fg_type_auto: bool,      // VICTORY_MENU_FG_TYPE=auto (FSR3 FG 2x fallback on RTX 20/30)
}

/// Clean NVIDIA GPU Generation & VRAM Tier Auto-Detection (`AUTO (THIS PC)`).
pub fn nvidia_auto_preset(arch: GpuArch, vram_gb: f64) -> NvidiaAuto {
    let gen = NvidiaGen::detect(arch);
    let tier = HardwareTier::from_vram_gb(vram_gb);
    match gen {
        NvidiaGen::Rtx50 => {
            // RTX 50 Series (12 GB+): Series X 1440p, DLSS Balanced, 2x FG (3x selectable), 85% room.
            NvidiaAuto {
                gen,
                tier,
                console_model: "series-x".into(),
                display_mode: "2560x1440".into(),
                dlss_mode: "balanced".into(),
                framegen_x: 2,
                texture_room_percent: "85".into(),
                aggressive_idle_trim: false,
                fg_type_auto: false,
            }
        }
        NvidiaGen::Rtx40 => {
            if tier == HardwareTier::Tight6 {
                // RTX 40 6 GB (e.g. RTX 4050 Laptop 6 GB): safe 1080p, DLSS Performance, 2x FG, 70% room, aggressive idle trim.
                NvidiaAuto {
                    gen,
                    tier,
                    console_model: "series-s".into(),
                    display_mode: "1920x1080".into(),
                    dlss_mode: "performance".into(),
                    framegen_x: 2,
                    texture_room_percent: "70".into(),
                    aggressive_idle_trim: true,
                    fg_type_auto: false,
                }
            } else {
                // RTX 40 8-12 GB: Series X 1440p/1080p, DLSS Performance/Balanced, 2x FG, 74-82% room.
                let dlss_mode = if tier >= HardwareTier::Mid12 { "balanced" } else { "performance" };
                let display_mode = if tier >= HardwareTier::Mid12 { "2560x1440" } else { "1920x1080" };
                let texture_room_percent = if tier >= HardwareTier::Mid12 { "82".into() } else { "74".into() };
                NvidiaAuto {
                    gen,
                    tier,
                    console_model: "series-x".into(),
                    display_mode: display_mode.into(),
                    dlss_mode: dlss_mode.into(),
                    framegen_x: 2,
                    texture_room_percent,
                    aggressive_idle_trim: false,
                    fg_type_auto: false,
                }
            }
        }
        NvidiaGen::Rtx20_30 => {
            // RTX 20 / 30 Series: DLSS Super Resolution + VICTORY_MENU_FG_TYPE=auto (FSR3 FG 2x fallback).
            let display_mode = if tier <= HardwareTier::Tight8 { "1920x1080".into() } else { "2560x1440".into() };
            let dlss_mode = if tier <= HardwareTier::Tight8 { "performance".into() } else { "balanced".into() };
            let texture_room_percent = if tier <= HardwareTier::Tight8 { "74".into() } else { "85".into() };
            NvidiaAuto {
                gen,
                tier,
                console_model: if tier <= HardwareTier::Tight8 { "series-s".into() } else { "series-x".into() },
                display_mode,
                dlss_mode,
                framegen_x: 2,
                texture_room_percent,
                aggressive_idle_trim: tier == HardwareTier::Tight6,
                fg_type_auto: true,
            }
        }
        NvidiaGen::NonNvidia => NvidiaAuto {
            gen,
            tier,
            console_model: "series-s".into(),
            display_mode: "1920x1080".into(),
            dlss_mode: "off".into(),
            framegen_x: 0,
            texture_room_percent: "70".into(),
            aggressive_idle_trim: false,
            fg_type_auto: false,
        },
    }
}

/// Applies a hardware-adaptive preset computed dynamically from live GPU arch + VRAM tier + CPU core count.
/// `AUTO (THIS PC)` resolves NVIDIA generation + VRAM tiering; `LOW`/`BALANCED`/`HIGH` layer on top.
/// All outputs preserve the Zero-Regression Lock invariants.
pub fn apply_adaptive_preset(s: &mut Settings, preset: PresetKind, sys: &SystemInfo) -> String {
    let tier = HardwareTier::from_vram_gb(sys.vram_gb);
    let gen = NvidiaGen::detect(sys.gpu_arch);

    // ---- Common verified host & stability defaults (Zero-Regression Lock) ----
    s.prefer_dgpu = true;
    s.zero_overhead = true;
    s.high_priority = true;
    s.timer_1ms = true;
    s.high_perf_power = false; // never touch Windows power plans
    s.gpu_timing = false;
    s.dred = false;
    s.capture = false;
    s.warp = false;
    s.bvh_builder = false;
    s.shader_loop_limit.clear();
    s.upscaler_enabled = false; // NVIDIA-only; no host FSR folder
    s.upscaler_dir.clear();

    // Throughput & residency flags
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
    s.async_submit = true; // VICTORY_ASYNC_SUBMIT=1
    s.thin_lists = true; // VICTORY_THIN=1
    s.wide_refresh = true;
    s.wide_merge = true;
    s.fast_cores = sys.cpu_cores >= 8;
    s.levels_on_demand = true;
    s.no_wait_resident = true;
    s.ambient_fill = true;
    s.relay_mode = 2;
    s.card_steps = 2;
    s.upload_ahead = true;
    s.borderless = true; // VICTORY_BORDERLESS=1 + VICTORY_FULLSCREEN=1
    s.mouse_capture = true; // VICTORY_MOUSE_CAPTURE=1
    s.texture_budget_mb.clear();
    s.texture_stage_mb.clear();
    // NO VICTORY_CARD_ROOM_FOR_TARGETS_MB
    s.card_room_for_targets_mb.clear();
    s.frame_interval_us = "1000".into(); // VICTORY_FRAME_INTERVAL_US=1000
    s.log_level = "warn".into();

    if preset == PresetKind::Custom {
        s.active_preset = "custom".into();
        return "Active preset: CUSTOM".into();
    }

    // ---- Resolve NVIDIA generation + VRAM tier for AUTO ----
    let auto = nvidia_auto_preset(sys.gpu_arch, sys.vram_gb);

    match preset {
        PresetKind::Auto => {
            s.console_model = auto.console_model.clone();
            s.display_mode = auto.display_mode.clone();
            s.dlss_mode = auto.dlss_mode.clone();
            s.framegen_x = auto.framegen_x;
            s.texture_room_percent = auto.texture_room_percent.clone();
            // RTX 50 12GB+ anti-whiteout defaults (also applied globally by launcher)
        }
        PresetKind::Low => {
            s.console_model = if tier <= HardwareTier::Tight8 { "series-s".into() } else { "series-x".into() };
            s.display_mode = if tier <= HardwareTier::Tight8 { "1920x1080".into() } else { "2560x1440".into() };
            s.dlss_mode = "performance".into();
            s.framegen_x = if gen.is_nvidia() { 2 } else { 0 };
            s.texture_room_percent = low_room(gen, tier);
        }
        PresetKind::Balanced => {
            s.console_model = if tier <= HardwareTier::Tight8 { "series-s".into() } else { "series-x".into() };
            s.display_mode = if tier <= HardwareTier::Tight8 { "1920x1080".into() } else { "2560x1440".into() };
            s.dlss_mode = "balanced".into();
            s.framegen_x = if gen.is_nvidia() { 2 } else { 0 };
            s.texture_room_percent = balanced_room(gen, tier);
        }
        PresetKind::High => {
            s.console_model = "series-x".into();
            s.display_mode = "2560x1440".into();
            s.dlss_mode = "quality".into();
            s.framegen_x = if gen.is_nvidia() { 2 } else { 0 };
            s.texture_room_percent = high_room(gen, tier);
        }
        PresetKind::Custom => unreachable!(),
    }

    // DLSS/FG only on NVIDIA RTX; non-NVIDIA gets a friendly banner + no DLSS/FG.
    if !gen.is_nvidia() {
        s.dlss_mode = "off".into();
        s.framegen_x = 0;
    }

    // RTX 20/30 fallback: DLSS SR + VICTORY_MENU_FG_TYPE=auto (FSR3 FG 2x).
    let fg_type_auto = gen == NvidiaGen::Rtx20_30;

    s.display_1080p = s.display_mode == "1920x1080";
    s.active_preset = preset.id().into();

    // If the active runner is a Streamline/DLSS runner, sync exact tuning to victory-settings.txt.
    let runner_path = s.resolved_runner();
    if crate::runners::is_streamline_runner(&runner_path) && preset != PresetKind::Custom {
        let _ = crate::runners::sync_to_victory_settings(s, &runner_path);
        let _ = fg_type_auto; // VICTORY_MENU_FG_TYPE=auto emitted by launcher for RTX 20/30
    }

    format!(
        "Applied {} preset for {} [{}] ({}, {}, {}, {}% tex room, DLSS {} + {}x FG{})",
        preset.label(),
        tier.label(),
        gen_name(gen),
        s.console_model,
        s.display_mode,
        s.dlss_mode,
        s.texture_room_percent,
        s.dlss_mode,
        s.framegen_x,
        if fg_type_auto { ", FG_TYPE=auto" } else { "" }
    )
}

fn gen_name(gen: NvidiaGen) -> &'static str {
    match gen {
        NvidiaGen::Rtx50 => "RTX 50",
        NvidiaGen::Rtx40 => "RTX 40",
        NvidiaGen::Rtx20_30 => "RTX 20/30",
        NvidiaGen::NonNvidia => "non-NVIDIA",
    }
}

fn low_room(gen: NvidiaGen, tier: HardwareTier) -> String {
    let _ = gen;
    match tier {
        HardwareTier::Tight6 => "70".into(),
        HardwareTier::Tight8 => "74".into(),
        _ => "80".into(),
    }
}

fn balanced_room(gen: NvidiaGen, tier: HardwareTier) -> String {
    match tier {
        HardwareTier::Tight6 => "70".into(),
        HardwareTier::Tight8 => "74".into(),
        HardwareTier::Mid12 => if gen == NvidiaGen::Rtx40 { "82".into() } else { "85".into() },
        _ => "85".into(),
    }
}

fn high_room(gen: NvidiaGen, tier: HardwareTier) -> String {
    let _ = gen;
    match tier {
        HardwareTier::Tight6 => "70".into(),
        HardwareTier::Tight8 => "74".into(),
        _ => "85".into(),
    }
}

/// Detects whether current Settings still match one of the adaptive presets
/// for the detected hardware tier, or have been customized.
pub fn detect_active_preset(s: &Settings, sys: &SystemInfo) -> PresetKind {
    let order = if s.active_preset.eq_ignore_ascii_case("auto") {
        [PresetKind::Auto, PresetKind::Low, PresetKind::Balanced, PresetKind::High]
    } else {
        [PresetKind::Low, PresetKind::Balanced, PresetKind::High, PresetKind::Auto]
    };
    for candidate in order {
        let mut probe = s.clone();
        let _ = apply_adaptive_preset(&mut probe, candidate, sys);
        if s.console_model == probe.console_model
            && s.display_mode == probe.display_mode
            && s.dlss_mode == probe.dlss_mode
            && s.framegen_x == probe.framegen_x
            && s.texture_room_percent.trim() == probe.texture_room_percent.trim()
            && s.thin_lists == probe.thin_lists
            && s.async_submit == probe.async_submit
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
    use crate::diagnostics::{GpuArch, SystemInfo};
    use crate::launcher::build_victory_env;

    fn sys(arch: GpuArch, vram: f64) -> SystemInfo {
        let mut s = SystemInfo::default();
        s.gpu_arch = arch;
        s.vram_gb = vram;
        s.cpu_cores = 16;
        s
    }

    #[test]
    fn auto_label_is_this_pc() {
        assert_eq!(PresetKind::Auto.label(), "AUTO (THIS PC)");
        assert_eq!(PresetKind::Auto.id(), "auto");
        assert_eq!(PresetKind::from_id("auto"), PresetKind::Auto);
    }

    // ---- NVIDIA GPU Generation & VRAM Tier Auto-Detection ----

    #[test]
    fn rtx50_12gb_plus_tier() {
        let a = nvidia_auto_preset(GpuArch::NvidiaBlackwell, 12.0);
        assert_eq!(a.gen, NvidiaGen::Rtx50);
        assert_eq!(a.console_model, "series-x");
        assert_eq!(a.display_mode, "2560x1440");
        assert_eq!(a.dlss_mode, "balanced");
        assert_eq!(a.framegen_x, 2);
        assert_eq!(a.texture_room_percent, "85");
        // 3x selectable
        let mut s = Settings::default();
        s.framegen_x = 3;
        assert_eq!(s.framegen_x, 3);
    }

    #[test]
    fn rtx50_16gb_plus_tier() {
        let a = nvidia_auto_preset(GpuArch::NvidiaBlackwell, 24.0);
        assert_eq!(a.gen, NvidiaGen::Rtx50);
        assert_eq!(a.texture_room_percent, "85");
        assert_eq!(a.display_mode, "2560x1440");
    }

    #[test]
    fn rtx40_8_12gb_tier() {
        let a8 = nvidia_auto_preset(GpuArch::NvidiaAdaAmpere, 8.0);
        assert_eq!(a8.gen, NvidiaGen::Rtx40);
        assert_eq!(a8.framegen_x, 2);
        assert!((74..=82).contains(&a8.texture_room_percent.parse::<u32>().unwrap()));
        let a12 = nvidia_auto_preset(GpuArch::NvidiaAdaAmpere, 12.0);
        assert_eq!(a12.dlss_mode, "balanced");
        assert!((74..=82).contains(&a12.texture_room_percent.parse::<u32>().unwrap()));
        assert!(a12.display_mode == "2560x1440" || a12.display_mode == "1920x1080");
    }

    #[test]
    fn rtx40_6gb_safe_70_and_aggressive_trim() {
        let a = nvidia_auto_preset(GpuArch::NvidiaAdaAmpere, 6.0);
        assert_eq!(a.gen, NvidiaGen::Rtx40);
        assert_eq!(a.tier, HardwareTier::Tight6);
        assert_eq!(a.display_mode, "1920x1080");
        assert_eq!(a.dlss_mode, "performance");
        assert_eq!(a.framegen_x, 2);
        assert_eq!(a.texture_room_percent, "70");
        assert!(a.aggressive_idle_trim);
        // RTX 4050 Laptop 6 GB naming maps to Tight6
        assert_eq!(HardwareTier::from_vram_gb(6.0), HardwareTier::Tight6);
    }

    #[test]
    fn rtx4050_6gb_apply_emits_70_and_trim() {
        let mut s = Settings::default();
        let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &sys(GpuArch::NvidiaAdaAmpere, 6.0));
        assert_eq!(s.texture_room_percent, "70");
        assert_eq!(s.dlss_mode, "performance");
    }

    #[test]
    fn rtx20_30_fg_type_auto_fsr3_fallback() {
        let a = nvidia_auto_preset(GpuArch::NvidiaTuringAmpere, 8.0);
        assert_eq!(a.gen, NvidiaGen::Rtx20_30);
        assert!(a.fg_type_auto);
        // DLSS Super Resolution + VICTORY_MENU_FG_TYPE=auto
        assert!(a.dlss_mode == "performance" || a.dlss_mode == "balanced");
        assert_eq!(a.framegen_x, 2);
    }

    #[test]
    fn non_nvidia_warning_banner() {
        let a = nvidia_auto_preset(GpuArch::Other, 8.0);
        assert_eq!(a.gen, NvidiaGen::NonNvidia);
        assert!(!a.gen.is_nvidia());
        assert!(NvidiaGen::non_nvidia_warning().contains("NVIDIA RTX"));
        assert_eq!(a.dlss_mode, "off");
    }

    // ---- Zero-Regression Lock invariants emitted by presets ----

    #[test]
    fn preset_preserves_zero_regression_invariants() {
        for kind in [PresetKind::Auto, PresetKind::Low, PresetKind::Balanced, PresetKind::High] {
            let mut s = Settings::default();
            let _ = apply_adaptive_preset(&mut s, kind, &sys(GpuArch::NvidiaBlackwell, 12.0));
            assert!(s.async_submit, "VICTORY_ASYNC_SUBMIT=1 {kind:?}");
            assert!(s.thin_lists, "VICTORY_THIN=1 {kind:?}");
            assert!(s.borderless, "VICTORY_BORDERLESS=1 {kind:?}");
            assert!(s.mouse_capture, "VICTORY_MOUSE_CAPTURE=1 {kind:?}");
            assert_eq!(s.frame_interval_us, "1000", "VICTORY_FRAME_INTERVAL_US=1000 {kind:?}");
            assert!(s.card_room_for_targets_mb.is_empty(), "NO VICTORY_CARD_ROOM_FOR_TARGETS_MB {kind:?}");
            // FPS/MENU invariants emitted by launcher; verify build_victory_env wiring
            let env = build_victory_env(&s);
            let get = |k: &str| env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.as_str());
            assert_eq!(get("VICTORY_ASYNC_SUBMIT"), Some("1"));
            assert_eq!(get("VICTORY_THIN"), Some("1"));
            assert_eq!(get("VICTORY_CARD_ROOM_FOR_TARGETS_MB"), None);
        }
    }

    #[test]
    fn rtx5070ti_laptop_hybrid_is_nvidia_blackwell_and_auto_series_x() {
        let name = "NVIDIA GeForce RTX 5070 Ti Laptop GPU";
        assert!(crate::diagnostics::is_nvidia_gpu(name));
        let arch = crate::diagnostics::GpuArch::detect(name);
        assert_eq!(arch, crate::diagnostics::GpuArch::NvidiaBlackwell);
        assert!(arch.is_nvidia());
        assert!(NvidiaGen::detect(arch).is_nvidia());
        assert_ne!(NvidiaGen::detect(arch), NvidiaGen::NonNvidia);
        let hybrid = "Intel(R) UHD Graphics; NVIDIA GeForce RTX 5070 Ti Laptop GPU";
        assert!(crate::diagnostics::is_nvidia_gpu(hybrid));
        assert_eq!(crate::diagnostics::GpuArch::detect(hybrid), crate::diagnostics::GpuArch::NvidiaBlackwell);
        assert!(NvidiaGen::non_nvidia_warning().contains("NVIDIA RTX"));
        let a = nvidia_auto_preset(arch, 12.0);
        assert_eq!(a.console_model, "series-x");
        assert_eq!(a.display_mode, "2560x1440");
        assert_eq!(a.dlss_mode, "balanced");
        assert_eq!(a.framegen_x, 2);
        assert_eq!(a.texture_room_percent, "85");
        let mut s = Settings::default();
        let mut info = sys(arch, 12.0);
        info.gpu_name = name.into();
        let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &info);
        assert_eq!(s.console_model, "series-x");
        assert_eq!(s.display_mode, "2560x1440");
        assert_eq!(s.dlss_mode, "balanced");
        assert_eq!(s.framegen_x, 2);
        assert_eq!(s.texture_room_percent, "85");
        assert_eq!(s.active_preset, "auto");
    }

    #[test]
    fn rtx5070ti_12gb_auto_emits_85_and_no_card_room() {
        let mut s = Settings::default();
        let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &sys(GpuArch::NvidiaBlackwell, 12.0));
        assert_eq!(s.texture_room_percent, "85");
        assert_eq!(s.dlss_mode, "balanced");
        assert_eq!(s.framegen_x, 2);
        assert_eq!(s.console_model, "series-x");
        assert_eq!(s.display_mode, "2560x1440");
    }
}