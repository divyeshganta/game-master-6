use std::fs;
use std::path::{Path, PathBuf};

use crate::runners::RunnerKind;

#[derive(Clone, Debug)]
pub struct Settings {
    pub exe: String,
    pub game_dir: String,
    pub runner: String,
    pub runner_kind: String, // "auto" or RunnerKind::id()
    pub console_model: String,
    pub title_args: String,
    pub log_level: String,
    pub capture: bool,
    pub gpu_timing: bool,
    pub dred: bool,
    pub warp: bool,
    pub custom_env: String,
    pub skip_pipelines: String,
    pub upscaler_dir: String,
    pub upscaler_enabled: bool,
    pub shader_loop_limit: String,
    pub start_maximized: bool,
    pub zero_overhead: bool,
    pub prefer_dgpu: bool,
    pub high_priority: bool,
    pub isolated_cache: bool,
    pub offline_mode: bool,
    pub theme: String,
    // v6.0 performance & stability tuning
    pub card_buffers_min_mb: String, // "", "1", "4", "8", "16" -> VICTORY_CARD_BUFFERS_MIN_MB
    pub card_rehash_runs: bool,      // VICTORY_CARD_REHASH_RUNS=1
    pub join_copies: bool,           // VICTORY_JOIN_COPIES=1
    pub timer_1ms: bool,             // Win32 timeBeginPeriod(1) while running
    pub auto_clean_strays: bool,     // Kill stray GameRunner/victory processes before launch
    pub high_perf_power: bool,       // Activate Windows High/Ultimate Performance power plan on launch
    pub hud_overlay: bool,           // Click-through always-on-top in-game FPS/RAM HUD
    pub auto_backup_saves: bool,     // Auto-backup saves before launch
    pub hud_corner: u8,              // 0=TL, 1=TR, 2=BL, 3=BR
    // v10.1 input & ztrx engine settings
    pub input_mode: String,          // "auto", "pad", "kbm" -> VICTORY_INPUT_MODE
    pub keymap: String,              // Custom keyboard stand-in bindings -> VICTORY_KEYMAP
    pub relay_mode: u8,              // 0=OFF, 1=RELAY 1, 2=RELAY 2 -> VICTORY_RELAY
    pub wide_back: bool,             // VICTORY_WIDE_BACK=1
    pub pipeline_wait_ms: u32,       // VICTORY_PIPELINE_WAIT_MS (default 24)
    pub bvh_builder: bool,           // VICTORY_BVH_BUILDER=1 (experimental ray-tracing BVH)
    pub mouse_aim: bool,             // VICTORY_MOUSE_AIM=1 (Mouse camera + LMB/RMB triggers)
    pub mouse_sens: String,          // VICTORY_MOUSE_SENS (0.6, 1.0, 1.5, 2.0)
    pub mouse_lock: bool,            // VICTORY_MOUSE_LOCK=1 (360-deg cursor centering; F8 pauses)
    pub async_submit: bool,
    pub queue2: bool,
    pub indirect_window: bool,
    pub texture_no_wait: bool,
    pub merge_passes: bool,
    // v1.4 Apex FPS & Open-World KBM suite
    pub display_1080p: bool,
    pub wide_refresh: bool,
    pub wide_merge: bool,
    pub inplace_buf: bool,
    pub kbm_crosshair: bool,
    pub profiler_on: bool,
    pub ambient_fill: bool,
    pub hoist_clamps: bool,
    pub fast_cores: bool,
    // v1.4 studio launcher & runner options
    pub thin_lists: bool,                // VICTORY_THIN=1 (opt-in parallel list translation)
    pub display_mode: String,            // "auto", "1920x1080", "2560x1440", "3840x2160"
    pub borderless: bool,                // Borderless fullscreen vs windowed
    pub queue2_landing_mb: String,       // "48", "96", "128" -> VICTORY_QUEUE2_LANDING_MB
    pub ambient_shade: String,           // VICTORY_AMBIENT_SHADE
    pub last_prepared_runner_sig: String,// "size:mtime" of runner that last prepared cache
    // v1.4 hardware-adaptive & verified BASELINE residency/pipeline settings
    pub active_preset: String,           // "low", "balanced", "high", "custom"
    pub card_buffers_mb: String,         // "", "256", "512", "1024", "2048", "4096" -> VICTORY_CARD_BUFFERS_MB
    pub texture_room_percent: String,    // "88".."97" -> VICTORY_TEXTURE_ROOM_PERCENT
    pub card_room_for_targets_mb: String,// "0", "2048", "2560", "3072" -> VICTORY_CARD_ROOM_FOR_TARGETS_MB
    pub texture_budget_mb: String,       // "" (auto) or explicit MiB -> VICTORY_TEXTURE_BUDGET_MB
    pub texture_stage_mb: String,        // "" (auto) or explicit MiB -> VICTORY_TEXTURE_STAGE_MB
    pub card_steps: u8,                  // 0, 1, 2 -> VICTORY_CARD_STEPS
    pub relay_copy: bool,                // VICTORY_RELAY_COPY=1
    pub indirect_batch: u8,              // 1, 2, 4 -> VICTORY_INDIRECT_BATCH
    pub map_threads: bool,               // VICTORY_MAP_THREADS=1
    pub card_awake: bool,                // VICTORY_CARD_AWAKE=1
    pub show_after: bool,                // VICTORY_SHOW_AFTER=1
    pub join_parts: u8,                  // 4, 8, 16 -> VICTORY_JOIN_PARTS
    pub upload_ahead: bool,              // VICTORY_UPLOAD_AHEAD=1
    pub levels_on_demand: bool,          // VICTORY_LEVELS_ON_DEMAND=1
    pub no_wait_resident: bool,          // VICTORY_WAIT_RESIDENT=0 (skip map/residency stalls)
    pub ambient_val: String,             // "off", "0.6", "0.8" -> VICTORY_AMBIENT
    pub ambient_air: String,             // "0", "1.0", "1.2", "1.5" -> VICTORY_AMBIENT_AIR
    // v1.4 universal HUD density & keymap standard
    pub hud_mode: u8,                    // 0=Compact, 1=Standard+FrametimeGraph, 2=Minimal
    pub keymap_preset: String,           // "open-world", "fps", "left-handed", "pad-only"
    pub ab_experiment: String,           // "off", "async-submit", "thin", "wide-refresh", "upload-ahead" -> VICTORY_AB
    pub runner_env_overrides: Vec<(String, String)>, // per-runner KEY=VAL;... overrides keyed by runner_slug
    pub win_x: i32,
    pub win_y: i32,
    pub win_w: i32,
    pub win_h: i32,
    pub pinned_baseline_runner: String,
    pub pinned_baseline_locked: bool,
    pub dlss_mode: String,               // "off", "performance", "balanced", "quality"
    pub framegen_x: u32,                 // 0=off, 2..=6
    pub frame_interval_us: String,       // e.g. "50000" (20 fps base -> 60 fps FG)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            exe: String::new(),
            game_dir: String::new(),
            runner: r"runners\baseline\victory.exe".into(),
            runner_kind: "auto".into(),
            console_model: "series-s".into(),
            title_args: String::new(),
            log_level: "warn".into(),
            capture: false,
            gpu_timing: false,
            dred: false,
            warp: false,
            custom_env: String::new(),
            skip_pipelines: String::new(),
            upscaler_dir: String::new(),
            upscaler_enabled: false,
            shader_loop_limit: String::new(),
            start_maximized: true,
            zero_overhead: true,
            prefer_dgpu: true,
            high_priority: true,
            isolated_cache: false,
            offline_mode: false,
            theme: "vice".into(),
            // v1.4: Hardware-adaptive BALANCED baseline defaults (12 GB sweet spot)
            card_buffers_min_mb: "4".into(),
            card_rehash_runs: true,
            join_copies: true,
            timer_1ms: true,
            auto_clean_strays: true,
            high_perf_power: false,
            hud_overlay: false,
            auto_backup_saves: true,
            hud_corner: 0,
            // Controller-first native PC default. KBM stays dormant (opt-in in INPUT).
            input_mode: "pad".into(),
            keymap: Self::default_keymap_string(),
            relay_mode: 2,
            wide_back: true,
            pipeline_wait_ms: 24,
            bvh_builder: false,
            mouse_aim: false,
            mouse_sens: "1.0".into(),
            mouse_lock: false,
            async_submit: true,
            queue2: true,
            indirect_window: true,
            texture_no_wait: true,
            merge_passes: true,
            display_1080p: false,
            wide_refresh: true,
            wide_merge: true,
            inplace_buf: false,
            // No floating reticle over the game window by default.
            kbm_crosshair: false,
            profiler_on: false,
            ambient_fill: true,
            hoist_clamps: true,
            fast_cores: true,
            thin_lists: false,
            display_mode: "1920x1080".into(),
            borderless: true,
            queue2_landing_mb: "96".into(),
            ambient_shade: "1.0,0.6,0.1,0.45".into(),
            last_prepared_runner_sig: String::new(),
            active_preset: "balanced".into(),
            card_buffers_mb: "1024".into(),
            texture_room_percent: "94".into(),
            card_room_for_targets_mb: "2560".into(),
            texture_budget_mb: String::new(),
            texture_stage_mb: String::new(),
            card_steps: 2,
            relay_copy: true,
            indirect_batch: 2,
            map_threads: true,
            card_awake: true,
            show_after: true,
            join_parts: 8,
            upload_ahead: true,
            levels_on_demand: true,
            no_wait_resident: true,
            ambient_val: "0.3".into(),
            ambient_air: "1.2".into(),
            hud_mode: 1,
            keymap_preset: "open-world".into(),
            ab_experiment: "off".into(),
            runner_env_overrides: Vec::new(),
            win_x: -1,
            win_y: -1,
            win_w: 1360,
            win_h: 820,
            pinned_baseline_runner: String::new(),
            pinned_baseline_locked: false,
            dlss_mode: "quality".into(),
            framegen_x: 3,
            frame_interval_us: String::new(),
        }
    }
}

