use crate::config::Settings;
use crate::diagnostics::{GpuArch, SystemInfo};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum HardwareTier {
    Tight6,
    Tight8,
    Mid10,
    Mid12,
    High16,
    Ultra20,
}

/// Math-backed, GiB-safe VRAM tier boundaries. Windows reports a 12 GB board as ~11.9 GiB,
/// an 8 GB board as ~7.9 GiB and a 6 GB board as ~5.8-6.0 GiB, so probed GiB UNDER-reports
/// nominal capacity. These floors were validated against real 4050/5070 Ti sessions.
pub const VRAM_MIN_6GB_GIB: f64 = 5.5;
pub const VRAM_MIN_8GB_GIB: f64 = 7.0;
pub const VRAM_MIN_10GB_GIB: f64 = 9.5;
pub const VRAM_MIN_12GB_GIB: f64 = 11.0;
pub const VRAM_MIN_16GB_GIB: f64 = 13.5;
pub const VRAM_MIN_20GB_GIB: f64 = 18.5;

/// Normalise probed dedicated VRAM (GiB) onto a nominal tier size (GB).
pub fn normalize_vram_gb(vram_gb: f64) -> f64 {
    if vram_gb >= VRAM_MIN_20GB_GIB {
        20.0
    } else if vram_gb >= VRAM_MIN_16GB_GIB {
        16.0
    } else if vram_gb >= VRAM_MIN_12GB_GIB {
        12.0
    } else if vram_gb >= VRAM_MIN_10GB_GIB {
        10.0
    } else if vram_gb >= VRAM_MIN_8GB_GIB {
        8.0
    } else if vram_gb >= VRAM_MIN_6GB_GIB {
        6.0
    } else {
        vram_gb
    }
}

