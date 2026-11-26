use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use crate::config::{split_args, Settings};
use crate::diagnostics::{logs_dir, strip_ansi, SystemInfo};
use crate::runners::cache_dir_for;
use crate::win32::{
    boost_process, kill_stray_runners, local_timestamp_filename, local_timestamp_human,
    set_high_perf_gpu_preference, set_timer_1ms,
};

fn is_ztrx_runner(path: &Path) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("ztrx"))
}

/// Builds the exact, deduplicated list of verified runner environment variables
/// directly from `Settings` (zero hardcoded bypasses, zero stale/dead flags).
pub fn build_runner_env(s: &Settings) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut set = |k: &str, v: String| {
        if let Some(existing) = out.iter_mut().find(|(ek, _)| ek == k) {
            existing.1 = v;
        } else {
            out.push((k.to_string(), v));
        }
    };

    // Display & presentation
    set("VICTORY_BORDERLESS", if s.borderless { "1" } else { "0" }.into());
    match s.display_mode.trim() {
        "auto" | "" => {}
        dm => set("VICTORY_DISPLAY_MODE", dm.to_string()),
    }

    // VRAM, card buffers & texture residency (verified against shadow.rs & resource.rs)
    if !s.card_buffers_min_mb.trim().is_empty() {
        set("VICTORY_CARD_BUFFERS_MIN_MB", s.card_buffers_min_mb.trim().to_string());
    }
    if !s.card_buffers_mb.trim().is_empty() {
        set("VICTORY_CARD_BUFFERS_MB", s.card_buffers_mb.trim().to_string());
    }
    if !s.texture_room_percent.trim().is_empty() {
        set("VICTORY_TEXTURE_ROOM_PERCENT", s.texture_room_percent.trim().to_string());
    }
    if !s.card_room_for_targets_mb.trim().is_empty() {
        set("VICTORY_CARD_ROOM_FOR_TARGETS_MB", s.card_room_for_targets_mb.trim().to_string());
    }
    if !s.texture_budget_mb.trim().is_empty() {
        set("VICTORY_TEXTURE_BUDGET_MB", s.texture_budget_mb.trim().to_string());
    }
    if !s.texture_stage_mb.trim().is_empty() {
        set("VICTORY_TEXTURE_STAGE_MB", s.texture_stage_mb.trim().to_string());
    }
    if s.levels_on_demand {
        set("VICTORY_LEVELS_ON_DEMAND", "1".into());
    }
    if s.upload_ahead {
        set("VICTORY_UPLOAD_AHEAD", "1".into());
    }
    if s.texture_no_wait {
        set("VICTORY_TEXTURE_NO_WAIT", "1".into());
    }
    if s.no_wait_resident {
        set("VICTORY_WAIT_RESIDENT", "0".into());
    }

    // GPU pipeline, relay & CPU threading (verified against host.rs, queue.rs, cores.rs)
    if s.relay_mode > 0 {
        set("VICTORY_RELAY", s.relay_mode.to_string());
    }
    if s.relay_copy {
        set("VICTORY_RELAY_COPY", "1".into());
    }
    if s.async_submit {
        set("VICTORY_ASYNC_SUBMIT", "1".into());
    }
    set("VICTORY_THIN", if s.thin_lists { "1" } else { "0" }.into());
    if s.card_steps > 0 {
        set("VICTORY_CARD_STEPS", s.card_steps.to_string());
    }
    if s.wide_back {
        set("VICTORY_WIDE_BACK", "1".into());
    }
    if s.wide_refresh {
        set("VICTORY_WIDE_REFRESH", "1".into());
    }
    if s.wide_merge {
        set("VICTORY_WIDE_MERGE", "1".into());
    }
    if s.card_rehash_runs {
        set("VICTORY_CARD_REHASH_RUNS", "1".into());
    }
    if s.join_copies {
        set("VICTORY_JOIN_COPIES", "1".into());
    }
    if s.join_parts > 0 {
        set("VICTORY_JOIN_PARTS", s.join_parts.to_string());
    }
    if s.indirect_window {
        set("VICTORY_INDIRECT_WINDOW", "1".into());
    }
    if s.indirect_batch > 0 {
        set("VICTORY_INDIRECT_BATCH", s.indirect_batch.to_string());
    }
    if s.map_threads {
        set("VICTORY_MAP_THREADS", "1".into());
    }
    if s.card_awake {
        set("VICTORY_CARD_AWAKE", "1".into());
    }
    if s.show_after {
        set("VICTORY_SHOW_AFTER", "1".into());
    }
    if s.inplace_buf {
        set("VICTORY_INPLACE", "1".into());
    }
    if s.pipeline_wait_ms > 0 {
        set("VICTORY_PIPELINE_WAIT_MS", s.pipeline_wait_ms.to_string());
    }
    set("VICTORY_FAST_CORES", if s.fast_cores { "1" } else { "0" }.into());
    if s.profiler_on {
        set("VICTORY_PROFILE", "1".into());
    }

    // Lighting & experimental
    if s.ambient_fill {
        let v_trim = s.ambient_val.trim();
        let aval = if v_trim.is_empty() || v_trim.eq_ignore_ascii_case("off") { "0.8" } else { v_trim };
        set("VICTORY_AMBIENT", aval.to_string());
        let air_trim = s.ambient_air.trim();
        let air = if air_trim.is_empty() || air_trim == "0" { "1.5" } else { air_trim };
        set("VICTORY_AMBIENT_AIR", air.to_string());
    } else {
        set("VICTORY_AMBIENT", "off".into());
    }
    if s.bvh_builder {
        set("VICTORY_BVH_BUILDER", "1".into());
    }
    if let Some(ab_val) = crate::ab_bench::AbExperiment::from_id(&s.ab_experiment).env_value() {
        set("VICTORY_AB", ab_val.to_string());
    }

    // Custom user env overrides always win last (KEY=VAL;KEY2=VAL2)
    for pair in s.custom_env.split(';') {
        if let Some((k, v)) = pair.split_once('=') {
            let k = k.trim();
            if !k.is_empty() {
                set(k, v.trim().to_string());
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessKind {
    Game,
    Prepare,
}

impl ProcessKind {
    pub fn label(self) -> &'static str {
        match self {
            ProcessKind::Game => "GAME RUNNER",
            ProcessKind::Prepare => "SHADER PREPARE",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LiveFrameTelemetry {
    pub fps: f64,
    pub frame_ms: f64,
    pub gpu_ms: f64,
    pub draws: u32,
    pub dropped_draws: u32,
}

pub struct ActiveSession {
    pub kind: ProcessKind,
    pub child: Child,
    pub log_path: PathBuf,
    pub started_at: Instant,
    pub log_offset: u64,
    pub parsed_progress: Option<u32>,
    pub phase: String,
    pub timer_active: bool,
    pub live_telemetry: Option<LiveFrameTelemetry>,
    pub window_branded: bool,
    pub window_maximized: bool,
}

pub struct PollUpdate {
    pub new_lines: Vec<String>,
    pub finished: Option<FinishSummary>,
}

pub struct FinishSummary {
    pub kind: ProcessKind,
    pub exit_code: Option<i32>,
    pub elapsed_secs: u64,
    pub diagnosis: Option<String>,
    pub log_path: PathBuf,
}

fn validate_core(s: &Settings) -> Result<(), String> {
    let mut problems = Vec::new();
    if !Path::new(&s.exe).is_file() {
        problems.push(format!("Game executable not found:\n  {}", s.exe));
    }
    let runner_path = s.resolved_runner();
    if !runner_path.is_file() {
        problems.push(format!("Compatibility runner not found:\n  {}", runner_path.display()));
    }
    if !Path::new(&s.game_dir).is_dir() {
        problems.push(format!("Game data folder not found:\n  {}", s.game_dir));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n\n"))
    }
}

pub fn start(s: &Settings, sys: &SystemInfo) -> Result<ActiveSession, String> {
    validate_core(s)?;

    if s.auto_clean_strays {
        let _ = kill_stray_runners();
    }

    let log_dir = logs_dir();
    let stamp = local_timestamp_filename();
    let log_path = log_dir.join(format!("GM6-{stamp}.log"));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Cannot create log file: {e}"))?;

    let victory_log = s.effective_victory_log();
    let capture_dir = if s.capture {
        Some(log_dir.join(format!("frames-{stamp}")))
    } else {
        None
    };

    let perf_env_pairs = build_runner_env(s);
    let perf_env_parts: Vec<String> = perf_env_pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    let perf_env_str = if perf_env_parts.is_empty() {
        "(default)".to_string()
    } else {
        perf_env_parts.join(" ")
    };

    let header = format!(
        "=== Game Master 6 v1.0 session ({})\n\
         exe              : {}\n\
         game_dir         : {}\n\
         runner           : {}\n\
         console          : {}\n\
         title_args       : {}\n\
         log_level        : {} (VICTORY_LOG={})\n\
         capture          : {}\n\
         gpu_timing       : {}\n\
         upscaler         : {} ({})\n\
         shader_loop_limit: {}\n\
         gpu_offload_env  : {}\n\
         timer_1ms        : {} | high_priority: {} | prefer_dgpu: {}\n\
         host_memory      : {:.1}/{:.1} GB RAM avail, {:.1}/{:.1} GB Commit avail, AVX2={}\n\n",
        local_timestamp_human(),
        s.exe,
        s.game_dir,
        s.runner,
        s.console_model,
        if s.title_args.trim().is_empty() { "(none)" } else { s.title_args.trim() },
        s.log_level,
        victory_log,
        s.capture,
        s.gpu_timing,
        if s.upscaler_enabled { "ON" } else { "OFF" },
        if s.upscaler_dir.trim().is_empty() { "(none)" } else { s.upscaler_dir.trim() },
        if s.shader_loop_limit.trim().is_empty() { "AUTO" } else { s.shader_loop_limit.trim() },
        perf_env_str,
        s.timer_1ms,
        s.high_priority,
        s.prefer_dgpu,
        sys.avail_ram_gb,
        sys.total_ram_gb,
        sys.avail_commit_gb,
        sys.total_commit_gb,
        sys.avx2,
    );
    let _ = file.write_all(header.as_bytes());
    let _ = file.flush();
    let initial_offset = file.metadata().map(|m| m.len()).unwrap_or(0);

    let runner_path = s.resolved_runner();
    let kind = s.active_runner_kind();
    let cache_dir = cache_dir_for(&runner_path, kind, s.isolated_cache);
    let mut cmd = Command::new(&runner_path);
    let extra_args = split_args(&s.title_args);

    if kind.is_victory_family() {
        cmd.arg("run")
            .arg("--console")
            .arg(&s.console_model)
            .arg("--execute")
            .arg(&s.exe);
        if !extra_args.is_empty() {
            cmd.arg("--");
            for arg in extra_args {
                cmd.arg(arg);
            }
        }
        cmd.env("VICTORY_GAME_DIR", &s.game_dir)
            .env("VICTORY_LOG", victory_log);
        for (k, v) in &perf_env_pairs {
            cmd.env(k, v);
        }
        if s.isolated_cache {
            cmd.env("VICTORY_CACHE_DIR", &cache_dir);
        }
        if let Some(ref cdir) = capture_dir {
            cmd.env("VICTORY_CAPTURE", cdir);
        }
        if s.gpu_timing {
            cmd.env("VICTORY_GPU_TIMING", "1");
        }
        if s.dred || kind == crate::runners::RunnerKind::VictoryDiag {
            cmd.env("VICTORY_D3D12_DRED", "1");
        }
        if s.warp {
            cmd.env("VICTORY_D3D12_WARP", "1");
        }
        if !s.skip_pipelines.trim().is_empty() {
            cmd.env("VICTORY_SKIP_PIPELINES", s.skip_pipelines.trim());
        }
        if !s.shader_loop_limit.trim().is_empty() {
            cmd.env("VICTORY_SHADER_LOOP_LIMIT", s.shader_loop_limit.trim());
        }
        cmd.env("VICTORY_WINDOW_TITLE", "Game Master 6 - Title")
            .env("VICTORY_WINDOW_ICON", Settings::resolve_asset("logo.ico"));
    } else {
        // Universal adapter for custom compatibility runners and forks
        cmd.current_dir(&s.game_dir).arg(&s.exe);
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.env("VICTORY_GAME_DIR", &s.game_dir)
            .env("XCL_GAME_DIR", &s.game_dir);
    }

    // Future-proof custom environment overrides (KEY=VAL;KEY2=VAL2)
    for pair in s.custom_env.split(';') {
        if let Some((k, v)) = pair.split_once('=') {
            let k = k.trim();
            if !k.is_empty() {
                cmd.env(k, v.trim());
            }
        }
    }

    let out = file.try_clone().map_err(|e| format!("Cannot clone log handle: {e}"))?;
    let err = file.try_clone().map_err(|e| format!("Cannot clone log handle: {e}"))?;
    cmd.stdout(Stdio::from(out)).stderr(Stdio::from(err));

    if s.prefer_dgpu {
        unsafe {
            let _ = set_high_perf_gpu_preference(&runner_path, true);
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start runner '{}': {e}", runner_path.display()))?;

    if s.high_priority {
        unsafe {
            boost_process(child.id());
        }
    }

    let mut timer_active = false;
    if s.timer_1ms {
        unsafe {
            set_timer_1ms(true);
        }
        timer_active = true;
    }

    Ok(ActiveSession {
        kind: ProcessKind::Game,
        child,
        log_path,
        started_at: Instant::now(),
        log_offset: initial_offset,
        parsed_progress: None,
        phase: "Initializing compatibility runner...".into(),
        timer_active,
        live_telemetry: None,
        window_branded: false,
        window_maximized: false,
    })
}

pub fn prepare(s: &Settings, jobs: u32) -> Result<ActiveSession, String> {
    validate_core(s)?;
    let kind = s.active_runner_kind();
    if !kind.supports_prepare() {
        return Err(format!(
            "PREPARE CACHE (translate) is supported on VIctory / victory-fork / ztrx runners.\nActive adapter is {}. Switch to a VIctory runner in the RUNNERS tab.",
            kind.badge()
        ));
    }
    let game = Path::new(&s.game_dir);
    let shaders = game.join("shaders.rpf");
    let bvh = game.join("XboxBvhBuilder.dll");
    let upscaler = game.join("amd_fidelityfx_upscaler_xbox.dll");
    let exe = Path::new(&s.exe);

    let mut missing = Vec::new();
    for p in [&shaders, exe, &bvh, &upscaler] {
        if !p.is_file() {
            missing.push(format!("  - {}", p.display()));
        }
    }
    if !missing.is_empty() {
        return Err(format!("PREPARE requires these missing files:\n\n{}", missing.join("\n")));
    }

    let stamp = local_timestamp_filename();
    let log_path = logs_dir().join(format!("prepare-{stamp}.log"));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Cannot create prepare log: {e}"))?;

    let runner_path = s.resolved_runner();
    let ztrx = is_ztrx_runner(&runner_path);
    let is_baseline = runner_path.to_string_lossy().to_ascii_lowercase().contains("baseline");
    let use_cache_flag = !ztrx && !is_baseline;
    let flags_desc = if !use_cache_flag {
        format!("translate --pipelines --jobs {jobs}")
    } else {
        format!("translate --cache --pipelines --jobs {jobs}")
    };

    let header = format!(
        "=== Game Master 6 v1.0 shader preparation ({})\n\
         runner   : {}\n\
         cmd      : {}\n\
         shaders  : {}\n\
         exe      : {}\n\
         log_level: {}\n\n",
        local_timestamp_human(),
        s.runner,
        flags_desc,
        shaders.display(),
        exe.display(),
        s.effective_victory_log(),
    );
    let _ = file.write_all(header.as_bytes());
    let _ = file.flush();
    let initial_offset = file.metadata().map(|m| m.len()).unwrap_or(0);

    let out = file.try_clone().map_err(|e| e.to_string())?;
    let err = file.try_clone().map_err(|e| e.to_string())?;

    let cache_dir = cache_dir_for(&runner_path, kind, s.isolated_cache);
    let mut pcmd = Command::new(&runner_path);
    pcmd.arg("translate");
    if use_cache_flag {
        pcmd.arg("--cache");
    }
    pcmd.arg("--pipelines")
        .arg("--jobs")
        .arg(jobs.to_string())
        .arg(&shaders)
        .arg(exe)
        .arg(&bvh)
        .arg(&upscaler)
        .env("VICTORY_LOG", s.effective_victory_log())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));
    if s.isolated_cache {
        pcmd.env("VICTORY_CACHE_DIR", &cache_dir);
    }
    if !s.shader_loop_limit.trim().is_empty() {
        pcmd.env("VICTORY_SHADER_LOOP_LIMIT", s.shader_loop_limit.trim());
    }
    for (key, val) in build_runner_env(s) {
        pcmd.env(key, val);
    }
    let child = pcmd
        .spawn()
        .map_err(|e| format!("Failed to start shader preparation: {e}"))?;

    Ok(ActiveSession {
        kind: ProcessKind::Prepare,
        child,
        log_path,
        started_at: Instant::now(),
        log_offset: initial_offset,
        parsed_progress: None,
        phase: "Translating shaders & building pipeline cache...".into(),
        timer_active: false,
        live_telemetry: None,
        window_branded: false,
        window_maximized: false,
    })
}

pub fn poll(session: &mut ActiveSession) -> PollUpdate {
    let mut new_lines = Vec::new();
    if let Ok(mut f) = File::open(&session.log_path) {
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len > session.log_offset {
            if f.seek(SeekFrom::Start(session.log_offset)).is_ok() {
                let mut buf = Vec::new();
                if f.read_to_end(&mut buf).is_ok() {
                    session.log_offset += buf.len() as u64;
                    let text = String::from_utf8_lossy(&buf);
                    for raw_line in text.lines() {
                        let clean = strip_ansi(raw_line);
                        let trimmed = clean.trim();
                        if !trimmed.is_empty() {
                            if let Some(pct) = extract_progress(trimmed) {
                                session.parsed_progress = Some(pct);
                            }
                            if let Some(phase) = classify_phase(trimmed) {
                                session.phase = phase;
                            }
                            if let Some(ft) = parse_live_frame_telemetry(trimmed) {
                                session.live_telemetry = Some(ft);
                            }
                            new_lines.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
    }

    let mut finished = None;
    if let Ok(Some(status)) = session.child.try_wait() {
        if session.timer_active {
            unsafe {
                set_timer_1ms(false);
            }
            session.timer_active = false;
        }
        let code = status.code();
        let elapsed = session.started_at.elapsed().as_secs();
        let diagnosis = if code != Some(0) {
            Some(diagnose_failure(code, &session.log_path))
        } else {
            None
        };
        finished = Some(FinishSummary {
            kind: session.kind,
            exit_code: code,
            elapsed_secs: elapsed,
            diagnosis,
            log_path: session.log_path.clone(),
        });
    }

    PollUpdate { new_lines, finished }
}

pub fn stop(session: &mut ActiveSession) {
    let _ = session.child.kill();
    let _ = session.child.wait();
    if session.timer_active {
        unsafe {
            set_timer_1ms(false);
        }
        session.timer_active = false;
    }
}

/// Parses ztrx / VIctory live frame telemetry lines such as:
/// `30.0 fps, 33.3 ms/frame (...; 0.9 on the GPU; ...), 11 draws/frame, dropped 0 without a host pipeline`
fn parse_live_frame_telemetry(line: &str) -> Option<LiveFrameTelemetry> {
    if !line.contains("fps,") || !line.contains("ms/frame") {
        return None;
    }
    let fps_idx = line.find(" fps,")?;
    let fps_str = line[..fps_idx].split_whitespace().last()?;
    let fps = fps_str.parse::<f64>().ok()?;

    let ms_idx = line.find(" ms/frame")?;
    let ms_str = line[..ms_idx].split_whitespace().last()?;
    let frame_ms = ms_str.parse::<f64>().ok()?;

    let gpu_ms = line
        .find(" on the GPU")
        .and_then(|gi| line[..gi].split_whitespace().last())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let draws = line
        .find(" draws/frame")
        .and_then(|di| line[..di].split_whitespace().last())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let dropped_draws = line
        .find("dropped ")
        .and_then(|di| line.get(di + 8..)?.split_whitespace().next())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    Some(LiveFrameTelemetry {
        fps,
        frame_ms,
        gpu_ms,
        draws,
        dropped_draws,
    })
}

fn extract_progress(line: &str) -> Option<u32> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'%' && i > 0 {
            let mut start = i;
            while start > 0 && bytes[start - 1].is_ascii_digit() {
                start -= 1;
            }
            if start < i {
                if let Ok(v) = line[start..i].parse::<u32>() {
                    if v <= 100 {
                        return Some(v);
                    }
                }
            }
        }
    }
    for token in line.split_whitespace() {
        if let Some((a, b)) = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '/').split_once('/') {
            if let (Ok(cur), Ok(tot)) = (a.parse::<u64>(), b.parse::<u64>()) {
                if tot > 0 && cur <= tot {
                    return Some(((cur * 100) / tot).min(100) as u32);
                }
            }
        }
    }
    None
}

fn classify_phase(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("ms/frame") {
        Some("In-game — live frame telemetry active".into())
    } else if lower.contains("pipeline") {
        Some("Building graphics pipelines...".into())
    } else if lower.contains("shader") {
        Some("Translating shaders...".into())
    } else if lower.contains("d3d12") || lower.contains("dx12") || lower.contains("device") {
        Some("Initializing Direct3D 12 device...".into())
    } else if lower.contains("upscal") || lower.contains("fidelityfx") {
        Some("Configuring FidelityFX upscaler...".into())
    } else if lower.contains("present") || lower.contains("swapchain") {
        Some("Title active — presenting frames".into())
    } else {
        None
    }
}

fn extract_pre_breakpad_context(lines: &[String]) -> Option<String> {
    let bp_idx = lines.iter().position(|l| {
        let lo = l.to_ascii_lowercase();
        lo.contains("backtracebreakpad") || lo.contains("crashcontext.log") || lo.contains("upload_file_minidump")
    })?;
    let start = bp_idx.saturating_sub(8);
    let ctx: Vec<&str> = lines[start..bp_idx]
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("==="))
        .collect();
    if ctx.is_empty() {
        None
    } else {
        Some(ctx.join("\n"))
    }
}

fn diagnose_failure(code: Option<i32>, log_path: &Path) -> String {
    let tail = crate::diagnostics::read_log_tail(log_path, 200);
    let joined = tail.join("\n");
    let lower = joined.to_ascii_lowercase();

    if code == Some(1455) || lower.contains("1455") || lower.contains("paging file") || lower.contains("commitment") {
        return "Windows Error 1455 (Paging file / commit limit too small).\n\nClose background apps or click VIRTUAL MEM in Settings to increase your Windows paging file, then try again.".into();
    }
    if lower.contains("0x887a0005") || lower.contains("0x887a0006") || lower.contains("device_removed") || lower.contains("device_hung") {
        return "GPU Driver Reset / DXGI Device Removed (0x887A0005/6).\n\nWindows reset the GPU because a heavy world frame exceeded TdrDelay. Click FIX TDR 15s in Settings (and reboot), or switch to the ztrx runner / enable D3D12 DRED with a diag runner.".into();
    }
    if code == Some(-2147483645) || lower.contains("0x80000003") || lower.contains("backtracebreakpad") {
        let mut msg = "Game/Runner hit a breakpoint or assertion (0x80000003) and wrote a local crash dump.\n(Offline Breakpad upload being blocked is normal.)".to_string();
        if let Some(ctx) = extract_pre_breakpad_context(&tail) {
            msg.push_str("\n\nLines immediately before crash:\n");
            msg.push_str(&ctx);
        } else {
            msg.push_str("\n\nTry running PREPARE CACHE first, switching to Series S mode, or inspecting the Logs tab.");
        }
        if let Some((tid, cpath)) = crate::diagnostics::latest_victory_crashes_log() {
            if tid == "00000000" {
                msg.push_str("\n\nHint: crashes.log is under TitleID 00000000 — click LINK EXE TO GAME DIR on HOME so G: and MicrosoftGame.Config resolve!");
            } else {
                let ctail = crate::diagnostics::read_log_tail(&cpath, 20);
                let said: Vec<String> = ctail.into_iter().filter(|l| l.contains("title said:")).collect();
                if !said.is_empty() {
                    msg.push_str("\n\nTitle debug output (crashes.log):\n");
                    msg.push_str(&said.join("\n"));
                }
            }
        }
        return msg;
    }
    if code == Some(-1073741819) || lower.contains("0xc0000005") || lower.contains("access violation") {
        return "Runner exited with an access violation (0xC0000005).\n\nCheck the Logs tab for the exact stack trace, or try switching to Series S mode / running PREPARE first.".into();
    }
    if lower.contains("avx2") {
        return "Runner reported missing AVX2 CPU support. An AVX2-capable x64 CPU is required.".into();
    }
    if lower.contains("mesh shader") || lower.contains("shader model") || lower.contains("d3d12") {
        return "Direct3D 12 / Shader Model 6.6 / Mesh Shader initialization failed. Update your GPU drivers and verify D3D12 support.".into();
    }

    let code_str = code.map(|c| format!("exit code {c} (0x{:08X})", c as u32)).unwrap_or_else(|| "terminated".into());
    if let Some(last_err) = tail.iter().rev().find(|l| {
        let lo = l.to_ascii_lowercase();
        lo.contains("error") || lo.contains("panic") || lo.contains("failed")
    }) {
        format!("Runner exited with {code_str}.\n\nLast error:\n{last_err}")
    } else {
        format!("Runner exited with {code_str}. Open the Logs tab to inspect the full session log.")
    }
}

pub fn run_shader_census(s: &Settings) -> Result<crate::diagnostics::ShaderCensus, String> {
    validate_core(s)?;
    let runner_path = s.resolved_runner();
    let game = Path::new(&s.game_dir);
    let shaders = game.join("shaders.rpf");
    let exe = Path::new(&s.exe);
    let bvh = game.join("XboxBvhBuilder.dll");
    let upscaler = game.join("amd_fidelityfx_upscaler_xbox.dll");
    if !shaders.is_file() {
        return Err("shaders.rpf not found in game_dir".into());
    }
    let mut cmd = Command::new(&runner_path);
    cmd.arg("translate").arg("--list").arg(&shaders);
    if exe.is_file() { cmd.arg(exe); }
    if bvh.is_file() { cmd.arg(&bvh); }
    if upscaler.is_file() { cmd.arg(&upscaler); }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.output().map_err(|e| format!("Failed to run shader census: {e}"))?;
    let combined = format!(
        "{}
{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<String> = combined.lines().map(|l| strip_ansi(l)).collect();
    Ok(crate::diagnostics::parse_shader_census(&lines))
}