impl Settings {
    pub fn app_dir() -> PathBuf {
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("GameMaster.exe"))
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }

    pub fn path() -> PathBuf {
        Self::app_dir().join("GameMaster.settings")
    }

    pub fn load() -> Self {
        let mut s = Self::default();
        let Ok(text) = fs::read_to_string(Self::path()) else {
            return s;
        };
        let mut explicit_upscaler_enabled: Option<bool> = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let v = value.trim();
                match key.trim() {
                    "exe" => s.exe = v.to_string(),
                    "game_dir" => s.game_dir = v.to_string(),
                    "runner" => {
                        if !v.is_empty() {
                            s.runner = v.to_string();
                        }
                    }
                    "runner_kind" => {
                        if !v.is_empty() {
                            s.runner_kind = v.to_string();
                        }
                    }
                    "console" => {
                        s.console_model = if v.eq_ignore_ascii_case("series-s") {
                            "series-s".into()
                        } else {
                            "series-x".into()
                        };
                    }
                    "title_args" => s.title_args = v.to_string(),
                    "log_level" => s.log_level = normalize_log_level(v),
                    "capture" => s.capture = parse_bool(v),
                    "gpu_timing" => s.gpu_timing = parse_bool(v),
                    "dred" => s.dred = parse_bool(v),
                    "warp" => s.warp = parse_bool(v),
                    "custom_env" => s.custom_env = v.to_string(),
                    "skip_pipelines" => s.skip_pipelines = v.to_string(),
                    "upscaler_dir" => s.upscaler_dir = v.to_string(),
                    "upscaler_enabled" => explicit_upscaler_enabled = Some(parse_bool(v)),
                    "shader_loop_limit" => s.shader_loop_limit = v.to_string(),
                    "start_maximized" => s.start_maximized = parse_bool(v),
                    "zero_overhead" => s.zero_overhead = parse_bool(v),
                    "prefer_dgpu" => s.prefer_dgpu = parse_bool(v),
                    "high_priority" => s.high_priority = parse_bool(v),
                    "isolated_cache" => s.isolated_cache = parse_bool(v),
                    "offline_mode" => s.offline_mode = parse_bool(v),
                    "theme" => s.theme = v.to_ascii_lowercase(),
                    "card_buffers_min_mb" => s.card_buffers_min_mb = v.to_string(),
                    "card_rehash_runs" => s.card_rehash_runs = parse_bool(v),
                    "join_copies" => s.join_copies = parse_bool(v),
                    "timer_1ms" => s.timer_1ms = parse_bool(v),
                    "auto_clean_strays" => s.auto_clean_strays = parse_bool(v),
                    "high_perf_power" => s.high_perf_power = parse_bool(v),
                    "hud_overlay" => s.hud_overlay = parse_bool(v),
                    "auto_backup_saves" => s.auto_backup_saves = parse_bool(v),
                    "hud_corner" => s.hud_corner = v.parse::<u8>().unwrap_or(0) % 4,
                    "input_mode" => {
                        if !v.is_empty() {
                            s.input_mode = v.to_ascii_lowercase();
                        }
                    }
                    "keymap" => {
                        if !v.is_empty() {
                            s.keymap = v.to_string();
                        }
                    }
                    "relay_mode" => s.relay_mode = v.parse::<u8>().unwrap_or(2).min(2),
                    "wide_back" => s.wide_back = parse_bool(v),
                    "pipeline_wait_ms" => s.pipeline_wait_ms = v.parse::<u32>().unwrap_or(24),
                    "bvh_builder" => s.bvh_builder = parse_bool(v),
                    "mouse_aim" => s.mouse_aim = parse_bool(v),
                    "mouse_sens" => {
                        if !v.is_empty() {
                            s.mouse_sens = v.to_string();
                        }
                    }
                    "mouse_lock" => s.mouse_lock = parse_bool(v),
                    "async_submit" => s.async_submit = parse_bool(v),
                    "queue2" => s.queue2 = parse_bool(v),
                    "indirect_window" => s.indirect_window = parse_bool(v),
                    "texture_no_wait" => s.texture_no_wait = parse_bool(v),
                    "merge_passes" => s.merge_passes = parse_bool(v),
                    "display_1080p" => s.display_1080p = parse_bool(v),
                    "wide_refresh" => s.wide_refresh = parse_bool(v),
                    "wide_merge" => s.wide_merge = parse_bool(v),
                    "inplace_buf" => s.inplace_buf = parse_bool(v),
                    "kbm_crosshair" => s.kbm_crosshair = parse_bool(v),
                    "profiler_on" => s.profiler_on = parse_bool(v),
                    "ambient_fill" => s.ambient_fill = parse_bool(v),
                    "hoist_clamps" => s.hoist_clamps = parse_bool(v),
                    "fast_cores" => s.fast_cores = parse_bool(v),
                    "thin_lists" => s.thin_lists = parse_bool(v),
                    "display_mode" => if !v.trim().is_empty() { s.display_mode = v.trim().to_string(); },
                    "borderless" => s.borderless = parse_bool(v),
                    "queue2_landing_mb" => s.queue2_landing_mb = v.trim().to_string(),
                    "ambient_shade" => if !v.trim().is_empty() { s.ambient_shade = v.trim().to_string(); },
                    "last_prepared_runner_sig" => s.last_prepared_runner_sig = v.trim().to_string(),
                    "active_preset" => if !v.trim().is_empty() { s.active_preset = v.trim().to_ascii_lowercase(); },
                    "card_buffers_mb" => s.card_buffers_mb = v.trim().to_string(),
                    "texture_room_percent" => s.texture_room_percent = v.trim().to_string(),
                    "card_room_for_targets_mb" => s.card_room_for_targets_mb = v.trim().to_string(),
                    "texture_budget_mb" => s.texture_budget_mb = v.trim().to_string(),
                    "texture_stage_mb" => s.texture_stage_mb = v.trim().to_string(),
                    "card_steps" => s.card_steps = v.trim().parse::<u8>().unwrap_or(2).min(2),
                    "relay_copy" => s.relay_copy = parse_bool(v),
                    "indirect_batch" => s.indirect_batch = v.trim().parse::<u8>().unwrap_or(2).clamp(1, 4),
                    "map_threads" => s.map_threads = parse_bool(v),
                    "card_awake" => s.card_awake = parse_bool(v),
                    "show_after" => s.show_after = parse_bool(v),
                    "join_parts" => s.join_parts = v.trim().parse::<u8>().unwrap_or(8).clamp(1, 32),
                    "upload_ahead" => s.upload_ahead = parse_bool(v),
                    "levels_on_demand" => s.levels_on_demand = parse_bool(v),
                    "no_wait_resident" => s.no_wait_resident = parse_bool(v),
                    "ambient_val" => if !v.trim().is_empty() { s.ambient_val = v.trim().to_string(); },
                    "ambient_air" => s.ambient_air = v.trim().to_string(),
                    "hud_mode" => s.hud_mode = v.trim().parse::<u8>().unwrap_or(1) % 3,
                    "keymap_preset" => if !v.trim().is_empty() { s.keymap_preset = v.trim().to_ascii_lowercase(); },
                    "ab_experiment" => if !v.trim().is_empty() { s.ab_experiment = v.trim().to_ascii_lowercase(); },
                    "win_x" => s.win_x = v.trim().parse().unwrap_or(-1),
                    "win_y" => s.win_y = v.trim().parse().unwrap_or(-1),
                    "win_w" => s.win_w = v.trim().parse().unwrap_or(1360).clamp(1080, 3840),
                    "win_h" => s.win_h = v.trim().parse().unwrap_or(820).clamp(680, 2160),
                    "pinned_baseline_runner" => s.pinned_baseline_runner = v.trim().to_string(),
                    "pinned_baseline_locked" => s.pinned_baseline_locked = parse_bool(v),
                    "dlss_mode" => if !v.trim().is_empty() { s.dlss_mode = v.trim().to_ascii_lowercase(); },
                    "framegen_x" => s.framegen_x = v.trim().parse::<u32>().unwrap_or(0).clamp(0, 6),
                    "frame_interval_us" => s.frame_interval_us = v.trim().to_string(),
                    other => {
                        if let Some(id) = other.strip_prefix("runner_env.") {
                            let id_clean = id.trim().to_ascii_lowercase();
                            if !id_clean.is_empty() && !v.trim().is_empty() {
                                s.set_runner_env_for(&id_clean, v.trim());
                            }
                        }
                    }
                }
            }
        }
        s.upscaler_enabled = explicit_upscaler_enabled.unwrap_or(!s.upscaler_dir.trim().is_empty());

        if !s.resolved_runner().is_file() {
            let beside = Self::app_dir().join("GameRunner.exe");
            if beside.is_file() {
                s.runner = "GameRunner.exe".into();
            }
        }
        s
    }

    pub fn save(&self) -> Result<(), String> {
        let mut text = format!(
            "# Game Master 6 v1.4 settings\n\
             exe={}\n\
             game_dir={}\n\
             runner={}\n\
             runner_kind={}\n\
             console={}\n\
             title_args={}\n\
             log_level={}\n\
             capture={}\n\
             gpu_timing={}\n\
             dred={}\n\
             warp={}\n\
             custom_env={}\n\
             skip_pipelines={}\n\
             upscaler_dir={}\n\
             upscaler_enabled={}\n\
             shader_loop_limit={}\n\
             start_maximized={}\n\
             zero_overhead={}\n\
             prefer_dgpu={}\n\
             high_priority={}\n\
             isolated_cache={}\n\
             offline_mode={}\n\
             theme={}\n\
             card_buffers_min_mb={}\n\
             card_rehash_runs={}\n\
             join_copies={}\n\
             timer_1ms={}\n\
             auto_clean_strays={}\n\
             high_perf_power={}\n\
             hud_overlay={}\n\
             auto_backup_saves={}\n\
             hud_corner={}\n\
             input_mode={}\n\
             keymap={}\n\
             relay_mode={}\n\
             wide_back={}\n\
             pipeline_wait_ms={}\n\
             bvh_builder={}\n\
             mouse_aim={}\n\
             mouse_sens={}\n\
             mouse_lock={}\n\
             async_submit={}\n\
             queue2={}\n\
             indirect_window={}\n\
             texture_no_wait={}\n\
             merge_passes={}\n\
             display_1080p={}\n\
             wide_refresh={}\n\
             wide_merge={}\n\
             inplace_buf={}\n\
             kbm_crosshair={}\n\
             profiler_on={}\n\
             ambient_fill={}\n\
             hoist_clamps={}\n\
             fast_cores={}\n\
             thin_lists={}\n\
             display_mode={}\n\
             borderless={}\n\
             queue2_landing_mb={}\n\
             ambient_shade={}\n\
             last_prepared_runner_sig={}\n\
             active_preset={}\n\
             card_buffers_mb={}\n\
             texture_room_percent={}\n\
             card_room_for_targets_mb={}\n\
             texture_budget_mb={}\n\
             texture_stage_mb={}\n\
             card_steps={}\n\
             relay_copy={}\n\
             indirect_batch={}\n\
             map_threads={}\n\
             card_awake={}\n\
             show_after={}\n\
             join_parts={}\n\
             upload_ahead={}\n\
             levels_on_demand={}\n\
             no_wait_resident={}\n\
             ambient_val={}\n\
             ambient_air={}\n\
             hud_mode={}\n\
             keymap_preset={}\n\
             ab_experiment={}\n\
             win_x={}\n\
             win_y={}\n\
             win_w={}\n\
             win_h={}\n\
             pinned_baseline_runner={}\n\
             pinned_baseline_locked={}\n\
             dlss_mode={}\n\
             framegen_x={}\n\
             frame_interval_us={}\n",
            self.exe.trim(),
            self.game_dir.trim(),
            self.runner.trim(),
            self.runner_kind.trim(),
            self.console_model.trim(),
            self.title_args.trim(),
            self.log_level.trim(),
            bstr(self.capture),
            bstr(self.gpu_timing),
            bstr(self.dred),
            bstr(self.warp),
            self.custom_env.trim(),
            self.skip_pipelines.trim(),
            self.upscaler_dir.trim(),
            bstr(self.upscaler_enabled),
            self.shader_loop_limit.trim(),
            bstr(self.start_maximized),
            bstr(self.zero_overhead),
            bstr(self.prefer_dgpu),
            bstr(self.high_priority),
            bstr(self.isolated_cache),
            bstr(self.offline_mode),
            self.theme.trim(),
            self.card_buffers_min_mb.trim(),
            bstr(self.card_rehash_runs),
            bstr(self.join_copies),
            bstr(self.timer_1ms),
            bstr(self.auto_clean_strays),
            bstr(self.high_perf_power),
            bstr(self.hud_overlay),
            bstr(self.auto_backup_saves),
            self.hud_corner,
            self.input_mode.trim(),
            self.keymap.trim(),
            self.relay_mode,
            bstr(self.wide_back),
            self.pipeline_wait_ms,
            bstr(self.bvh_builder),
            bstr(self.mouse_aim),
            self.mouse_sens.trim(),
            bstr(self.mouse_lock),
            bstr(self.async_submit),
            bstr(self.queue2),
            bstr(self.indirect_window),
            bstr(self.texture_no_wait),
            bstr(self.merge_passes),
            bstr(self.display_1080p),
            bstr(self.wide_refresh),
            bstr(self.wide_merge),
            bstr(self.inplace_buf),
            bstr(self.kbm_crosshair),
            bstr(self.profiler_on),
            bstr(self.ambient_fill),
            bstr(self.hoist_clamps),
            bstr(self.fast_cores),
            bstr(self.thin_lists),
            self.display_mode.trim(),
            bstr(self.borderless),
            self.queue2_landing_mb.trim(),
            self.ambient_shade.trim(),
            self.last_prepared_runner_sig.trim(),
            self.active_preset.trim(),
            self.card_buffers_mb.trim(),
            self.texture_room_percent.trim(),
            self.card_room_for_targets_mb.trim(),
            self.texture_budget_mb.trim(),
            self.texture_stage_mb.trim(),
            self.card_steps,
            bstr(self.relay_copy),
            self.indirect_batch,
            bstr(self.map_threads),
            bstr(self.card_awake),
            bstr(self.show_after),
            self.join_parts,
            bstr(self.upload_ahead),
            bstr(self.levels_on_demand),
            bstr(self.no_wait_resident),
            self.ambient_val.trim(),
            self.ambient_air.trim(),
            self.hud_mode,
            self.keymap_preset.trim(),
            self.ab_experiment.trim(),
            self.win_x,
            self.win_y,
            self.win_w,
            self.win_h,
            self.pinned_baseline_runner.trim(),
            bstr(self.pinned_baseline_locked),
            self.dlss_mode.trim(),
            self.framegen_x,
            self.frame_interval_us.trim(),
        );
        for (id, val) in &self.runner_env_overrides {
            if !id.trim().is_empty() && !val.trim().is_empty() {
                text.push_str(&format!("runner_env.{}={}\n", id.trim(), val.trim()));
            }
        }
        let dest = Self::path();
        let tmp = Self::app_dir().join("GameMaster.settings.tmp");
        if fs::write(&tmp, &text).is_ok() {
            if fs::rename(&tmp, &dest).is_ok() {
                return Ok(());
            }
            let _ = fs::remove_file(&tmp);
        }
        fs::write(&dest, text).map_err(|e| format!("Failed to save settings: {e}"))
    }

    pub fn runner_signature(&self) -> String {
        let p = self.resolved_runner();
        let Ok(meta) = fs::metadata(&p) else {
            return String::new();
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!(
            "{}:{}:{}",
            p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            meta.len(),
            mtime
        )
    }

    pub fn cache_needs_refresh(&self, cache_ready: bool) -> bool {
        if !cache_ready {
            return true;
        }
        let sig = self.runner_signature();
        !self.last_prepared_runner_sig.is_empty() && !sig.is_empty() && self.last_prepared_runner_sig != sig
    }

    pub fn display_mode_label(&self) -> &'static str {
        match self.display_mode.trim() {
            "auto" => "Auto (Native)",
            "1280x720" => "1280 × 720",
            "1600x900" => "1600 × 900",
            "2560x1440" => "2560 × 1440",
            "3840x2160" => "3840 × 2160",
            _ => "1920 × 1080",
        }
    }

    pub fn dlss_mode_label(&self) -> &'static str {
        match self.dlss_mode.trim().to_ascii_lowercase().as_str() {
            "off" => "DLSS: OFF",
            "performance" => "DLSS: PERF",
            "balanced" => "DLSS: BAL",
            "dlaa" => "DLSS: DLAA",
            _ => "DLSS: QUAL",
        }
    }

    pub fn framegen_label(&self) -> String {
        if self.framegen_x == 0 || self.dlss_mode.trim().eq_ignore_ascii_case("off") {
            "FG: OFF".into()
        } else {
            format!("FG: {}X", self.framegen_x)
        }
    }

    pub fn cycle_dlss_mode(&mut self) {
        self.dlss_mode = match self.dlss_mode.trim().to_ascii_lowercase().as_str() {
            "off" => "performance",
            "performance" => "balanced",
            "balanced" => "quality",
            _ => "off",
        }
        .into();
        if self.dlss_mode == "off" {
            self.framegen_x = 0;
        }
    }

    pub fn cycle_framegen_x(&mut self, max_x: u32) {
        let cap = max_x.clamp(2, 6);
        self.framegen_x = match self.framegen_x {
            0 => 2,
            x if x < cap => x + 1,
            _ => 0,
        };
        if self.framegen_x > 0 && self.dlss_mode.trim().eq_ignore_ascii_case("off") {
            self.dlss_mode = "quality".into();
        }
    }

    pub fn resolved_runner(&self) -> PathBuf {
        let p = Path::new(self.runner.trim());
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            Self::app_dir().join(p)
        }
    }

    pub fn active_runner_kind(&self) -> RunnerKind {
        if let Some(k) = RunnerKind::from_id(&self.runner_kind) {
            k
        } else {
            RunnerKind::detect_from_path(&self.resolved_runner())
        }
    }

    pub fn effective_victory_log(&self) -> &'static str {
        match self.log_level.as_str() {
            "warn" => "warn",
            "diagnostics" => "debug",
            "trace" => "victory_gpu=trace,victory_shader=trace,info",
            _ => "info",
        }
    }

    pub fn is_series_s(&self) -> bool {
        self.console_model.eq_ignore_ascii_case("series-s")
    }

    pub fn version_label(&self) -> String {
        version_from(&self.game_dir)
    }

    pub fn cycle_card_buffers(&mut self) -> &'static str {
        match self.card_buffers_min_mb.trim() {
            "1" => {
                self.card_buffers_min_mb = "4".into();
                "4 MB"
            }
            "4" => {
                self.card_buffers_min_mb = "8".into();
                "8 MB"
            }
            "8" => {
                self.card_buffers_min_mb = "16".into();
                "16 MB"
            }
            "16" => {
                self.card_buffers_min_mb.clear();
                "OFF"
            }
            _ => {
                self.card_buffers_min_mb = "1".into();
                "1 MB"
            }
        }
    }

    pub fn card_buffers_label(&self) -> String {
        let v = self.card_buffers_min_mb.trim();
        if v.is_empty() {
            "OFF".into()
        } else {
            format!("{v} MB")
        }
    }

    pub fn cycle_texture_room(&mut self) -> String {
        const STEPS: [&str; 6] = ["75", "80", "82", "85", "90", "94"];
        let cur = self.texture_room_percent.trim();
        let next = match STEPS.iter().position(|&s| s == cur) {
            Some(i) => STEPS[(i + 1) % STEPS.len()],
            None => "82",
        };
        self.texture_room_percent = next.to_string();
        format!("{next}%")
    }

    pub fn texture_room_label(&self) -> String {
        let v = self.texture_room_percent.trim();
        if v.is_empty() {
            "85%".into()
        } else {
            format!("{v}%")
        }
    }
}