impl HardwareTier {
    /// Classify probed dedicated VRAM into a safe tier. 0/undetected falls back to Mid12.
    pub fn from_vram_gb(vram_gb: f64) -> Self {
        let gb = normalize_vram_gb(vram_gb);
        if gb <= 0.5 {
            HardwareTier::Mid12
        } else if gb <= 6.0 {
            HardwareTier::Tight6
        } else if gb <= 8.0 {
            HardwareTier::Tight8
        } else if gb <= 10.0 {
            HardwareTier::Mid10
        } else if gb <= 12.0 {
            HardwareTier::Mid12
        } else if gb <= 16.0 {
            HardwareTier::High16
        } else {
            HardwareTier::Ultra20
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            HardwareTier::Tight6 => "6GB TIER",
            HardwareTier::Tight8 => "8GB TIER",
            HardwareTier::Mid10 => "10GB TIER",
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

/// Math-backed NVIDIA GPU auto-tuning (`AUTO (THIS PC)`), derived from real VRAM testing.
/// Tiers: 6 GB -> 60% + LOD, 8 GB -> 68% + LOD, 10 GB -> 80%, 12 GB+ -> 85%.
pub fn nvidia_auto_preset(arch: GpuArch, vram_gb: f64) -> NvidiaAuto {
    let gen = NvidiaGen::detect(arch);
    let tier = HardwareTier::from_vram_gb(vram_gb);
    if !gen.is_nvidia() {
        return NvidiaAuto {
            gen,
            tier,
            console_model: "series-s".into(),
            display_mode: "1920x1080".into(),
            dlss_mode: "off".into(),
            framegen_x: 0,
            texture_room_percent: "70".into(),
            aggressive_idle_trim: false,
            fg_type_auto: false,
        };
    }
    let (console, res, dlss, room, aggressive) = match tier {
        HardwareTier::Tight6 => ("series-s", "1920x1080", "performance", "60", true),
        HardwareTier::Tight8 => ("series-s", "1920x1080", "performance", "68", true),
        HardwareTier::Mid10 => ("series-x", "2560x1440", "performance", "80", false),
        _ => ("series-x", "2560x1440", "balanced", "85", false),
    };
    NvidiaAuto {
        gen,
        tier,
        console_model: console.into(),
        display_mode: res.into(),
        dlss_mode: dlss.into(),
        framegen_x: 2,
        texture_room_percent: room.into(),
        aggressive_idle_trim: aggressive,
        // RTX 20/30 need the FSR3 frame-gen fallback; 40/50 use native DLSS FG.
        fg_type_auto: gen == NvidiaGen::Rtx20_30,
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
    // 6/8 GB boards stream levels on demand (LOD=1); 10/12 GB+ preload (LOD=0).
    s.levels_on_demand = tier <= HardwareTier::Tight8;
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
        HardwareTier::Tight6 => "60".into(),
        HardwareTier::Tight8 => "68".into(),
        HardwareTier::Mid10 => "80".into(),
        _ => "85".into(),
    }
}

fn balanced_room(gen: NvidiaGen, tier: HardwareTier) -> String {
    let _ = gen;
    match tier {
        HardwareTier::Tight6 => "60".into(),
        HardwareTier::Tight8 => "68".into(),
        HardwareTier::Mid10 => "80".into(),
        _ => "85".into(),
    }
}

fn high_room(gen: NvidiaGen, tier: HardwareTier) -> String {
    let _ = gen;
    match tier {
        HardwareTier::Tight6 => "60".into(),
        HardwareTier::Tight8 => "68".into(),
        HardwareTier::Mid10 => "80".into(),
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
        assert_eq!(a8.texture_room_percent, "68");
        assert_eq!(a8.console_model, "series-s");
        assert_eq!(a8.display_mode, "1920x1080");
        assert_eq!(a8.dlss_mode, "performance");
        let a12 = nvidia_auto_preset(GpuArch::NvidiaAdaAmpere, 12.0);
        assert_eq!(a12.dlss_mode, "balanced");
        assert_eq!(a12.texture_room_percent, "85");
        assert_eq!(a12.display_mode, "2560x1440");
        assert_eq!(a12.console_model, "series-x");
    }

    #[test]
    fn rtx40_6gb_safe_60_and_aggressive_trim() {
        let a = nvidia_auto_preset(GpuArch::NvidiaAdaAmpere, 6.0);
        assert_eq!(a.gen, NvidiaGen::Rtx40);
        assert_eq!(a.tier, HardwareTier::Tight6);
        assert_eq!(a.display_mode, "1920x1080");
        assert_eq!(a.dlss_mode, "performance");
        assert_eq!(a.framegen_x, 2);
        assert_eq!(a.texture_room_percent, "60");
        assert!(a.aggressive_idle_trim);
        // RTX 4050 Laptop 6 GB naming maps to Tight6
        assert_eq!(HardwareTier::from_vram_gb(6.0), HardwareTier::Tight6);
    }

    #[test]
    fn rtx4050_6gb_apply_emits_60_and_trim() {
        let mut s = Settings::default();
        let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &sys(GpuArch::NvidiaAdaAmpere, 6.0));
        assert_eq!(s.texture_room_percent, "60");
        assert_eq!(s.dlss_mode, "performance");
        assert!(s.levels_on_demand);
    }

    /// Math-backed universal tiering: 6 -> 60%+LOD, 8 -> 68%+LOD, 10 -> 80%, 11.9 -> 85%+no LOD.
    #[test]
    fn universal_nvidia_vram_tiers() {
        let cases: [(f64, &str, &str, &str, &str, bool); 4] = [
            (6.0, "60", "series-s", "1920x1080", "performance", true),
            (8.0, "68", "series-s", "1920x1080", "performance", true),
            (10.0, "80", "series-x", "2560x1440", "performance", false),
            (11.9, "85", "series-x", "2560x1440", "balanced", false),
        ];
        for (vram, room, console, res, dlss, lod) in cases {
            let a = nvidia_auto_preset(GpuArch::NvidiaBlackwell, vram);
            assert_eq!(a.texture_room_percent, room, "room @ {vram} GB");
            assert_eq!(a.console_model, console, "console @ {vram} GB");
            assert_eq!(a.display_mode, res, "resolution @ {vram} GB");
            assert_eq!(a.dlss_mode, dlss, "DLSS @ {vram} GB");
            assert_eq!(a.framegen_x, 2, "FG @ {vram} GB");
            let mut s = Settings::default();
            let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &sys(GpuArch::NvidiaBlackwell, vram));
            assert_eq!(s.levels_on_demand, lod, "LEVELS_ON_DEMAND @ {vram} GB");
        }
        assert_eq!(HardwareTier::from_vram_gb(11.9), HardwareTier::Mid12);
        assert_eq!(HardwareTier::from_vram_gb(10.0), HardwareTier::Mid10);
        assert_eq!(HardwareTier::from_vram_gb(8.0), HardwareTier::Tight8);
        assert_eq!(HardwareTier::from_vram_gb(6.0), HardwareTier::Tight6);
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
        let a = nvidia_auto_preset(arch, 11.9);
        assert_eq!(a.console_model, "series-x");
        assert_eq!(a.display_mode, "2560x1440");
        assert_eq!(a.dlss_mode, "balanced");
        assert_eq!(a.framegen_x, 2);
        assert_eq!(a.texture_room_percent, "85");
        let mut s = Settings::default();
        let mut info = sys(arch, 11.9);
        info.gpu_name = name.into();
        let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &info);
        assert_eq!(s.console_model, "series-x");
        assert_eq!(s.display_mode, "2560x1440");
        assert_eq!(s.dlss_mode, "balanced");
        assert_eq!(s.framegen_x, 2);
        assert_eq!(s.texture_room_percent, "85");
        assert_eq!(s.active_preset, "auto");
        assert!(!s.levels_on_demand);
    }

    #[test]
    fn rtx5070ti_12gb_auto_emits_85_and_no_card_room() {
        let mut s = Settings::default();
        let _ = apply_adaptive_preset(&mut s, PresetKind::Auto, &sys(GpuArch::NvidiaBlackwell, 11.9));
        assert_eq!(s.texture_room_percent, "85");
        assert_eq!(s.dlss_mode, "balanced");
        assert_eq!(s.framegen_x, 2);
        assert_eq!(s.console_model, "series-x");
        assert_eq!(s.display_mode, "2560x1440");
        assert!(!s.levels_on_demand);
    }
}