fn bstr(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

fn parse_bool(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn normalize_log_level(v: &str) -> String {
    let lower = v.to_ascii_lowercase();
    if lower.contains("victory_gpu") || lower == "trace" {
        "trace".into()
    } else if lower == "diagnostics" || lower == "debug" {
        "diagnostics".into()
    } else if lower == "warn" || lower == "warning" {
        "warn".into()
    } else {
        "info".into()
    }
}

pub fn version_from(path: &str) -> String {
    let bytes = path.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        if (bytes[i] == b'.' || bytes[i] == b'-' || bytes[i] == b'_' || bytes[i] == b' ')
            && (bytes[i + 1] == b'v' || bytes[i + 1] == b'V')
        {
            let mut end = i + 2;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
                end += 1;
            }
            if end > i + 2 {
                return path[i + 1..end].to_string();
            }
        }
    }
    "LOCAL BUILD".to_string()
}

pub fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in input.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    args.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        args.push(cur);
    }
    args
}

impl Settings {
    pub fn skipped_hashes(&self) -> Vec<String> {
        self.skip_pipelines
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .collect()
    }

    pub fn is_hash_skipped(&self, hash: &str) -> bool {
        let h = hash.to_ascii_lowercase();
        self.skipped_hashes().iter().any(|x| x == &h)
    }

    pub fn toggle_skip_hash(&mut self, hash: &str) -> bool {
        let h = hash.to_ascii_lowercase();
        let mut list = self.skipped_hashes();
        let added = if let Some(pos) = list.iter().position(|x| x == &h) {
            list.remove(pos);
            false
        } else {
            list.push(h);
            true
        };
        self.skip_pipelines = list.join(",");
        added
    }

    pub fn cycle_theme(&mut self) -> &'static str {
        match self.theme.as_str() {
            "vice" => {
                self.theme = "sunset".into();
                "LEONIDA SUNSET"
            }
            "sunset" => {
                self.theme = "carbon".into();
                "STEALTH CARBON"
            }
            "carbon" => {
                self.theme = "midnight".into();
                "MIDNIGHT CYAN"
            }
            _ => {
                self.theme = "vice".into();
                "VICE NEON"
            }
        }
    }

    pub fn theme_label(&self) -> &'static str {
        match self.theme.as_str() {
            "sunset" => "THEME: SUNSET",
            "carbon" => "THEME: CARBON",
            "midnight" => "THEME: MIDNIGHT",
            _ => "THEME: VICE",
        }
    }
}

impl Settings {
    pub fn profiles_dir() -> PathBuf {
        let dir = Self::app_dir().join("profiles");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Saves key runner-specific tuning settings under `profiles\<slug>.profile`.
    pub fn save_runner_profile(&self, slug: &str) {
        if slug.trim().is_empty() {
            return;
        }
        let path = Self::profiles_dir().join(format!("{slug}.profile"));
        let text = format!(
            "console={}\nlog_level={}\ncard_buffers_min_mb={}\ncard_rehash_runs={}\njoin_copies={}\ndred={}\ngpu_timing={}\nskip_pipelines={}\n",
            self.console_model,
            self.log_level,
            self.card_buffers_min_mb,
            bstr(self.card_rehash_runs),
            bstr(self.join_copies),
            bstr(self.dred),
            bstr(self.gpu_timing),
            self.skip_pipelines,
        );
        let _ = fs::write(path, text);
    }

    /// Loads runner-specific overrides from `profiles\<slug>.profile` if present.
    pub fn load_runner_profile(&mut self, slug: &str) -> bool {
        let path = Self::profiles_dir().join(format!("{slug}.profile"));
        let Ok(text) = fs::read_to_string(path) else {
            return false;
        };
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim();
                match k.trim() {
                    "console" => {
                        self.console_model = if v.eq_ignore_ascii_case("series-s") { "series-s".into() } else { "series-x".into() };
                    }
                    "log_level" => self.log_level = normalize_log_level(v),
                    "card_buffers_min_mb" => self.card_buffers_min_mb = v.to_string(),
                    "card_rehash_runs" => self.card_rehash_runs = parse_bool(v),
                    "join_copies" => self.join_copies = parse_bool(v),
                    "dred" => self.dred = parse_bool(v),
                    "gpu_timing" => self.gpu_timing = parse_bool(v),
                    "skip_pipelines" => self.skip_pipelines = v.to_string(),
                    _ => {}
                }
            }
        }
        true
    }
}

impl Settings {
    pub fn hud_mode_label(&self) -> &'static str {
        match self.hud_mode % 3 {
            0 => "COMPACT",
            2 => "MINIMAL",
            _ => "STD+GRAPH",
        }
    }

    pub fn cycle_hud_mode(&mut self) -> &'static str {
        self.hud_mode = (self.hud_mode + 1) % 3;
        self.hud_mode_label()
    }

    pub fn cycle_ab_experiment(&mut self) -> &'static str {
        let next = crate::ab_bench::AbExperiment::from_id(&self.ab_experiment).cycle_next();
        self.ab_experiment = next.id().to_string();
        next.label()
    }

    pub fn cycle_keymap_preset(&mut self) -> &'static str {
        let next = crate::keymap::KeymapPreset::from_id(&self.keymap_preset).cycle_next();
        self.keymap_preset = next.id().to_string();
        let bindings = crate::keymap::preset_bindings(next);
        if !bindings.is_empty() {
            self.keymap = crate::keymap::serialize_bindings(&bindings);
        }
        next.label()
    }

    pub fn cycle_hud_corner(&mut self) -> &'static str {
        self.hud_corner = (self.hud_corner + 1) % 4;
        self.hud_corner_label()
    }

    pub fn hud_corner_label(&self) -> &'static str {
        match self.hud_corner % 4 {
            1 => "TOP-RIGHT",
            2 => "BOT-LEFT",
            3 => "BOT-RIGHT",
            _ => "TOP-LEFT",
        }
    }

    pub fn has_runner_profile(slug: &str) -> bool {
        Self::profiles_dir().join(format!("{slug}.profile")).is_file()
    }
}


pub const KEYBIND_SLOTS: [(&str, &str, &str); 14] = [
    ("A", "Confirm / Sprint (A)", "Enter"),
    ("B", "Back / Reload (B)", "Back"),
    ("X", "Jump / Cover (X)", "X"),
    ("Y", "Enter Vehicle / Account (Y)", "Y"),
    ("LB", "Weapon Wheel / LB", "Q"),
    ("RB", "Handbrake / Cover (RB)", "E"),
    ("LT", "Aim / Brake (LT)", "Z"),
    ("RT", "Fire / Accelerate (RT)", "C"),
    ("LS", "Crouch / Horn (L3)", "F"),
    ("RS", "Look Behind (R3)", "H"),
    ("MENU", "Pause Menu (Start)", "Esc"),
    ("VIEW", "Camera / Map (Select)", "Tab"),
    ("UP", "Phone / D-Pad Up", "Up"),
    ("DOWN", "Character / D-Pad Down", "Down"),
];

impl Settings {
    pub fn default_keymap_string() -> String {
        "A=Enter,B=Back,X=X,Y=Y,LB=Q,RB=E,LT=Z,RT=C,LS=F,RS=H,MENU=Esc,VIEW=Tab,UP=Up,DOWN=Down,LEFT=Left,RIGHT=Right,LY+=W,LY-=S,LX-=A,LX+=D,RY+=I,RY-=K,RX-=J,RX+=L".into()
    }

    pub fn get_keybind(&self, action: &str) -> String {
        for pair in self.keymap.split([',', ';']) {
            if let Some((k, v)) = pair.split_once('=') {
                if k.trim().eq_ignore_ascii_case(action) {
                    return v.trim().to_string();
                }
            }
        }
        for (k, _, def) in KEYBIND_SLOTS {
            if k.eq_ignore_ascii_case(action) {
                return def.to_string();
            }
        }
        "Unbound".into()
    }

    pub fn set_keybind(&mut self, action: &str, new_key: &str) {
        let clean_key = new_key.trim();
        if clean_key.is_empty() {
            return;
        }
        let mut pairs: Vec<(String, String)> = Vec::new();
        let mut found = false;
        let base = if self.keymap.trim().is_empty() {
            Self::default_keymap_string()
        } else {
            self.keymap.clone()
        };
        for pair in base.split([',', ';']) {
            if let Some((k, v)) = pair.split_once('=') {
                let kt = k.trim();
                if kt.eq_ignore_ascii_case(action) {
                    pairs.push((kt.to_string(), clean_key.to_string()));
                    found = true;
                } else {
                    pairs.push((kt.to_string(), v.trim().to_string()));
                }
            }
        }
        if !found {
            pairs.push((action.to_string(), clean_key.to_string()));
        }
        self.keymap = pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(",");
    }

    pub fn cycle_input_mode(&mut self) -> &'static str {
        match self.input_mode.as_str() {
            "auto" => {
                self.input_mode = "kbm".into();
                "KEYBOARD & MOUSE (FORCED)"
            }
            "kbm" => {
                self.input_mode = "pad".into();
                "PRIMARY CONTROLLER (XINPUT #1)"
            }
            _ => {
                self.input_mode = "auto".into();
                "AUTO (PAD #1 OR KBM FALLBACK)"
            }
        }
    }

    pub fn input_mode_label(&self) -> &'static str {
        match self.input_mode.as_str() {
            "kbm" => "MODE: KBM",
            "pad" => "MODE: PAD",
            _ => "MODE: AUTO",
        }
    }

    pub fn cycle_relay_mode(&mut self) -> &'static str {
        self.relay_mode = (self.relay_mode + 1) % 3;
        self.relay_label()
    }

    pub fn relay_label(&self) -> &'static str {
        match self.relay_mode {
            1 => "STAGE 1",
            2 => "STAGE 2",
            _ => "OFF",
        }
    }

    pub fn cycle_pipeline_wait(&mut self) -> u32 {
        self.pipeline_wait_ms = match self.pipeline_wait_ms {
            0 => 12,
            12 => 24,
            24 => 48,
            _ => 0,
        };
        self.pipeline_wait_ms
    }
}

impl Settings {
    pub fn cycle_mouse_sens(&mut self) -> &'static str {
        match self.mouse_sens.trim() {
            "0.6" => {
                self.mouse_sens = "1.0".into();
                "1.0x"
            }
            "1.0" => {
                self.mouse_sens = "1.5".into();
                "1.5x"
            }
            "1.5" => {
                self.mouse_sens = "2.0".into();
                "2.0x"
            }
            _ => {
                self.mouse_sens = "0.6".into();
                "0.6x"
            }
        }
    }

    pub fn runner_env_for(&self, runner_id: &str) -> Option<&str> {
        let key = runner_id.trim().to_ascii_lowercase();
        self.runner_env_overrides
            .iter()
            .find(|(id, _)| id == &key)
            .map(|(_, v)| v.as_str())
    }

    pub fn set_runner_env_for(&mut self, runner_id: &str, env_val: &str) {
        let key = runner_id.trim().to_ascii_lowercase();
        if key.is_empty() {
            return;
        }
        let val = env_val.trim().to_string();
        if let Some(entry) = self.runner_env_overrides.iter_mut().find(|(id, _)| id == &key) {
            if val.is_empty() {
                self.runner_env_overrides.retain(|(id, _)| id != &key);
            } else {
                entry.1 = val;
            }
        } else if !val.is_empty() {
            self.runner_env_overrides.push((key, val));
        }
    }
    pub fn export_preset_json(&self, dst: &Path) -> Result<(), String> {
        let json = format!(
            "{{\n\
             \x20 \"console_model\": \"{}\",\n\
             \x20 \"display_mode\": \"{}\",\n\
             \x20 \"borderless\": {},\n\
             \x20 \"active_preset\": \"{}\",\n\
             \x20 \"card_buffers_min_mb\": \"{}\",\n\
             \x20 \"card_buffers_mb\": \"{}\",\n\
             \x20 \"texture_room_percent\": \"{}\",\n\
             \x20 \"card_room_for_targets_mb\": \"{}\",\n\
             \x20 \"relay_mode\": {},\n\
             \x20 \"card_steps\": {},\n\
             \x20 \"thin_lists\": {},\n\
             \x20 \"upload_ahead\": {},\n\
             \x20 \"no_wait_resident\": {},\n\
             \x20 \"ambient_val\": \"{}\",\n\
             \x20 \"ambient_air\": \"{}\"\n\
             }}\n",
            self.console_model.trim(),
            self.display_mode.trim(),
            self.borderless,
            self.active_preset.trim(),
            self.card_buffers_min_mb.trim(),
            self.card_buffers_mb.trim(),
            self.texture_room_percent.trim(),
            self.card_room_for_targets_mb.trim(),
            self.relay_mode,
            self.card_steps,
            self.thin_lists,
            self.upload_ahead,
            self.no_wait_resident,
            self.ambient_val.trim(),
            self.ambient_air.trim(),
        );
        fs::write(dst, json).map_err(|e| format!("Failed to export preset JSON: {e}"))
    }

    pub fn import_preset_json(&mut self, src: &Path) -> Result<(), String> {
        let raw = fs::read_to_string(src).map_err(|e| format!("Failed to read preset JSON: {e}"))?;
        for line in raw.lines() {
            let trimmed = line.trim().trim_end_matches(',');
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().trim_matches('"');
                let val = v.trim().trim_matches('"');
                match key {
                    "console_model" => if !val.is_empty() { self.console_model = val.to_string(); },
                    "display_mode" => if !val.is_empty() { self.display_mode = val.to_string(); },
                    "borderless" => self.borderless = parse_bool(val),
                    "active_preset" => if !val.is_empty() { self.active_preset = val.to_string(); },
                    "card_buffers_min_mb" => self.card_buffers_min_mb = val.to_string(),
                    "card_buffers_mb" => self.card_buffers_mb = val.to_string(),
                    "texture_room_percent" => self.texture_room_percent = val.to_string(),
                    "card_room_for_targets_mb" => self.card_room_for_targets_mb = val.to_string(),
                    "relay_mode" => self.relay_mode = val.parse().unwrap_or(self.relay_mode),
                    "card_steps" => self.card_steps = val.parse().unwrap_or(self.card_steps),
                    "thin_lists" => self.thin_lists = parse_bool(val),
                    "upload_ahead" => self.upload_ahead = parse_bool(val),
                    "no_wait_resident" => self.no_wait_resident = parse_bool(val),
                    "ambient_val" => self.ambient_val = val.to_string(),
                    "ambient_air" => self.ambient_air = val.to_string(),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub fn truncate_utf8_mid(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars || max_chars < 6 {
        return s.to_string();
    }
    let keep = (max_chars - 3) / 2;
    let head: String = s.chars().take(keep).collect();
    let tail: String = s.chars().skip(count - (max_chars - 3 - keep)).collect();
    format!("{head}...{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_handles_empty_long_and_unicode_paths() {
        let mut s = Settings::default();
        s.exe = String::new();
        s.game_dir = String::new();
        s.runner = String::new();
        s.upscaler_dir = String::new();
        assert!(!s.resolved_runner().as_os_str().is_empty());
        assert_eq!(s.display_mode_label(), "1920 × 1080");

        let long_seg = "a".repeat(260);
        s.exe = format!(r"C:\Titles\日本語\テスト\{long_seg}\title.exe");
        s.game_dir = format!(r"C:\Titles\日本語\テスト\{long_seg}");
        s.runner = r"C:\Tools\日本語\victory.exe".into();
        s.upscaler_dir = r"C:\Tools\FSR_テスト".into();
        s.set_runner_env_for("xcl", "XCL_LOG=warn;XCL_TEST=1");
        assert_eq!(s.runner_env_for("xcl"), Some("XCL_LOG=warn;XCL_TEST=1"));
        assert_eq!(s.resolved_runner(), PathBuf::from(r"C:\Tools\日本語\victory.exe"));
        assert!(s.is_series_s());
    }
}
