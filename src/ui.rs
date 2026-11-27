use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::builder;
use crate::config::Settings;
use crate::saves::{self, ModItem};
use crate::telemetry::{self, LogComparison, RunnerBenchSummary, SessionRecord};
use crate::diagnostics::{
    clean_old_logs, compute_system_tip, export_ai_debug_bundle, export_report, export_tdr_reg, extract_slow_shaders, folder_stats, format_bytes, list_recent_logs, logs_dir, probe_top_ram_apps, read_log_tail,
    AssetAudit, LogEntry, ShaderCensus, SlowShader, SystemInfo,
};
use crate::hud::{CrosshairOverlay, HudOverlay};
use crate::launcher::{self, ActiveSession, ProcessKind};
use crate::monitor::{self, ProcessStats};
use crate::portwatch::{self, PortWatchSnapshot};
use crate::runners::{cache_dir_for, clean_junk_binaries, delete_runner, discover_runners, rollback_runner, runner_slug, runners_dir, seed_isolated_cache_from_global, DiscoveredRunner};
use crate::render::{
    paint, Fonts, LogoSurface, RenderContext, StatusKind, Target, LOG_LEVELS, SHADER_PRESETS,
    TUNING_SCROLL_STEP, tuning_max_scroll,
};
use crate::win32::*;

const DEFAULT_W: c_int = 1360;
const DEFAULT_H: c_int = 820;
const MIN_W: c_int = 1080;
const MIN_H: c_int = 680;
const TIMER_ID: usize = 1;

struct AppState {
    settings: Settings,
    sys: SystemInfo,
    audit: AssetAudit,
    session: Option<ActiveSession>,
    view: usize,
    hover: Target,
    hits: Vec<(RECT, Target)>,
    status_text: String,
    status_kind: StatusKind,
    data_label: String,
    file_count_label: String,
    scan_rx: Option<Receiver<(u64, u64)>>,
    recent_logs: Vec<LogEntry>,
    selected_log: usize,
    log_lines: Vec<String>,
    log_scroll: usize,
    log_filter: usize,
    anim_tick: u32,
    toast: Option<(String, Instant)>,
    fonts: Fonts,
    logo: Option<LogoSurface>,
    tracking_mouse: bool,
    hand_cursor: HCURSOR,
    arrow_cursor: HCURSOR,
    runners: Vec<DiscoveredRunner>,
    portwatch: PortWatchSnapshot,
    portwatch_rx: Option<Receiver<PortWatchSnapshot>>,
    portwatch_scroll: usize,
    runner_scroll: usize,
    tuning_scroll: usize,
    last_mouse_x: i32,
    backup_count: usize,
    slow_shaders: Vec<SlowShader>,
    ram_history: Vec<u32>,
    commit_history: Vec<u32>,
    recent_sessions: Vec<SessionRecord>,
    proc_stats: Option<ProcessStats>,
    hud: Option<HudOverlay>,
    crosshair: Option<CrosshairOverlay>,
    playtime_label: String,
    mod_count: usize,
    gpu_rx: Option<Receiver<(String, String, f64)>>,
    fps_history: Vec<u32>,
    runner_benches: Vec<RunnerBenchSummary>,
    mods: Vec<ModItem>,
    listening_keybind: Option<usize>,
    shader_census: ShaderCensus,
    census_rx: Option<Receiver<Result<ShaderCensus, String>>>,
    last_crash: Option<(String, String, PathBuf)>,
    /// Exact warning text the user dismissed, so a new warning reappears.
    dismissed_warning: Option<String>,
    adv_collapsed: [bool; 5],
    system_tip: (bool, String),
    tip_rx: Option<Receiver<(bool, String)>>,
    last_tip_probe: Instant,
    is_first_run: bool,
    modal_open: bool,
    benchmark_restore_log: Option<String>,
    log_comparison: Option<LogComparison>,
    show_log_compare: bool,
    free_ram_history: Vec<u32>,
    avail_commit_history: Vec<u32>,
    vram_headroom_history: Vec<u32>,
    last_headroom_sample: Instant,
    adv_filter: &'static str,
    auto_doctor: Option<telemetry::AutoDoctorReport>,
    cache_hash_mismatch: bool,
    beats_baseline: bool,
}

impl AppState {
    fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn active_toast(&self) -> Option<&str> {
        self.toast.as_ref().and_then(|(msg, ts)| {
            if ts.elapsed() < Duration::from_secs(4) {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    fn save_settings(&mut self, note: &str) {
        match self.settings.save() {
            Ok(()) => self.set_toast(note),
            Err(e) => self.set_toast(e),
        }
        self.refresh_diagnostics(false);
    }

    fn refresh_diagnostics(&mut self, rescan_size: bool) {
        let (prev_gpu, prev_drv, prev_vram) = (self.sys.gpu_name.clone(), self.sys.gpu_driver.clone(), self.sys.vram_gb);
        self.sys = SystemInfo::collect(&self.settings.game_dir);
        self.sys.gpu_name = prev_gpu;
        self.sys.gpu_driver = prev_drv;
        self.sys.vram_gb = prev_vram;
        self.audit = AssetAudit::scan(&self.settings, &self.sys);
        self.runners = discover_runners(&self.settings);
        self.backup_count = saves::count_backups();
        self.mods = saves::list_mods(8);
        self.mod_count = self.mods.len();
        let installed: Vec<String> = self
            .runners
            .iter()
            .flat_map(|r| {
                let mut v = vec![r.name.clone()];
                if let Some(f) = r.path.file_name().and_then(|s| s.to_str()) {
                    v.push(f.to_string());
                }
                v
            })
            .collect();
        self.runner_benches = telemetry::aggregate_by_runner(&installed);
        let (p_secs, p_cnt) = telemetry::total_playtime_summary();
        self.playtime_label = format!("PLAYTIME {} ({} RUNS)", telemetry::format_playtime(p_secs), p_cnt);
        self.recent_logs = list_recent_logs(12);
        if self.selected_log >= self.recent_logs.len() {
            self.selected_log = 0;
        }
        if self.session.is_none() {
            if let Some(entry) = self.recent_logs.get(self.selected_log) {
                self.log_lines = read_log_tail(&entry.path, 250);
            } else {
                self.log_lines.clear();
            }
            self.slow_shaders = extract_slow_shaders(&self.log_lines, 6);
            if self.audit.passed == self.audit.total {
                self.status_text = "SYSTEM READY".into();
                self.status_kind = StatusKind::Ready;
            } else {
                self.status_text = format!("CHECK {}/{}", self.audit.passed, self.audit.total);
                self.status_kind = StatusKind::Warning;
            }
        }
        if rescan_size {
            self.start_size_scan();
        }
    }

    fn start_size_scan(&mut self) {
        self.data_label = "SCANNING DATA...".into();
        self.file_count_label = "...".into();
        let (tx, rx) = channel();
        let path = PathBuf::from(&self.settings.game_dir);
        thread::spawn(move || {
            let stats = folder_stats(&path);
            let _ = tx.send(stats);
        });
        self.scan_rx = Some(rx);
    }

    fn start_portwatch_sync(&mut self) {
        if self.portwatch_rx.is_some() {
            return;
        }
        let installed_repos: Vec<String> = self
            .runners
            .iter()
            .filter(|r| !r.repo.is_empty())
            .map(|r| r.repo.clone())
            .collect();
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut snap = portwatch::fetch_live();
            if !installed_repos.is_empty() {
                snap.remote_commits = portwatch::fetch_remote_commits(&installed_repos);
            }
            let _ = tx.send(snap);
        });
        self.portwatch_rx = Some(rx);
    }

    fn start_tip_probe(&mut self) {
        if self.tip_rx.is_some() {
            return;
        }
        self.last_tip_probe = Instant::now();
        let s = self.settings.clone();
        let sys = self.sys.clone();
        let audit = self.audit.clone();
        let (tx, rx) = channel();
        thread::spawn(move || {
            let apps = probe_top_ram_apps();
            let batt = is_on_battery_power();
            let tip = compute_system_tip(&s, &sys, &audit, &apps, batt);
            let _ = tx.send(tip);
        });
        self.tip_rx = Some(rx);
    }

    fn start_shader_census_bg(&mut self) {
        if self.census_rx.is_some() {
            return;
        }
        let s_clone = self.settings.clone();
        let (tx, rx) = channel();
        thread::spawn(move || {
            let _ = tx.send(launcher::run_shader_census(&s_clone));
        });
        self.census_rx = Some(rx);
    }

    fn sync_telemetry_and_census(&mut self, trigger_census: bool) {
        telemetry::sync_sessions_from_logs();
        self.recent_sessions = telemetry::load_recent_sessions(8);
        let installed: Vec<String> = self
            .runners
            .iter()
            .flat_map(|r| {
                let mut v = vec![r.name.clone()];
                if let Some(f) = r.path.file_name().and_then(|s| s.to_str()) {
                    v.push(f.to_string());
                }
                v
            })
            .collect();
        self.runner_benches = telemetry::aggregate_by_runner(&installed);
        let (p_secs, p_cnt) = telemetry::total_playtime_summary();
        self.playtime_label = format!("PLAYTIME {} ({} RUNS)", telemetry::format_playtime(p_secs), p_cnt);
        if self.fps_history.is_empty() && !self.log_lines.is_empty() {
            self.fps_history = telemetry::parse_fps_series(&self.log_lines);
        }
        if trigger_census && self.audit.shader_cache_ready && self.shader_census.total == 0 {
            self.start_shader_census_bg();
        }
    }

    fn load_selected_log(&mut self) {
        self.log_scroll = 0;
        if let Some(entry) = self.recent_logs.get(self.selected_log) {
            self.log_lines = read_log_tail(&entry.path, 300);
        } else {
            self.log_lines.clear();
        }
        self.slow_shaders = extract_slow_shaders(&self.log_lines, 6);
    }
}


unsafe fn state_ptr(hwnd: HWND) -> *mut AppState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState
}

unsafe fn state(hwnd: HWND) -> &'static mut AppState {
    &mut *state_ptr(hwnd)
}

unsafe fn redraw(hwnd: HWND) {
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

/// Pauses WM_TIMER and prevents re-entrant AppState borrows while a modal dialog is open.
unsafe fn run_modal<T>(hwnd: HWND, f: impl FnOnce() -> T) -> Option<T> {
    {
        let st = state(hwnd);
        if st.modal_open {
            return None;
        }
        st.modal_open = true;
    }
    KillTimer(hwnd, TIMER_ID);
    let res = f();
    let st = state(hwnd);
    st.modal_open = false;
    let interval = if st.session.is_some() { 150 } else { 250 };
    SetTimer(hwnd, TIMER_ID, interval, None);
    Some(res)
}

unsafe fn show_msg(hwnd: HWND, body: &str, title: &str, flags: UINT) -> c_int {
    let b = wide(body);
    let t = wide(title);
    MessageBoxW(hwnd, b.as_ptr(), t.as_ptr(), flags)
}

unsafe fn handle_launch_or_stop(hwnd: HWND) {
    let st = state(hwnd);
    if let Some(mut sess) = st.session.take() {
        let kind = sess.kind;
        launcher::stop(&mut sess);
        st.proc_stats = None;
        if let Some(ref mut hud) = st.hud {
            hud.hide();
        }
        if let Some(ref mut ch) = st.crosshair {
            ch.hide();
        }
        KillTimer(hwnd, TIMER_ID);
        SetTimer(hwnd, TIMER_ID, 150, None);
        st.status_text = "STOPPED".into();
        st.status_kind = StatusKind::Warning;
        st.set_toast(format!("Stopped {}", kind.label()));
        st.refresh_diagnostics(false);
        redraw(hwnd);
        return;
    }

    if st.settings.auto_backup_saves {
        let _ = saves::backup_saves();
        st.backup_count = saves::count_backups();
    }
    match launcher::start(&st.settings, &st.sys) {
        Ok(sess) => {
            st.last_crash = None;
            st.status_text = "RUNNING".into();
            st.status_kind = StatusKind::Running;
            st.log_lines = vec![format!("Started Game Master 6 runner -> {}", sess.log_path.display())];
            st.log_scroll = 0;
            st.session = Some(sess);
            st.recent_logs = list_recent_logs(12);
            st.selected_log = 0;
            st.set_toast("Launched Title");
            st.fps_history.clear();
            if st.settings.hud_overlay {
                if let Some(ref mut hud) = st.hud {
                    hud.set_mode(st.settings.hud_mode);
                    hud.set_corner(st.settings.hud_corner);
                    let rname = st.settings.resolved_runner().file_name().and_then(|s| s.to_str()).unwrap_or("runner").to_string();
                    hud.update(&format!("Title · {rname} · STARTING..."), rgb(0, 240, 255));
                }
            }
            if st.settings.zero_overhead {
                KillTimer(hwnd, TIMER_ID);
                SetTimer(hwnd, TIMER_ID, if st.settings.kbm_crosshair { 120 } else if st.settings.hud_overlay { 1000 } else { 1500 }, None);
                ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
        Err(e) => {
            st.status_text = "LAUNCH BLOCKED".into();
            st.status_kind = StatusKind::Error;
            show_msg(hwnd, &e, "Cannot Launch Game Master 6", MB_ICONWARNING | MB_OK);
        }
    }
    redraw(hwnd);
}

unsafe fn handle_prepare(hwnd: HWND) {
    let st = state(hwnd);
    if st.session.is_some() {
        show_msg(
            hwnd,
            "A runner or shader preparation session is already active.\nStop the active session first.",
            "Session Already Running",
            MB_ICONINFORMATION | MB_OK,
        );
        return;
    }

    if st.audit.shader_cache_ready {
        let ans = show_msg(
            hwnd,
            &format!(
                "Shader cache is already prepared ({}).\n\nRe-run shader & pipeline preparation anyway?",
                st.audit.shader_cache_label
            ),
            "Cache Already Prepared",
            MB_ICONQUESTION | MB_YESNO,
        );
        if ans != IDYES {
            st.set_toast("Shader cache already prepared â€” ready to launch!");
            redraw(hwnd);
            return;
        }
    }

    match launcher::prepare(&st.settings, st.sys.recommended_jobs()) {
        Ok(sess) => {
            st.settings.last_prepared_runner_sig = st.settings.runner_signature();
            let _ = st.settings.save();
            st.status_text = "PREPARING".into();
            st.status_kind = StatusKind::Preparing;
            st.log_lines = vec![format!("Started shader preparation -> {}", sess.log_path.display())];
            st.log_scroll = 0;
            st.session = Some(sess);
            st.recent_logs = list_recent_logs(12);
            st.selected_log = 0;
            st.set_toast("Building shader & pipeline cache");
        }
        Err(e) => {
            st.status_text = "PREPARE BLOCKED".into();
            st.status_kind = StatusKind::Error;
            show_msg(hwnd, &e, "Cannot Prepare Cache", MB_ICONWARNING | MB_OK);
        }
    }
    redraw(hwnd);
}

unsafe fn handle_click(hwnd: HWND, target: Target) {
    let st = state(hwnd);
    match target {
        Target::None => {}
        Target::Nav(idx) => {
            st.listening_keybind = None;
            st.view = idx;
            if idx != 3 {
                st.tuning_scroll = 0;
            }
            if idx == 1
                && !st.settings.offline_mode
                && (st.portwatch.synced_at == "Offline registry"
                    || st.portwatch.synced_at == "Offline cache"
                    || st.portwatch.error.is_some())
                && st.portwatch_rx.is_none()
            {
                st.start_portwatch_sync();
            }
            if idx == 4 {
                st.recent_logs = list_recent_logs(12);
                if st.session.is_none() {
                    st.load_selected_log();
                }
                st.log_comparison = telemetry::compare_last_two_logs();
            }
            if idx == 5 {
                st.sync_telemetry_and_census(true);
            }
            redraw(hwnd);
        }
        Target::LaunchOrStop => handle_launch_or_stop(hwnd),
        Target::Prepare => handle_prepare(hwnd),
        Target::Rescan => {
            st.refresh_diagnostics(true);
            st.set_toast("Rescanned build files & telemetry");
            redraw(hwnd);
        }
        Target::OpenGameDir => {
            let p = PathBuf::from(&st.settings.game_dir);
            if p.is_dir() {
                open_path(hwnd, &p);
            } else {
                show_msg(hwnd, "Configured game data folder does not exist.", "Folder Not Found", MB_ICONWARNING | MB_OK);
            }
        }
        Target::OpenAppDir => open_path(hwnd, &Settings::app_dir()),
        Target::OpenLogsDir => open_path(hwnd, &logs_dir()),
        Target::OpenCurrentLog => {
            if let Some(ref sess) = st.session {
                open_path(hwnd, &sess.log_path);
            } else if let Some(entry) = st.recent_logs.get(st.selected_log) {
                open_path(hwnd, &entry.path);
            } else {
                open_path(hwnd, &logs_dir());
            }
        }
        Target::ExportReport => match export_report(&st.settings, &st.sys, &st.audit, &st.data_label) {
            Ok(path) => {
                st.set_toast("System report exported to reports/");
                let open = show_msg(
                    hwnd,
                    &format!("System report saved to:\n{}\n\nOpen report now?", path.display()),
                    "Report Exported",
                    MB_ICONINFORMATION | MB_YESNO,
                );
                if open == IDYES {
                    open_path(hwnd, &path);
                }
                redraw(hwnd);
            }
            Err(e) => {
                show_msg(hwnd, &e, "Export Failed", MB_ICONERROR | MB_OK);
            }
        },
        Target::OpenSettingsFile => {
            let _ = st.settings.save();
            open_path(hwnd, &Settings::path());
        }
        Target::ResetDefaults => {
            let confirm = show_msg(
                hwnd,
                "Reset all launcher settings to defaults?",
                "Reset Settings",
                MB_ICONQUESTION | MB_YESNO,
            );
            if confirm == IDYES {
                st.settings = Settings::default();
                st.save_settings("Settings reset to defaults");
                st.start_size_scan();
                redraw(hwnd);
            }
        }
        Target::ToggleUpscaler => {
            st.settings.upscaler_enabled = !st.settings.upscaler_enabled;
            st.save_settings(if st.settings.upscaler_enabled {
                "Host upscaler enabled (saved)"
            } else {
                "Host upscaler disabled (saved)"
            });
            redraw(hwnd);
        }
        Target::ToggleGpuTiming => {
            st.settings.gpu_timing = !st.settings.gpu_timing;
            st.save_settings(if st.settings.gpu_timing {
                "GPU timing enabled (saved)"
            } else {
                "GPU timing disabled (saved)"
            });
            redraw(hwnd);
        }
        Target::ToggleCapture => {
            st.settings.capture = !st.settings.capture;
            st.save_settings(if st.settings.capture {
                "Frame capture enabled (saved)"
            } else {
                "Frame capture disabled (saved)"
            });
            redraw(hwnd);
        }
        Target::ToggleStartMaximized => {
            st.settings.start_maximized = !st.settings.start_maximized;
            st.save_settings("Startup window preference saved");
            redraw(hwnd);
        }
        Target::ToggleZeroOverhead => {
            st.settings.zero_overhead = !st.settings.zero_overhead;
            st.save_settings(if st.settings.zero_overhead {
                "Zero-overhead play mode enabled"
            } else {
                "Zero-overhead play mode disabled"
            });
            redraw(hwnd);
        }
        Target::TogglePreferDgpu => {
            st.settings.prefer_dgpu = !st.settings.prefer_dgpu;
            let runner = st.settings.resolved_runner();
            let runner_name = runner
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("victory.exe");
            let reg_res = set_high_perf_gpu_preference(&runner, st.settings.prefer_dgpu);
            match reg_res {
                Ok(()) => st.save_settings(&if st.settings.prefer_dgpu {
                    format!("High-Perf dGPU set for {runner_name}")
                } else {
                    "Cleared dGPU registry override".to_string()
                }),
                Err(e) => st.set_toast(e),
            }
            redraw(hwnd);
        }
        Target::ToggleHighPriority => {
            st.settings.high_priority = !st.settings.high_priority;
            st.save_settings(if st.settings.high_priority {
                "High priority + no throttling enabled"
            } else {
                "Normal process priority restored"
            });
            redraw(hwnd);
        }
        Target::OpenVirtualMem => {
            open_virtual_memory_settings(hwnd);
        }
        Target::FixTdrDelay => {
            let cur = st.sys.tdr_delay_secs.unwrap_or(2);
            match export_tdr_reg() {
                Ok(reg_path) => {
                    let msg = format!(
                        "Current Windows GPU TdrDelay is {cur}s (default 2s causes driver resets during 2s+ world frames).\n\n                         Created registry helper:\n{}\n\n                         Apply it now? (Windows will ask for Admin approval; reboot required afterward.)",
                        reg_path.display()
                    );
                    let ans = show_msg(hwnd, &msg, "Fix GPU Timeout (TdrDelay = 15s)", MB_ICONINFORMATION | MB_YESNO);
                    if ans == IDYES {
                        open_path(hwnd, &reg_path);
                    }
                    st.set_toast("Exported reports/fix-tdr-15s.reg");
                    redraw(hwnd);
                }
                Err(e) => {
                    show_msg(hwnd, &e, "TDR Helper Error", MB_ICONERROR | MB_OK);
                }
            }
        }
        Target::LinkExeToGameDir => {
            let src_exe = PathBuf::from(&st.settings.exe);
            let game_dir = PathBuf::from(&st.settings.game_dir);
            if let Some(fname) = src_exe.file_name() {
                let dst_exe = game_dir.join(fname);
                let msg = format!(
                    "Co-locate game executable inside your Game Data folder?\n\n                     From: {}\nTo:   {}\n\n                     Uses an instant 0-byte NTFS hardlink on the same drive so VIctory / victory-fork can mount G:, read MicrosoftGame.Config, and load title DLLs properly.",
                    src_exe.display(),
                    dst_exe.display()
                );
                let ans = show_msg(hwnd, &msg, "Fix Split EXE / Game Dir", MB_ICONQUESTION | MB_YESNO);
                if ans == IDYES {
                    match hardlink_or_copy(&src_exe, &dst_exe) {
                        Ok(how) => {
                            st.settings.exe = dst_exe.to_string_lossy().into_owned();
                            st.save_settings(&format!("Executable {how} into game_dir!"));
                            redraw(hwnd);
                        }
                        Err(e) => {
                            show_msg(hwnd, &e, "Could Not Link Executable", MB_ICONERROR | MB_OK);
                        }
                    }
                }
            }
        }
        Target::ViewLogsQuick => {
            st.view = 4;
            st.recent_logs = list_recent_logs(12);
            if st.session.is_none() {
                st.load_selected_log();
            }
            redraw(hwnd);
        }
        Target::TuneQuick => {
            st.view = 3;
            redraw(hwnd);
        }
        Target::RunnersQuick => {
            st.view = 1;
            if !st.settings.offline_mode
                && (st.portwatch.synced_at == "Offline registry"
                    || st.portwatch.synced_at == "Offline cache"
                    || st.portwatch.error.is_some())
                && st.portwatch_rx.is_none()
            {
                st.start_portwatch_sync();
            }
            redraw(hwnd);
        }
        Target::SelectRunner(idx) => {
            if let Some(run) = st.runners.get(idx).cloned() {
                let old_slug = runner_slug(&st.settings.resolved_runner(), st.settings.active_runner_kind());
                let cur_env = st.settings.custom_env.clone();
                st.settings.set_runner_env_for(&old_slug, &cur_env);
                st.settings.save_runner_profile(&old_slug);
                let app_dir = Settings::app_dir();
                let rel = run
                    .path
                    .strip_prefix(&app_dir)
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| run.path.to_string_lossy().into_owned());
                st.settings.runner = rel;
                st.settings.runner_kind = run.kind.id().into();
                let new_slug = runner_slug(&run.path, run.kind);
                let loaded_prof = st.settings.load_runner_profile(&new_slug);
                st.settings.custom_env = st.settings.runner_env_for(&new_slug).unwrap_or("").to_string();
                if st.settings.prefer_dgpu {
                    let _ = set_high_perf_gpu_preference(&run.path, true);
                }
                let note = if loaded_prof {
                    format!("Switched to {} (loaded saved profile)", run.name)
                } else {
                    format!("Switched runner to {} ({})", run.name, run.kind.badge())
                };
                st.save_settings(&note);
                redraw(hwnd);
            }
        }
        Target::CycleRunnerKind => {
            let next = st.settings.active_runner_kind().cycle_next();
            st.settings.runner_kind = next.id().into();
            st.save_settings(&format!("Adapter set to {}", next.badge()));
            redraw(hwnd);
        }
        Target::RescanRunners => {
            st.runners = discover_runners(&st.settings);
            st.set_toast(format!("Found {} local runner(s)", st.runners.len()));
            redraw(hwnd);
        }
        Target::OpenRunnersDir => open_path(hwnd, &runners_dir()),
        Target::OpenCacheDir => {
            let cdir = cache_dir_for(
                &st.settings.resolved_runner(),
                st.settings.active_runner_kind(),
                st.settings.isolated_cache,
            );
            let _ = std::fs::create_dir_all(&cdir);
            open_path(hwnd, &cdir);
        }
        Target::SyncPortWatch => {
            if st.settings.offline_mode {
                st.set_toast("Strict Offline Mode is ON — turn it OFF in Settings to sync");
            } else {
                st.start_portwatch_sync();
                st.set_toast("Syncing live PortWatch VI standings...");
            }
            redraw(hwnd);
        }
        Target::OpenPortWatchRepo(idx) => {
            if let Some(proj) = st.portwatch.projects.get(idx) {
                open_url(hwnd, &proj.url);
            }
        }
        Target::ToggleDred => {
            st.settings.dred = !st.settings.dred;
            st.save_settings(if st.settings.dred {
                "D3D12 DRED fault tracing enabled (requires diag runner)"
            } else {
                "D3D12 DRED disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleIsolatedCache => {
            st.settings.isolated_cache = !st.settings.isolated_cache;
            st.save_settings(if st.settings.isolated_cache {
                "Isolated per-runner shader cache enabled"
            } else {
                "Using shared %LOCALAPPDATA%\\victory\\cache"
            });
            redraw(hwnd);
        }
        Target::ToggleWarp => {
            st.settings.warp = !st.settings.warp;
            st.save_settings(if st.settings.warp {
                "WARP software D3D12 enabled (safe, slow)"
            } else {
                "WARP software D3D12 disabled"
            });
            redraw(hwnd);
        }
        Target::BuildPortWatchRepo(idx) => {
            if let Some(proj) = st.portwatch.projects.get(idx).cloned() {
                let runners_root = runners_dir();
                let name_lc = proj.name.to_ascii_lowercase();
                let slug_lc = proj.repo_slug();
                let repo_lc = proj.repo.to_ascii_lowercase();
                let existing_folder = st
                    .runners
                    .iter()
                    .find(|r| {
                        crate::render::matches_portwatch_project(
                            r,
                            &runners_root,
                            &name_lc,
                            &slug_lc,
                            &repo_lc,
                        )
                    })
                    .and_then(|r| {
                        let parent = r.path.parent()?;
                        if parent.parent() == Some(runners_root.as_path()) {
                            parent.file_name().map(|s| s.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    });
                let target_name = existing_folder.unwrap_or_else(|| proj.name.clone());
                let warn = format!(
                    "Clone & compile community repository into runners\\{}?\n\nRepo: https://github.com/{}\n\nSECURITY NOTICE: Compiling third-party repos runs their Cargo/CMake build scripts. The auto-builder scans build.rs / CMakeLists.txt for suspicious commands and backs up any previous .exe before replacing.\n\nProceed only if you trust this repository.",
                    target_name, proj.repo
                );
                let ok = show_msg(hwnd, &warn, "Confirm Safe Auto-Build", MB_ICONWARNING | MB_YESNO);
                if ok == IDYES {
                    match builder::launch_clone_and_build(&proj.repo, &target_name) {
                        Ok(()) => {
                            st.set_toast(format!("Safe builder started for {} — click RESCAN when done", target_name));
                            redraw(hwnd);
                        }
                        Err(e) => {
                            show_msg(hwnd, &e, "Cannot Auto-Build", MB_ICONWARNING | MB_OK);
                        }
                    }
                }
            }
        }
        Target::OpenTrackersFile => {
            let p = portwatch::trackers_file();
            open_path(hwnd, &p);
        }
        Target::ToggleOfflineMode => {
            st.settings.offline_mode = !st.settings.offline_mode;
            if st.settings.offline_mode {
                st.save_settings("Strict Offline Mode ON (no auto-network calls)");
            } else {
                st.save_settings("Online mode ON — syncing live tracker...");
                st.start_portwatch_sync();
            }
            redraw(hwnd);
        }
        Target::ClearActiveCache => {
            let cdir = cache_dir_for(
                &st.settings.resolved_runner(),
                st.settings.active_runner_kind(),
                st.settings.isolated_cache,
            );
            let confirm = show_msg(
                hwnd,
                &format!("Clear translated shaders & pipeline cache in:\n{}\n\nGame files and saves are NOT touched.", cdir.display()),
                "Clear Runner Cache",
                MB_ICONQUESTION | MB_YESNO,
            );
            if confirm == IDYES {
                let _ = std::fs::remove_dir_all(cdir.join("shaders"));
                let _ = std::fs::remove_dir_all(cdir.join("pipelines"));
                st.refresh_diagnostics(false);
                st.set_toast("Cleared active runner shader & pipeline cache");
                redraw(hwnd);
            }
        }
        Target::BackupSaves => match saves::backup_saves() {
            Ok((dest, files)) => {
                st.backup_count = saves::count_backups();
                st.set_toast(format!("Backed up {files} save file(s)"));
                let open = show_msg(
                    hwnd,
                    &format!("Backed up {files} file(s) to:\n{}\n\nOpen backup folder?", dest.display()),
                    "Saves Backed Up",
                    MB_ICONINFORMATION | MB_YESNO,
                );
                if open == IDYES {
                    open_path(hwnd, &dest);
                }
                redraw(hwnd);
            }
            Err(e) => {
                show_msg(hwnd, &e, "Save Backup", MB_ICONWARNING | MB_OK);
            }
        },
        Target::OpenBackupsDir => open_path(hwnd, &saves::backups_dir()),
        Target::ApplyPreset(preset) => {
            let kind = match preset {
                0 => crate::presets::PresetKind::High,
                1 => crate::presets::PresetKind::Balanced,
                _ => crate::presets::PresetKind::Low,
            };
            let msg = crate::presets::apply_adaptive_preset(&mut st.settings, kind, &st.sys);
            st.save_settings(&msg);
            redraw(hwnd);
        }
        Target::SetConsole(is_s) => {
            st.settings.console_model = if is_s { "series-s".into() } else { "series-x".into() };
            st.save_settings(if is_s { "Switched to Xbox Series S profile" } else { "Switched to Xbox Series X profile" });
            redraw(hwnd);
        }
        Target::SetLogLevel(idx) => {
            if let Some(&lvl) = LOG_LEVELS.get(idx) {
                st.settings.log_level = lvl.into();
                st.save_settings(&format!("Log level set to {lvl}"));
                redraw(hwnd);
            }
        }
        Target::SetShaderLimit(idx) => {
            if let Some(&preset) = SHADER_PRESETS.get(idx) {
                st.settings.shader_loop_limit = if idx == 0 { String::new() } else { preset.into() };
                st.save_settings(&format!("Shader loop limit: {preset}"));
                redraw(hwnd);
            }
        }
        Target::BrowseExe => {
            let init = st.settings.exe.clone();
            if let Some(Some(p)) = run_modal(hwnd, || {
                browse_file(hwnd, "Select Title Executable", &[("Executables (*.exe)", "*.exe"), ("All Files", "*.*")], &init)
            }) {
                let st = state(hwnd);
                st.settings.exe = p;
                st.save_settings("Game executable updated");
                redraw(hwnd);
            }
        }
        Target::BrowseGameDir => {
            if let Some(Some(p)) = run_modal(hwnd, || browse_folder(hwnd, "Select Title Data Folder")) {
                let st = state(hwnd);
                st.settings.game_dir = p;
                st.save_settings("Game data folder updated");
                st.start_size_scan();
                redraw(hwnd);
            }
        }
        Target::BrowseRunner => {
            let init = st.settings.runner.clone();
            if let Some(Some(p)) = run_modal(hwnd, || {
                browse_file(hwnd, "Select Compatibility Runner (victory.exe)", &[("Executables (*.exe)", "*.exe")], &init)
            }) {
                let st = state(hwnd);
                st.settings.runner = p;
                st.save_settings("Runner executable updated");
                redraw(hwnd);
            }
        }
        Target::PastePath(idx) => {
            if let Some(raw) = paste_from_clipboard(hwnd) {
                let cleaned = raw.trim().trim_matches('"').trim().to_string();
                if cleaned.is_empty() {
                    st.set_toast("Clipboard is empty");
                } else {
                    match idx {
                        0 => {
                            st.settings.exe = cleaned;
                            st.save_settings("Pasted game executable path");
                        }
                        1 => {
                            st.settings.game_dir = cleaned;
                            st.save_settings("Pasted game data folder");
                            st.start_size_scan();
                        }
                        _ => {
                            st.settings.runner = cleaned;
                            st.save_settings("Pasted runner executable path");
                        }
                    }
                }
                redraw(hwnd);
            } else {
                st.set_toast("No text found in clipboard");
                redraw(hwnd);
            }
        }
        Target::BrowseUpscalerDir => {
            if let Some(Some(p)) = run_modal(hwnd, || browse_folder(hwnd, "Select Signed Host Upscaler DLL Folder")) {
                let st = state(hwnd);
                st.settings.upscaler_dir = p;
                st.settings.upscaler_enabled = true;
                st.save_settings("Upscaler folder set & enabled");
                redraw(hwnd);
            }
        }
        Target::ClearUpscalerDir => {
            st.settings.upscaler_dir.clear();
            st.settings.upscaler_enabled = false;
            st.save_settings("Upscaler folder cleared");
            redraw(hwnd);
        }
        Target::RollbackRunner(idx) => {
            if let Some(run) = st.runners.get(idx).cloned() {
                let ans = show_msg(
                    hwnd,
                    &format!("Restore previous backup (.bak) for {}?", run.name),
                    "Rollback Runner",
                    MB_ICONQUESTION | MB_YESNO,
                );
                if ans == IDYES {
                    match rollback_runner(&run.path) {
                        Ok(()) => {
                            st.refresh_diagnostics(false);
                            st.set_toast(format!("Rolled back {} to previous .bak", run.name));
                            redraw(hwnd);
                        }
                        Err(e) => {
                            show_msg(hwnd, &e, "Rollback Failed", MB_ICONERROR | MB_OK);
                        }
                    }
                }
            }
        }
        Target::DeleteRunner(idx) => {
            if let Some(run) = st.runners.get(idx).cloned() {
                let ans = show_msg(
                    hwnd,
                    &format!(
                        "Delete the runner '{}' and its runner folder?\n\n{}\n\nThis cannot be undone.",
                        run.name,
                        run.path.display()
                    ),
                    "Delete Runner",
                    MB_ICONWARNING | MB_YESNO,
                );
                if ans == IDYES {
                    match delete_runner(&run.path) {
                        Ok(()) => {
                            st.runners = discover_runners(&st.settings);
                            let del_repo_lc = run.repo.trim().to_ascii_lowercase();
                            if !del_repo_lc.is_empty()
                                && !st.runners.iter().any(|r| r.repo.trim().eq_ignore_ascii_case(&del_repo_lc))
                            {
                                st.portwatch.remote_commits.retain(|(repo, _)| repo != &del_repo_lc);
                            }
                            st.set_toast(format!("Deleted runner {}", run.name));
                            redraw(hwnd);
                        }
                        Err(e) => {
                            show_msg(hwnd, &e, "Delete Failed", MB_ICONERROR | MB_OK);
                        }
                    }
                }
            }
        }
        Target::ToggleSkipShader(idx) => {
            if let Some(sh) = st.slow_shaders.get(idx).cloned() {
                let added = st.settings.toggle_skip_hash(&sh.hash);
                st.save_settings(if added {
                    "Added shader hash to VICTORY_SKIP_PIPELINES"
                } else {
                    "Removed shader hash from VICTORY_SKIP_PIPELINES"
                });
                redraw(hwnd);
            }
        }
        Target::ClearSkipShaders => {
            st.settings.skip_pipelines.clear();
            st.save_settings("Cleared all skipped shader hashes");
            redraw(hwnd);
        }
        Target::ShowKeymap => {
            st.view = 5;
            st.sys.xinput_pad_details = poll_xinput_pads();
            st.sys.xinput_pads = st.sys.xinput_pad_details.iter().filter(|p| p.connected).count() as u32;
            st.set_toast("Opened Controls & Live Keybind Remapper");
            redraw(hwnd);
            return;
        }
        Target::ExportAiBundle => {
            match export_ai_debug_bundle(
                &st.settings,
                &st.sys,
                &st.audit,
                &st.data_label,
                &st.log_lines,
                &st.slow_shaders,
                &st.recent_sessions,
            ) {
                Ok((path, md_text)) => {
                    let copied = copy_to_clipboard(hwnd, &md_text);
                    let clip_note = if copied {
                        "Copied straight to your Windows Clipboard (ready for Ctrl+V)!"
                    } else {
                        "Saved to reports folder."
                    };
                    st.set_toast(if copied { "AI Debug Bundle copied to clipboard!" } else { "Exported AI Debug Bundle to reports/" });
                    let open = show_msg(
                        hwnd,
                        &format!("{clip_note}\n\nFile: {}\n\nOpen the Markdown file as well?", path.display()),
                        "AI Debug Bundle Ready",
                        MB_ICONINFORMATION | MB_YESNO,
                    );
                    if open == IDYES {
                        open_path(hwnd, &path);
                    }
                    redraw(hwnd);
                }
                Err(e) => { show_msg(hwnd, &e, "Export Failed", MB_ICONERROR | MB_OK); }
            }
        }
        Target::DismissCrash => {
            st.last_crash = None;
            st.set_toast("Dismissed crash summary");
            redraw(hwnd);
        }
        Target::DismissWarning => {
            st.dismissed_warning = Some(st.system_tip.1.clone());
            st.set_toast("Notice dismissed");
            redraw(hwnd);
        }
        Target::CycleAdvFilter => {
            st.adv_filter = match st.adv_filter {
                "ALL" => "VRAM",
                "VRAM" => "CPU",
                "CPU" => "LIGHTING",
                _ => "ALL",
            };
            st.set_toast(format!("Advanced filter: {}", st.adv_filter));
            redraw(hwnd);
        }
        Target::PinBaseline => {
            if st.settings.pinned_baseline_locked {
                st.settings.pinned_baseline_locked = false;
                st.save_settings("Baseline unlocked");
            } else {
                st.settings.pinned_baseline_runner = st.settings.runner.clone();
                st.settings.pinned_baseline_locked = true;
                st.save_settings("Pinned current runner & settings as baseline");
                crate::guardian::record_known_good_if_stable(999, Some(0));
            }
            redraw(hwnd);
        }
        Target::RollbackBaseline => {
            if let Ok(good) = crate::guardian::restore_last_known_good() {
                st.settings = good;
                st.save_settings("Rolled back to pinned baseline");
            } else if !st.settings.pinned_baseline_runner.trim().is_empty() {
                st.settings.runner = st.settings.pinned_baseline_runner.clone();
                st.save_settings("Restored pinned baseline runner");
            } else {
                st.set_toast("No pinned baseline saved yet — click PIN BASELINE first");
            }
            redraw(hwnd);
        }
        Target::PromoteBaseline => {
            st.settings.pinned_baseline_runner = st.settings.runner.clone();
            st.settings.pinned_baseline_locked = true;
            st.beats_baseline = false;
            st.save_settings("Promoted winning runner to pinned baseline (+5% gate passed)");
            crate::guardian::record_known_good_if_stable(999, Some(0));
            redraw(hwnd);
        }
        Target::ApplyDoctorFix => {
            if let Some(ref doc) = st.auto_doctor {
                match doc.recommended_fix {
                    Some(telemetry::DoctorFix::LowerTexRoom90) => {
                        st.settings.texture_room_percent = "90".into();
                        st.settings.card_room_for_targets_mb = "2048".into();
                        st.save_settings("Auto-Doctor: capped VRAM room to 90% / 2048 MB");
                    }
                    Some(telemetry::DoctorFix::EnableMemTrim) => {
                        st.save_settings("Auto-Doctor: idle memory trimming enabled");
                    }
                    Some(telemetry::DoctorFix::PrepareCache) => {
                        handle_prepare(hwnd);
                        return;
                    }
                    Some(telemetry::DoctorFix::SwitchBalanced1080p) => {
                        let _ = crate::presets::apply_adaptive_preset(&mut st.settings, crate::presets::PresetKind::Balanced, &st.sys);
                        st.save_settings("Auto-Doctor: switched to BALANCED 1080p");
                    }
                    None => {}
                }
                st.auto_doctor = None;
                redraw(hwnd);
            }
        }
        Target::CopyBenchmarkMd => {
            if let Some(ref doc) = st.auto_doctor {
                let md = doc.to_anonymized_markdown();
                let _ = copy_to_clipboard(hwnd, &md);
                st.set_toast("Copied anonymized benchmark Markdown to clipboard");
                redraw(hwnd);
            }
        }
        Target::KillRamHog(idx) => {
            let apps = crate::diagnostics::probe_top_ram_apps();
            if let Some((name, _)) = apps.get(idx) {
                if crate::diagnostics::kill_process_by_name(name) {
                    st.set_toast(format!("Closed {name}"));
                    st.refresh_diagnostics(false);
                } else {
                    st.set_toast(format!("Skipped protected or already closed process: {name}"));
                }
                redraw(hwnd);
            }
        }
        Target::BuildCustomRepo => {
            if let Some(Some(repo_url)) = run_modal(hwnd, || {
                prompt_text_input(
                    "Build Runner from Git URL",
                    "Enter GitHub URL or owner/repo (e.g. DoubleGhosty/cool_fox):",
                    "DoubleGhosty/cool_fox",
                )
            }) {
                let st = state(hwnd);
                let name = repo_url.rsplit('/').next().unwrap_or("custom-runner").trim_end_matches(".git");
                match crate::builder::launch_clone_and_build(&repo_url, name) {
                    Ok(()) => st.set_toast("Launched local source builder terminal"),
                    Err(e) => {
                        show_msg(hwnd, &e, "Builder Error", MB_ICONERROR | MB_OK);
                    }
                }
                redraw(hwnd);
            }
        }
        Target::ExportPresetJson => {
            let dst = crate::diagnostics::reports_dir().join(format!(
                "preset-{}-{}.json",
                st.settings.console_model,
                st.settings.active_preset
            ));
            match st.settings.export_preset_json(&dst) {
                Ok(()) => {
                    st.set_toast("Exported path-free preset JSON to reports\\");
                    open_path(hwnd, &dst);
                }
                Err(e) => {
                    show_msg(hwnd, &e, "Export Preset", MB_ICONERROR | MB_OK);
                }
            }
            redraw(hwnd);
        }
        Target::ImportPresetJson => {
            if let Some(Some(p)) = run_modal(hwnd, || {
                browse_file(hwnd, "Import Path-Free Preset JSON", &[("JSON Presets (*.json)", "*.json")], "")
            }) {
                let st = state(hwnd);
                match st.settings.import_preset_json(std::path::Path::new(&p)) {
                    Ok(()) => st.save_settings("Imported preset JSON cleanly"),
                    Err(e) => {
                        show_msg(hwnd, &e, "Import Preset", MB_ICONERROR | MB_OK);
                    }
                }
                redraw(hwnd);
            }
        }
        Target::OpenReleasesPage => {
            if !st.settings.offline_mode {
                open_path(hwnd, std::path::Path::new("https://github.com/game-master-6/game-master-6/releases"));
            }
        }
        Target::OpenSessionsCsv => {
            let p = telemetry::sessions_csv_path();
            if p.is_file() {
                open_path(hwnd, &p);
            } else {
                show_msg(hwnd, "No session benchmarks recorded yet. Run a session first!", "Sessions CSV", MB_ICONINFORMATION | MB_OK);
            }
        }
        Target::CleanOldLogs => {
            let ans = show_msg(
                hwnd,
                "Delete older log files in logs\\ (keeping the 5 newest)?\n\nSaves, shader caches, and game files are NOT touched.",
                "Clean Old Logs",
                MB_ICONQUESTION | MB_YESNO,
            );
            if ans == IDYES {
                let removed = clean_old_logs(5);
                st.recent_logs = list_recent_logs(12);
                st.selected_log = 0;
                st.load_selected_log();
                st.set_toast(format!("Cleaned {removed} old log file(s)"));
                redraw(hwnd);
            }
        }
        Target::CycleTheme => {
            let name = st.settings.cycle_theme();
            st.save_settings(&format!("Theme switched to {name}"));
            redraw(hwnd);
        }
        Target::SelectLog(idx) => {
            if idx < st.recent_logs.len() {
                st.selected_log = idx;
                st.load_selected_log();
                redraw(hwnd);
            }
        }
        Target::CycleCardBuffers => {
            let lbl = st.settings.cycle_card_buffers();
            st.save_settings(&format!("VRAM buffer offload min size: {lbl}"));
            redraw(hwnd);
        }
        Target::ToggleCardRehash => {
            st.settings.card_rehash_runs = !st.settings.card_rehash_runs;
            st.save_settings(if st.settings.card_rehash_runs {
                "GPU rehash batching enabled (VICTORY_CARD_REHASH_RUNS=1)"
            } else {
                "GPU rehash batching disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleJoinCopies => {
            st.settings.join_copies = !st.settings.join_copies;
            st.save_settings(if st.settings.join_copies {
                "Buffer copy coalescing enabled (VICTORY_JOIN_COPIES=1)"
            } else {
                "Buffer copy coalescing disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleTimer1ms => {
            st.settings.timer_1ms = !st.settings.timer_1ms;
            st.save_settings(if st.settings.timer_1ms {
                "1ms high-res Windows timer enabled for gameplay"
            } else {
                "Standard Windows timer resolution restored"
            });
            redraw(hwnd);
        }
        Target::CleanJunkRunners => {
            let removed = clean_junk_binaries();
            st.runners = discover_runners(&st.settings);
            st.set_toast(if removed > 0 {
                format!("Cleaned {removed} build-helper junk binary(ies) (xtask.exe)")
            } else {
                "No junk binaries found in runners\\".into()
            });
            redraw(hwnd);
        }
        Target::SeedIsolatedCache => {
            let cdir = cache_dir_for(
                &st.settings.resolved_runner(),
                st.settings.active_runner_kind(),
                true,
            );
            match seed_isolated_cache_from_global(&cdir) {
                Ok(n) => {
                    st.refresh_diagnostics(false);
                    st.set_toast(format!("Seeded isolated cache with {n} files (0-byte hardlinks)"));
                }
                Err(e) => {
                    show_msg(hwnd, &e, "Seed Isolated Cache", MB_ICONWARNING | MB_OK);
                }
            }
            redraw(hwnd);
        }
        Target::KillStrayRunners => {
            let n = kill_stray_runners();
            st.set_toast(if n > 0 {
                format!("Terminated {n} stray runner process group(s)")
            } else {
                "No stray GameRunner / victory processes running".into()
            });
            redraw(hwnd);
        }
        Target::AutoEnhance => {
            let mut actions: Vec<String> = Vec::new();
            // 1. Clean junk binaries (xtask.exe)
            let junk = clean_junk_binaries();
            if junk > 0 {
                actions.push(format!("Removed {junk} xtask.exe junk binary(ies)"));
            }
            // 2. Kill stray background runner processes
            let killed = kill_stray_runners();
            if killed > 0 {
                actions.push(format!("Stopped {killed} stray runner process(es)"));
            }
            // 3. Apply High-Performance dGPU preference strictly to the selected runner (if enabled)
            if st.settings.prefer_dgpu {
                let runner = st.settings.resolved_runner();
                if set_high_perf_gpu_preference(&runner, true).is_ok() {
                    let name = runner
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("victory.exe");
                    actions.push(format!("Locked High-Perf dGPU on {name}"));
                }
            }
            // 5. Fix split exe / game_dir via 0-byte NTFS hardlink if needed
            if st.audit.exe_outside_game_dir {
                let src_exe = PathBuf::from(&st.settings.exe);
                let game_dir = PathBuf::from(&st.settings.game_dir);
                if let Some(fname) = src_exe.file_name() {
                    let dst_exe = game_dir.join(fname);
                    if let Ok(how) = hardlink_or_copy(&src_exe, &dst_exe) {
                        st.settings.exe = dst_exe.to_string_lossy().into_owned();
                        actions.push(format!("Executable {how} into game_dir"));
                    }
                }
            }
            let _ = st.settings.save();
            st.refresh_diagnostics(false);
            let mut summary = if actions.is_empty() {
                "All automatic optimizations are already applied.".to_string()
            } else {
                format!("Auto Enhance applied:\n  - {}", actions.join("\n  - "))
            };
            if !st.sys.tdr_safe() {
                if let Ok(reg_path) = export_tdr_reg() {
                    summary.push_str(&format!(
                        "\n\nWindows GPU TdrDelay is still {}s (recommended 15s).\nApply fix-tdr-15s.reg now? (Admin prompt; reboot after.)",
                        st.sys.tdr_delay_secs.unwrap_or(2)
                    ));
                    let ans = show_msg(hwnd, &summary, "Auto Enhance Complete + TDR 15s", MB_ICONINFORMATION | MB_YESNO);
                    if ans == IDYES {
                        open_path(hwnd, &reg_path);
                    }
                }
            } else {
                show_msg(hwnd, &summary, "Auto Enhance Complete", MB_ICONINFORMATION | MB_OK);
            }
            st.set_toast("Auto Enhance complete â€” system optimized for runner!");
            redraw(hwnd);
        }
        Target::RumblePad(slot) => {
            if rumble_xinput_pad(slot, 320) {
                st.set_toast(format!("Rumble pulse sent to XInput Pad #{}", slot + 1));
            } else {
                st.set_toast("Controller did not respond to rumble");
            }
            redraw(hwnd);
        }
        Target::SetLogFilter(f) => {
            st.log_filter = f;
            st.log_scroll = 0;
            redraw(hwnd);
        }
        Target::CopyLogSlice => {
            let start = st.log_lines.len().saturating_sub(80);
            let snippet = st.log_lines[start..].join("\n");
            if copy_to_clipboard(hwnd, &snippet) {
                st.set_toast("Copied last 80 log lines to clipboard!");
            } else {
                st.set_toast("Could not copy to clipboard");
            }
            redraw(hwnd);
        }
        Target::ToggleConsoleQuick => {
            let next_s = !st.settings.is_series_s();
            st.settings.console_model = if next_s { "series-s".into() } else { "series-x".into() };
            st.save_settings(if next_s { "Switched to Xbox Series S mode" } else { "Switched to Xbox Series X mode" });
            redraw(hwnd);
        }
        Target::ToggleHudOverlay => {
            st.settings.hud_overlay = !st.settings.hud_overlay;
            if !st.settings.hud_overlay {
                if let Some(ref mut hud) = st.hud {
                    hud.hide();
                }
            }
            st.save_settings(if st.settings.hud_overlay {
                "In-game click-through FPS/RAM HUD enabled"
            } else {
                "In-game HUD overlay disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleAutoBackup => {
            st.settings.auto_backup_saves = !st.settings.auto_backup_saves;
            st.save_settings(if st.settings.auto_backup_saves {
                "Auto-backup saves before launch enabled"
            } else {
                "Auto-backup saves before launch disabled"
            });
            redraw(hwnd);
        }
        Target::RestoreLatestSave => {
            if let Some((name, _)) = saves::latest_backup() {
                let ans = show_msg(
                    hwnd,
                    &format!("Restore latest save snapshot '{name}' back into %LOCALAPPDATA%\\victory\\data?\n\nCurrent save files will be overwritten."),
                    "Restore Save Backup",
                    MB_ICONQUESTION | MB_YESNO,
                );
                if ans == IDYES {
                    match saves::restore_latest_backup() {
                        Ok((bname, n)) => {
                            st.set_toast(format!("Restored {n} save file(s) from {bname}"));
                        }
                        Err(e) => {
                            show_msg(hwnd, &e, "Restore Failed", MB_ICONERROR | MB_OK);
                        }
                    }
                }
            } else {
                show_msg(hwnd, "No save backups found yet. Click BACKUP SAVES first.", "Restore Save Backup", MB_ICONINFORMATION | MB_OK);
            }
            redraw(hwnd);
        }
        Target::OpenModsDir => {
            open_path(hwnd, &saves::mods_dir());
        }
        Target::CycleHudMode => {
            let lbl = st.settings.cycle_hud_mode();
            if let Some(ref mut hud) = st.hud {
                hud.set_mode(st.settings.hud_mode);
            }
            st.save_settings(&format!("HUD overlay mode: {lbl}"));
            redraw(hwnd);
        }
        Target::CycleKeymapPreset => {
            let lbl = st.settings.cycle_keymap_preset();
            st.save_settings(&format!("Keymap standard: {lbl}"));
            redraw(hwnd);
        }
        Target::CycleAbExperiment => {
            let lbl = st.settings.cycle_ab_experiment();
            st.save_settings(&format!("Live Baseline {lbl}"));
            redraw(hwnd);
        }
        Target::SafeBootRollback => {
            match crate::guardian::restore_last_known_good() {
                Ok(restored) => {
                    st.settings = restored;
                    st.last_crash = None;
                    st.set_toast("Restored Last-Known-Good settings (Safe Boot ready)");
                    st.refresh_diagnostics(false);
                }
                Err(e) => st.set_toast(e),
            }
            redraw(hwnd);
        }
        Target::CycleHudCorner => {
            let lbl = st.settings.cycle_hud_corner();
            if let Some(ref mut hud) = st.hud {
                hud.set_corner(st.settings.hud_corner);
            }
            st.save_settings(&format!("In-game HUD position set to {lbl}"));
            redraw(hwnd);
        }
        Target::EditCustomEnv => {
            let cur = st.settings.custom_env.clone();
            if let Some(Some(val)) = run_modal(hwnd, || {
                prompt_text_input(
                    "Edit Custom Environment Variables",
                    "Semicolon-separated KEY=VALUE pairs (e.g. VICTORY_D3D12_DEBUG=1):",
                    &cur,
                )
            }) {
                let st = state(hwnd);
                let slug = runner_slug(&st.settings.resolved_runner(), st.settings.active_runner_kind());
                st.settings.custom_env = val.clone();
                st.settings.set_runner_env_for(&slug, &val);
                st.save_settings("Per-runner environment variables saved");
                redraw(hwnd);
            }
        }
        Target::EditTitleArgs => {
            let cur = st.settings.title_args.clone();
            if let Some(Some(val)) = run_modal(hwnd, || {
                prompt_text_input(
                    "Edit Title Launch Arguments",
                    "Extra CLI flags passed to the title executable after -- :",
                    &cur,
                )
            }) {
                let st = state(hwnd);
                st.settings.title_args = val;
                st.save_settings("Title launch arguments saved");
                redraw(hwnd);
            }
        }
        Target::ExportCacheZip => {
            let cdir = cache_dir_for(
                &st.settings.resolved_runner(),
                st.settings.active_runner_kind(),
                st.settings.isolated_cache,
            );
            st.set_toast("Compressing active shader cache to backups\\...");
            redraw(hwnd);
            match saves::export_cache_zip(&cdir) {
                Ok(zp) => {
                    st.set_toast("Shader cache backed up to backups\\");
                    show_msg(hwnd, &format!("Shader & pipeline cache exported to:\n{}", zp.display()), "Cache Backup Ready", MB_ICONINFORMATION | MB_OK);
                }
                Err(e) => {
                    show_msg(hwnd, &e, "Cache Export Failed", MB_ICONERROR | MB_OK);
                }
            }
            redraw(hwnd);
        }
        Target::ToggleMod(idx) => {
            if let Some(m) = st.mods.get(idx).cloned() {
                match saves::toggle_mod(&m.path) {
                    Ok(en) => {
                        st.mods = saves::list_mods(8);
                        st.mod_count = st.mods.len();
                        st.set_toast(format!("{} {}", if en { "Enabled" } else { "Disabled" }, m.name));
                    }
                    Err(e) => {
                        show_msg(hwnd, &e, "Toggle Mod Failed", MB_ICONERROR | MB_OK);
                    }
                }
                redraw(hwnd);
            }
        }
        Target::CaptureScreenshot => {
            let cap = crate::diagnostics::captures_dir().join(format!("GM6-{}.bmp", local_timestamp_filename()));
            match capture_screen_bmp(&cap) {
                Ok(()) => {
                    st.set_toast(format!("Screenshot saved: captures\\{}", cap.file_name().unwrap_or_default().to_string_lossy()));
                    if let Some(ref mut hud) = st.hud {
                        if st.settings.hud_overlay && st.session.is_some() {
                            hud.update("Title · SCREENSHOT SAVED (captures\\)", rgb(34, 211, 138));
                        }
                    }
                }
                Err(e) => st.set_toast(e),
            }
            redraw(hwnd);
        }
        Target::CreateDesktopShortcut => {
            let exe = std::env::current_exe().unwrap_or_else(|_| Settings::app_dir().join("Game Master.exe"));
            let ico = Settings::app_dir().join("logo.ico");
            match create_desktop_shortcut(&exe, &ico) {
                Ok(lnk) => {
                    st.set_toast("Desktop shortcut created!");
                    show_msg(hwnd, &format!("Created Desktop shortcut:\n{lnk}"), "Desktop Shortcut Ready", MB_ICONINFORMATION | MB_OK);
                }
                Err(e) => {
                    show_msg(hwnd, &e, "Shortcut Failed", MB_ICONERROR | MB_OK);
                }
            }
            redraw(hwnd);
        }
        Target::CycleInputMode => {
            let lbl = st.settings.cycle_input_mode();
            st.save_settings(&format!("Input device mode: {lbl}"));
            redraw(hwnd);
        }
        Target::RemapKeybind(idx) => {
            if let Some(&(_, label, _)) = crate::config::KEYBIND_SLOTS.get(idx) {
                st.listening_keybind = Some(idx);
                st.set_toast(format!("Press any key for '{label}' (Esc to cancel)..."));
                redraw(hwnd);
            }
        }
        Target::ResetKeybinds => {
            st.settings.keymap = Settings::default_keymap_string();
            st.save_settings("Keyboard stand-in keybinds reset to defaults");
            redraw(hwnd);
        }
        Target::CycleRelayMode => {
            let lbl = st.settings.cycle_relay_mode();
            st.save_settings(&format!("Async GPU Relay (VICTORY_RELAY): {lbl}"));
            redraw(hwnd);
        }
        Target::ToggleWideBack => {
            st.settings.wide_back = !st.settings.wide_back;
            st.save_settings(if st.settings.wide_back {
                "Parallel readback enabled (VICTORY_WIDE_BACK=1)"
            } else {
                "Parallel readback disabled"
            });
            redraw(hwnd);
        }
        Target::CyclePipelineWait => {
            let ms = st.settings.cycle_pipeline_wait();
            if ms > 0 {
                st.save_settings(&format!("Per-frame pipeline wait budget: {ms}ms"));
            } else {
                st.save_settings("Per-frame pipeline wait budget: default");
            }
            redraw(hwnd);
        }
        Target::ToggleBvhBuilder => {
            st.settings.bvh_builder = !st.settings.bvh_builder;
            st.save_settings(if st.settings.bvh_builder {
                "Experimental Ray-Tracing BVH Builder enabled (VICTORY_BVH_BUILDER=1)"
            } else {
                "Ray-Tracing BVH Builder disabled (safe default)"
            });
            redraw(hwnd);
        }
        Target::ToggleAsyncSubmit => {
            st.settings.async_submit = !st.settings.async_submit;
            st.save_settings(if st.settings.async_submit {
                "Async GPU submit enabled (VICTORY_ASYNC_SUBMIT=1 â€” A/B test one switch at a time)"
            } else {
                "Async GPU submit disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleQueue2 => {
            st.settings.queue2 = !st.settings.queue2;
            st.save_settings(if st.settings.queue2 {
                "Second copy queue enabled (VICTORY_QUEUE2=3)"
            } else {
                "Second copy queue disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleIndirectWindow => {
            st.settings.indirect_window = !st.settings.indirect_window;
            st.save_settings(if st.settings.indirect_window {
                "Indirect draw window enabled (VICTORY_INDIRECT_WINDOW=1)"
            } else {
                "Indirect draw window disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleTextureNoWait => {
            st.settings.texture_no_wait = !st.settings.texture_no_wait;
            st.save_settings(if st.settings.texture_no_wait {
                "Texture no-wait enabled (VICTORY_TEXTURE_NO_WAIT=1)"
            } else {
                "Texture no-wait disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleMergePasses => {
            st.settings.merge_passes = !st.settings.merge_passes;
            st.save_settings(if st.settings.merge_passes {
                "Shadow merge enabled (VICTORY_MERGE_GAP=64 VICTORY_MERGE_RUNS=4096)"
            } else {
                "Shadow merge disabled"
            });
            redraw(hwnd);
        }
        Target::ResetV11Perf => {
            st.settings.async_submit = false;
            st.settings.queue2 = false;
            st.settings.indirect_window = false;
            st.settings.texture_no_wait = false;
            st.settings.merge_passes = false;
            st.settings.wide_refresh = false;
            st.settings.wide_merge = false;
            st.settings.inplace_buf = false;
            st.settings.profiler_on = false;
            st.save_settings("All experimental runner perf switches set to OFF");
            redraw(hwnd);
        }
        Target::ToggleDisplay1080p => {
            st.settings.display_1080p = !st.settings.display_1080p;
            st.save_settings(if st.settings.display_1080p {
                "1080p cap enabled (VICTORY_DISPLAY_MODE=1920x1080 â€” 4x fewer pixels than 4K)"
            } else {
                "Display mode set to native 4K (3840x2160)"
            });
            redraw(hwnd);
        }
        Target::ToggleWideRefresh => {
            st.settings.wide_refresh = !st.settings.wide_refresh;
            st.save_settings(if st.settings.wide_refresh {
                "8-thread copy refresh enabled (VICTORY_WIDE_REFRESH=1)"
            } else {
                "Wide copy refresh disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleWideMerge => {
            st.settings.wide_merge = !st.settings.wide_merge;
            st.save_settings(if st.settings.wide_merge {
                "8-thread GPU writeback merge enabled (VICTORY_WIDE_MERGE=1)"
            } else {
                "Wide merge disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleInplaceBuf => {
            st.settings.inplace_buf = !st.settings.inplace_buf;
            st.save_settings(if st.settings.inplace_buf {
                "In-place buffer mapping enabled (VICTORY_INPLACE=1 â€” experimental)"
            } else {
                "In-place buffer mapping disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleKbmCrosshair => {
            st.settings.kbm_crosshair = !st.settings.kbm_crosshair;
            if !st.settings.kbm_crosshair {
                if let Some(ref mut ch) = st.crosshair {
                    ch.hide();
                }
            }
            st.save_settings(if st.settings.kbm_crosshair {
                "Standard center reticle + auto menu cursor (Esc/Tab/F8) enabled"
            } else {
                "Standard center reticle disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleProfiler => {
            st.settings.profiler_on = !st.settings.profiler_on;
            st.save_settings(if st.settings.profiler_on {
                "Built-in CPU profiler enabled (VICTORY_PROFILE=1)"
            } else {
                "Built-in CPU profiler disabled"
            });
            redraw(hwnd);
        }
        Target::ToggleAmbientFill => {
            st.settings.ambient_fill = !st.settings.ambient_fill;
            if st.settings.ambient_fill {
                if st.settings.ambient_val.trim().is_empty() || st.settings.ambient_val.trim().eq_ignore_ascii_case("off") {
                    st.settings.ambient_val = "0.8".into();
                }
                if st.settings.ambient_air.trim().is_empty() || st.settings.ambient_air.trim() == "0" {
                    st.settings.ambient_air = "1.5".into();
                }
            }
            st.save_settings(if st.settings.ambient_fill {
                "Ambient GI shade fill + crease AO enabled (VICTORY_AMBIENT=0.8)"
            } else {
                "Ambient GI shade fill disabled (VICTORY_AMBIENT=off)"
            });
            redraw(hwnd);
        }
        Target::ToggleHoistClamps => {
            st.settings.hoist_clamps = !st.settings.hoist_clamps;
            st.save_settings(if st.settings.hoist_clamps {
                "Hoisted indirect clamps enabled (VICTORY_HOIST_CLAMPS=7)"
            } else {
                "Hoisted indirect clamps disabled (VICTORY_HOIST_CLAMPS=0)"
            });
            redraw(hwnd);
        }
        Target::ToggleFastCores => {
            st.settings.fast_cores = !st.settings.fast_cores;
            st.save_settings(if st.settings.fast_cores {
                "Fast-core thread affinity enabled (VICTORY_FAST_CORES=1)"
            } else {
                "Fast-core thread affinity disabled (VICTORY_FAST_CORES=0)"
            });
            redraw(hwnd);
        }
        Target::ToggleNoWaitResident => {
            st.settings.no_wait_resident = !st.settings.no_wait_resident;
            st.save_settings(if st.settings.no_wait_resident {
                "No Residency Wait ON (VICTORY_WAIT_RESIDENT=0 — skips map stalls)"
            } else {
                "No Residency Wait OFF (default runner residency wait)"
            });
            redraw(hwnd);
        }
        Target::CycleHomeRunner => {
            if !st.runners.is_empty() {
                let cur = st.settings.resolved_runner();
                let idx = st.runners.iter().position(|r| r.path == cur).unwrap_or(0);
                let next = &st.runners[(idx + 1) % st.runners.len()];
                st.settings.runner = next.path.to_string_lossy().to_string();
                st.settings.runner_kind = next.kind.id().into();
                st.save_settings(&format!("Selected runner: {}", next.name));
                redraw(hwnd);
            }
        }
        Target::ImportRunner => {
            if let Some(Some(p)) = run_modal(hwnd, || {
                browse_file(
                    hwnd,
                    "Import Runner Executable (victory.exe)",
                    &[("Executables (*.exe)", "*.exe"), ("All Files", "*.*")],
                    "",
                )
            }) {
                let st = state(hwnd);
                match crate::runners::import_runner_exe(std::path::Path::new(&p)) {
                    Ok((name, dst)) => {
                        st.settings.runner = dst.to_string_lossy().to_string();
                        st.settings.runner_kind = crate::runners::RunnerKind::detect_from_path(&dst).id().into();
                        st.refresh_diagnostics(true);
                        st.save_settings(&format!("Imported runner '{name}' into runners\\{name}\\victory.exe"));
                    }
                    Err(e) => {
                        show_msg(hwnd, &e, "Import Runner Failed", MB_ICONERROR | MB_OK);
                    }
                }
                redraw(hwnd);
            }
        }
        Target::CycleDisplayMode => {
            let next = match st.settings.display_mode.trim() {
                "auto" => "1280x720",
                "1280x720" => "1600x900",
                "1600x900" => "1920x1080",
                "1920x1080" => "2560x1440",
                "2560x1440" => "3840x2160",
                _ => "auto",
            };
            st.settings.display_mode = next.into();
            st.settings.display_1080p = next == "1920x1080";
            st.save_settings(&format!("Display resolution set to {}", st.settings.display_mode_label()));
            redraw(hwnd);
        }
        Target::ToggleBorderless => {
            st.settings.borderless = !st.settings.borderless;
            st.settings.start_maximized = st.settings.borderless;
            st.save_settings(if st.settings.borderless {
                "Display mode: Borderless Fullscreen"
            } else {
                "Display mode: Windowed"
            });
            redraw(hwnd);
        }
        Target::ToggleThinLists => {
            st.settings.thin_lists = !st.settings.thin_lists;
            st.save_settings(if st.settings.thin_lists {
                "Parallel draw-call translation enabled (VICTORY_THIN=1 â€” experimental)"
            } else {
                "Parallel draw-call translation disabled (VICTORY_THIN=0)"
            });
            redraw(hwnd);
        }
        Target::CycleQueue2Landing => {
            let next = match st.settings.queue2_landing_mb.trim() {
                "48" => "96",
                "96" => "128",
                _ => "48",
            };
            st.settings.queue2_landing_mb = next.into();
            st.save_settings(&format!("VICTORY_QUEUE2_LANDING_MB={next}"));
            redraw(hwnd);
        }
        Target::ToggleMouseAim => {
            st.settings.mouse_aim = !st.settings.mouse_aim;
            st.save_settings(if st.settings.mouse_aim {
                "Mouse Aim camera + LMB/RMB triggers enabled (VICTORY_MOUSE_AIM=1)"
            } else {
                "Mouse Aim disabled (using keyboard camera binds)"
            });
            redraw(hwnd);
        }
        Target::CycleMouseSens => {
            let lbl = st.settings.cycle_mouse_sens();
            st.save_settings(&format!("Mouse Aim sensitivity set to {lbl}"));
            redraw(hwnd);
        }
        Target::ToggleMouseLock => {
            st.settings.mouse_lock = !st.settings.mouse_lock;
            st.save_settings(if st.settings.mouse_lock {
                "360-degree Mouse Lock enabled (press F8 in-game to pause/release)"
            } else {
                "Mouse Lock disabled (free cursor)"
            });
            redraw(hwnd);
        }
        Target::RunShaderCensus => {
            if st.census_rx.is_none() {
                st.start_shader_census_bg();
                st.set_toast("Running live shader census (translate --list)...");
                redraw(hwnd);
            }
        }
        Target::CycleTextureRoom => {
            let v = st.settings.cycle_texture_room();
            st.save_settings(&format!("Texture room ceiling set to {v} (VICTORY_TEXTURE_ROOM_PERCENT)"));
            redraw(hwnd);
        }
        Target::ToggleAdvSection(idx) => {
            if idx < st.adv_collapsed.len() {
                st.adv_collapsed[idx] = !st.adv_collapsed[idx];
                redraw(hwnd);
            }
        }
        Target::BenchmarkRun => {
            if st.session.is_some() {
                st.set_toast("A session is already running");
            } else {
                st.benchmark_restore_log = Some(st.settings.log_level.clone());
                st.settings.log_level = "info".into();
                handle_launch_or_stop(hwnd);
                let st = state(hwnd);
                if st.session.is_some() {
                    st.set_toast("Benchmark Run started (VICTORY_LOG=info; restores warn on exit)");
                } else {
                    st.settings.log_level = st.benchmark_restore_log.take().unwrap_or_else(|| "warn".into());
                }
            }
            redraw(hwnd);
        }
        Target::ToggleCompareLogs => {
            st.show_log_compare = !st.show_log_compare;
            if st.show_log_compare {
                st.log_comparison = telemetry::compare_last_two_logs();
            }
            redraw(hwnd);
        }
    }
}


unsafe fn on_timer(hwnd: HWND) {
    let st = state(hwnd);
    if st.modal_open {
        return;
    }
    let mut changed = false;

    if let Some(ref prx) = st.portwatch_rx {
        if let Ok(snap) = prx.try_recv() {
            let count = snap.projects.len();
            let err_opt = snap.error.clone();
            st.portwatch = snap;
            st.portwatch_rx = None;
            if let Some(err) = err_opt {
                st.set_toast(format!("PortWatch VI sync failed: {err}"));
            } else if count > 0 {
                st.set_toast(format!("PortWatch VI synced ({count} projects)"));
            } else {
                st.set_toast("PortWatch VI sync failed — no projects returned");
            }
            changed = true;
        }
    }

    if let Some(ref crx) = st.census_rx {
        if let Ok(res) = crx.try_recv() {
            st.census_rx = None;
            match res {
                Ok(c) => {
                    st.set_toast(format!("Shader census complete: {}/{} translated", c.translated, c.total));
                    st.shader_census = c;
                }
                Err(e) => {
                    st.set_toast(e);
                }
            }
            changed = true;
        }
    }

    if let Some(ref grx) = st.gpu_rx {
        if let Ok((gname, gdrv, vram)) = grx.try_recv() {
            st.sys.gpu_arch = crate::diagnostics::GpuArch::detect(&gname);
            st.sys.gpu_name = gname;
            st.sys.gpu_driver = gdrv;
            st.sys.vram_gb = vram;
            st.gpu_rx = None;
            if st.is_first_run {
                st.is_first_run = false;
                let _ = crate::presets::apply_adaptive_preset(
                    &mut st.settings,
                    crate::presets::PresetKind::Balanced,
                    &st.sys,
                );
                let _ = st.settings.save();
            }
            st.start_tip_probe();
            changed = true;
        }
    }

    if let Some(ref trx) = st.tip_rx {
        if let Ok(tip) = trx.try_recv() {
            if st.system_tip != tip {
                st.system_tip = tip;
                changed = true;
            }
            st.tip_rx = None;
        }
    }

    let paused_for_game = st.session.is_some() && st.settings.zero_overhead;
    if !paused_for_game && st.last_tip_probe.elapsed() >= Duration::from_secs(10) {
        st.start_tip_probe();
    }
    if !paused_for_game && st.last_headroom_sample.elapsed() >= Duration::from_secs(2) {
        st.last_headroom_sample = Instant::now();
        let free_ram_pct = (100u32).saturating_sub(st.sys.mem_load_pct.min(100));
        let com_gb = st.sys.avail_commit_gb.round().clamp(0.0, 100.0) as u32;
        let vram_pct: u32 = st.settings.texture_room_percent.trim().parse().unwrap_or(90).clamp(0, 100);
        for (vec, val) in [
            (&mut st.free_ram_history, free_ram_pct),
            (&mut st.avail_commit_history, com_gb),
            (&mut st.vram_headroom_history, vram_pct),
        ] {
            vec.push(val);
            if vec.len() > 60 {
                vec.remove(0);
            }
        }
        if st.view == 4 {
            changed = true;
        }
    }

    if let Some(ref rx) = st.scan_rx {
        if let Ok((bytes, count)) = rx.try_recv() {
            if bytes > 0 {
                st.data_label = format!("{} DATA", format_bytes(bytes));
                st.file_count_label = format!("{count} FILES");
            } else {
                st.data_label = "DATA NOT FOUND".into();
                st.file_count_label = "0 FILES".into();
            }
            st.scan_rx = None;
            changed = true;
        }
    }

    if st.session.is_some() {
        st.anim_tick = st.anim_tick.wrapping_add(1);
        if IsIconic(hwnd) == 0 && st.anim_tick % 2 == 0 {
            changed = true;
        }

        let poll_res = if let Some(ref mut sess) = st.session {
            if sess.kind == launcher::ProcessKind::Game && (!sess.window_branded || st.anim_tick % 10 == 0) {
                let ico = Settings::app_dir().join("logo.ico");
                if brand_runner_window(sess.child.id(), "Game Master 6 - Title", &ico, true) {
                    sess.window_branded = true;
                }
                if !sess.window_maximized {
                    let ok = if st.settings.borderless {
                        make_runner_window_borderless_fullscreen(sess.child.id())
                    } else {
                        maximize_runner_window(sess.child.id())
                    };
                    if ok {
                        sess.window_maximized = true;
                    }
                }
            }
            if st.anim_tick % 6 == 0 {
                st.proc_stats = monitor::sample_process(sess.child.id());
            }
            Some(launcher::poll(sess))
        } else {
            None
        };

        if st.settings.hud_overlay {
            if let (Some(ref sess), Some(ref mut hud)) = (&st.session, &mut st.hud) {
                let (ws_gb, commit_gb) = st
                    .proc_stats
                    .map(|ps| (ps.working_set_gb, ps.private_commit_gb))
                    .unwrap_or((0.0, 0.0));
                if let Some(ft) = sess.live_telemetry {
                    let scaled_fps = (ft.fps * 10.0).clamp(0.0, 1200.0) as u32;
                    if st.fps_history.last().copied() != Some(scaled_fps) || st.fps_history.len() < 2 {
                        push_history(&mut st.fps_history, scaled_fps);
                    }
                    let col = if ft.fps >= 25.0 {
                        rgb(34, 211, 138)
                    } else if ft.fps >= 15.0 {
                        rgb(0, 240, 255)
                    } else if ft.fps >= 10.0 {
                        rgb(255, 190, 60)
                    } else {
                        rgb(255, 77, 109)
                    };
                    hud.update_telemetry(
                        st.settings.hud_mode,
                        ft.fps,
                        ft.frame_ms,
                        ft.gpu_ms,
                        ft.draws,
                        ws_gb,
                        commit_gb,
                        col,
                    );
                } else {
                    let secs = sess.started_at.elapsed().as_secs();
                    let ram_str = if ws_gb > 0.01 { format!("{ws_gb:.1}G") } else { "-".into() };
                    hud.update(
                        &format!("Title · {} {:02}:{:02} · RAM {ram_str}", sess.kind.label(), secs / 60, secs % 60),
                        rgb(0, 240, 255),
                    );
                }
            }
        }
        if let Some(ref mut ch) = st.crosshair {
            ch.sync(st.settings.kbm_crosshair);
        }
        if let Some(update) = poll_res {
            if !update.new_lines.is_empty() {
                st.log_lines.extend(update.new_lines);
                if st.log_lines.len() > 600 {
                    let drop_n = st.log_lines.len() - 600;
                    st.log_lines.drain(0..drop_n);
                }
            }
            if let Some(fin) = update.finished {
                st.session = None;
                if let Some(ref mut ch) = st.crosshair {
                    ch.hide();
                }
                st.proc_stats = None;
                if let Some(ref mut hud) = st.hud {
                    hud.hide();
                }
                KillTimer(hwnd, TIMER_ID);
                enable_dark_titlebar(hwnd);
            RegisterHotKey(hwnd, 901, MOD_NOREPEAT, VK_F9);
            RegisterHotKey(hwnd, 902, MOD_NOREPEAT, VK_F10);
            RegisterHotKey(hwnd, 903, MOD_NOREPEAT, VK_F11);
            RegisterHotKey(hwnd, 904, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, 0x4B);
        RegisterHotKey(hwnd, 905, MOD_NOREPEAT, 0x77);
            SetTimer(hwnd, TIMER_ID, 150, None);
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                st.recent_logs = list_recent_logs(12);
                st.slow_shaders = extract_slow_shaders(&st.log_lines, 6);
                let rname = st.settings.resolved_runner().file_name().and_then(|s| s.to_str()).unwrap_or("runner").to_string();
                let rec = telemetry::build_record(
                    &rname,
                    st.settings.active_runner_kind().id(),
                    &st.settings.console_model,
                    fin.kind.label(),
                    fin.elapsed_secs,
                    fin.exit_code,
                    &st.log_lines,
                    &st.slow_shaders,
                );
                telemetry::append_session(&rec);
                telemetry::sync_sessions_from_logs();
                st.recent_sessions = telemetry::load_recent_sessions(8);
                let installed: Vec<String> = st
                    .runners
                    .iter()
                    .flat_map(|r| {
                        let mut v = vec![r.name.clone()];
                        if let Some(f) = r.path.file_name().and_then(|s| s.to_str()) {
                            v.push(f.to_string());
                        }
                        v
                    })
                    .collect();
                st.runner_benches = telemetry::aggregate_by_runner(&installed);
                let parsed_fps = telemetry::parse_fps_series(&st.log_lines);
                if !parsed_fps.is_empty() {
                    st.fps_history = parsed_fps;
                }
                if st.benchmark_restore_log.take().is_some() {
                    st.settings.log_level = "warn".into();
                    let _ = st.settings.save();
                }
                st.log_comparison = telemetry::compare_last_two_logs();
                let active_cache = cache_dir_for(
                    &st.settings.resolved_runner(),
                    st.settings.active_runner_kind(),
                    st.settings.isolated_cache,
                );
                if fin.kind == ProcessKind::Prepare && fin.exit_code == Some(0) {
                    crate::runners::write_cache_hash(&st.settings.resolved_runner(), &active_cache);
                    st.cache_hash_mismatch = false;
                } else if fin.kind == ProcessKind::Game {
                    let crashed = fin.exit_code.unwrap_or(0) != 0;
                    let doc = telemetry::analyze_session_log(&rname, &fin.log_path, &st.sys, crashed);
                    let base_fps = st.runner_benches.first().map(|b| b.avg_fps).unwrap_or(0.0);
                    st.beats_baseline = telemetry::benchmark_beats_baseline(doc.avg_fps, base_fps, crashed);
                    st.auto_doctor = Some(doc);
                }
                let (p_secs, p_cnt) = telemetry::total_playtime_summary();
                st.playtime_label = format!("PLAYTIME {} ({} RUNS)", telemetry::format_playtime(p_secs), p_cnt);
                if st.audit.shader_cache_ready {
                    st.start_shader_census_bg();
                }
                if fin.kind == ProcessKind::Game {
                    crate::guardian::record_known_good_if_stable(fin.elapsed_secs, fin.exit_code);
                    if fin.diagnosis.is_none() {
                        if let Some(verdict) = crate::guardian::diagnose_crash(fin.exit_code, fin.elapsed_secs, &st.log_lines) {
                            let code_tag = fin.exit_code.map(|c| format!("0x{:08X}", c as u32)).unwrap_or_else(|| "EARLY".into());
                            st.last_crash = Some((
                                code_tag,
                                format!("{}: {} — {}", verdict.category, verdict.cause, verdict.recommended_fix),
                                fin.log_path.clone(),
                            ));
                        }
                    }
                }
                if let Some(diag) = fin.diagnosis {
                    let code_tag = fin
                        .exit_code
                        .map(|c| format!("0x{:08X}", c as u32))
                        .unwrap_or_else(|| "ERR".into());
                    st.status_text = format!("EXIT {code_tag}");
                    st.status_kind = StatusKind::Error;
                    st.selected_log = 0;
                    st.last_crash = Some((code_tag.clone(), diag, fin.log_path.clone()));
                    st.set_toast(format!("Session exited ({code_tag}) — see Crash Summary"));
                    redraw(hwnd);
                } else {
                    st.status_text = "COMPLETED".into();
                    st.status_kind = StatusKind::Ready;
                    st.set_toast(format!(
                        "{} finished in {}m {:02}s",
                        fin.kind.label(),
                        fin.elapsed_secs / 60,
                        fin.elapsed_secs % 60
                    ));
                }
            }
        }
    } else if st.anim_tick % 20 == 0 {
        let (prev_gpu, prev_drv, prev_vram) = (st.sys.gpu_name.clone(), st.sys.gpu_driver.clone(), st.sys.vram_gb);
        st.sys = SystemInfo::collect(&st.settings.game_dir);
        st.sys.gpu_name = prev_gpu;
        st.sys.gpu_driver = prev_drv;
        st.sys.vram_gb = prev_vram;
        push_history(&mut st.ram_history, st.sys.mem_load_pct.min(100));
        push_history(&mut st.commit_history, commit_pct(&st.sys));
        changed = true;
    }

    if st.session.is_none() {
        st.anim_tick = st.anim_tick.wrapping_add(1);
    }

    if st.view == 1 && st.anim_tick % 30 == 0 && st.session.is_none() {
        st.runners = discover_runners(&st.settings);
        changed = true;
    }

    if changed && IsIconic(hwnd) == 0 {
        redraw(hwnd);
    }
}


fn hit_test(hits: &[(RECT, Target)], x: i32, y: i32) -> Target {
    for &(r, t) in hits.iter().rev() {
        if r.contains(x, y) {
            return t;
        }
    }
    Target::None
}

fn unpack_xy(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam as u32 & 0xffff) as i16 as i32;
    let y = ((lparam as u32 >> 16) & 0xffff) as i16 as i32;
    (x, y)
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let dir = Settings::app_dir();
            let fonts = Fonts::load(&dir);

            // Load multi-size icon via native LoadImageW
            let ico_path = wide(&dir.join("logo.ico").to_string_lossy());
            let big_ico = LoadImageW(std::ptr::null_mut(), ico_path.as_ptr(), IMAGE_ICON, 48, 48, LR_LOADFROMFILE);
            let sm_ico = LoadImageW(std::ptr::null_mut(), ico_path.as_ptr(), IMAGE_ICON, 16, 16, LR_LOADFROMFILE);
            if !big_ico.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG, big_ico as LPARAM);
            }
            if !sm_ico.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL, sm_ico as LPARAM);
            }

            let settings = Settings::load();
            let sys = SystemInfo::collect(&settings.game_dir);
            let audit = AssetAudit::scan(&settings, &sys);
            let runners = discover_runners(&settings);
            let recent_logs = list_recent_logs(12);
            let log_lines = recent_logs
                .first()
                .map(|e| read_log_tail(&e.path, 250))
                .unwrap_or_default();

            let (status_text, status_kind) = if audit.passed == audit.total {
                ("SYSTEM READY".into(), StatusKind::Ready)
            } else {
                (format!("CHECK {}/{}", audit.passed, audit.total), StatusKind::Warning)
            };

            let init_ram = sys.mem_load_pct.min(100);
            let init_commit = commit_pct(&sys);
            let mut st = Box::new(AppState {
                settings,
                sys,
                audit,
                session: None,
                view: 0,
                hover: Target::None,
                hits: Vec::new(),
                status_text,
                status_kind,
                data_label: "SCANNING DATA...".into(),
                file_count_label: "...".into(),
                scan_rx: None,
                recent_logs,
                selected_log: 0,
                log_lines,
                log_scroll: 0,
                log_filter: 0,
                anim_tick: 0,
                toast: None,
                fonts,
                logo: None,
                tracking_mouse: false,
                hand_cursor: LoadCursorW(std::ptr::null_mut(), IDC_HAND as *const u16),
                arrow_cursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW as *const u16),
                runners,
                portwatch: PortWatchSnapshot::default(),
                portwatch_rx: None,
                portwatch_scroll: 0,
                runner_scroll: 0,
                tuning_scroll: 0,
                last_mouse_x: 0,
                backup_count: saves::count_backups(),
                slow_shaders: Vec::new(),
                ram_history: vec![init_ram; 2],
                commit_history: vec![init_commit; 2],
                recent_sessions: telemetry::load_recent_sessions(8),
                proc_stats: None,
                hud: HudOverlay::create(),
            crosshair: CrosshairOverlay::create(),
                playtime_label: { let (ps, pc) = telemetry::total_playtime_summary(); format!("PLAYTIME {} ({} RUNS)", telemetry::format_playtime(ps), pc) },
                mod_count: saves::count_mods(),
                gpu_rx: {
                    let (gtx, grx) = channel();
                    thread::spawn(move || {
                        let (gn, gd) = probe_primary_gpu();
                        let vram = probe_vram_gb();
                        let _ = gtx.send((gn, gd, vram));
                    });
                    Some(grx)
                },
                fps_history: Vec::new(),
                runner_benches: Vec::new(),
                mods: saves::list_mods(8),
                listening_keybind: None,
                shader_census: ShaderCensus { translated: 0, total: 0, refused: 0, summary: "Baseline translator active. Click RUN CENSUS to scan the local shader archive.".into() },
                census_rx: None,
                last_crash: None,
                dismissed_warning: None,
                adv_collapsed: [false, false, false, false, true],
                system_tip: (false, String::new()),
                tip_rx: None,
                last_tip_probe: Instant::now(),
                is_first_run: !Settings::path().is_file(),
                modal_open: false,
                benchmark_restore_log: None,
                log_comparison: telemetry::compare_last_two_logs(),
                show_log_compare: false,
                free_ram_history: vec![100u32.saturating_sub(init_ram); 2],
                avail_commit_history: vec![init_commit; 2],
                vram_headroom_history: vec![90; 2],
                last_headroom_sample: Instant::now(),
                adv_filter: "ALL",
                auto_doctor: None,
                cache_hash_mismatch: false,
                beats_baseline: false,
            });
            st.cache_hash_mismatch = crate::runners::check_cache_hash_mismatch(
                &st.settings.resolved_runner(),
                &cache_dir_for(
                    &st.settings.resolved_runner(),
                    st.settings.active_runner_kind(),
                    st.settings.isolated_cache,
                ),
            ).is_some();
            st.slow_shaders = extract_slow_shaders(&st.log_lines, 6);
            st.sync_telemetry_and_census(true);
            st.start_tip_probe();
            if !st.settings.offline_mode {
                st.start_portwatch_sync();
            }
            st.start_size_scan();

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(st) as isize);
            DragAcceptFiles(hwnd, 1);
            enable_dark_titlebar(hwnd);
            RegisterHotKey(hwnd, 901, MOD_NOREPEAT, VK_F9);
            RegisterHotKey(hwnd, 902, MOD_NOREPEAT, VK_F10);
            RegisterHotKey(hwnd, 903, MOD_NOREPEAT, VK_F11);
            RegisterHotKey(hwnd, 904, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, 0x4B);
        RegisterHotKey(hwnd, 905, MOD_NOREPEAT, 0x77);
            SetTimer(hwnd, TIMER_ID, 250, None);
            0
        }
        WM_ERASEBKGND => 1,
        WM_GETMINMAXINFO => {
            let mmi = lparam as *mut MINMAXINFO;
            if !mmi.is_null() {
                (*mmi).pt_min_track_size.x = MIN_W;
                (*mmi).pt_min_track_size.y = MIN_H;
            }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut client = RECT::default();
            GetClientRect(hwnd, &mut client);
            let w = client.right.max(1);
            let h = client.bottom.max(1);

            let st = state(hwnd);
            if st.logo.is_none() {
                st.logo = LogoSurface::load(hdc, &Settings::app_dir().join("logo.bmp"));
            }

            let mem = CreateCompatibleDC(hdc);
            let bmp = CreateCompatibleBitmap(hdc, w, h);
            let old = SelectObject(mem, bmp);

            let toast_str = st.active_toast().map(|s| s.to_string());
            let ctx = RenderContext {
                settings: &st.settings,
                sys: &st.sys,
                audit: &st.audit,
                session: st.session.as_ref(),
                view: st.view,
                hover: st.hover,
                status_text: &st.status_text,
                status_kind: st.status_kind,
                data_label: &st.data_label,
                file_count_label: &st.file_count_label,
                anim_tick: st.anim_tick,
                toast: toast_str.as_deref(),
                recent_logs: &st.recent_logs,
                selected_log: st.selected_log,
                log_lines: &st.log_lines,
                log_scroll: st.log_scroll,
                fonts: &st.fonts,
                logo: st.logo.as_ref(),
                runners: &st.runners,
                portwatch: &st.portwatch,
                portwatch_syncing: st.portwatch_rx.is_some(),
                portwatch_scroll: st.portwatch_scroll,
                runner_scroll: st.runner_scroll,
                tuning_scroll: st.tuning_scroll,
                log_filter: st.log_filter,
                playtime_label: &st.playtime_label,
                mod_count: st.mod_count,
                fps_history: &st.fps_history,
                runner_benches: &st.runner_benches,
                mods: &st.mods,
                backup_count: st.backup_count,
                slow_shaders: &st.slow_shaders,
                ram_history: &st.ram_history,
                commit_history: &st.commit_history,
                recent_sessions: &st.recent_sessions,
                proc_stats: st.proc_stats,
                listening_keybind: st.listening_keybind,
                shader_census: &st.shader_census,
                census_running: st.census_rx.is_some(),
                last_crash: st.last_crash.as_ref(),
                dismissed_warning: st.dismissed_warning.as_deref(),
                adv_collapsed: st.adv_collapsed,
                system_tip: &st.system_tip,
                log_comparison: st.log_comparison.as_ref(),
                show_log_compare: st.show_log_compare,
                free_ram_history: &st.free_ram_history,
                avail_commit_history: &st.avail_commit_history,
                vram_headroom_history: &st.vram_headroom_history,
                adv_filter: st.adv_filter,
                auto_doctor: st.auto_doctor.as_ref(),
                cache_hash_mismatch: st.cache_hash_mismatch,
                beats_baseline: st.beats_baseline,
            };
            st.hits = paint(mem, w, h, &ctx);

            BitBlt(hdc, 0, 0, w, h, mem, 0, 0, SRCCOPY);
            SelectObject(mem, old);
            DeleteObject(bmp);
            DeleteDC(mem);
            EndPaint(hwnd, &ps);
            0
        }
        WM_SIZE => {
            redraw(hwnd);
            0
        }
        WM_SETCURSOR => {
            if (lparam as i32 & 0xffff) == HTCLIENT {
                let st = state(hwnd);
                SetCursor(if st.hover != Target::None { st.hand_cursor } else { st.arrow_cursor });
                return 1;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEMOVE => {
            let st = state(hwnd);
            if !st.tracking_mouse {
                let mut tme = TRACKMOUSEEVENT {
                    cb_size: std::mem::size_of::<TRACKMOUSEEVENT>() as DWORD,
                    dw_flags: TME_LEAVE,
                    hwnd_track: hwnd,
                    dw_hover_time: 0,
                };
                TrackMouseEvent(&mut tme);
                st.tracking_mouse = true;
            }
            let (x, y) = unpack_xy(lparam);
            st.last_mouse_x = x;
            let target = hit_test(&st.hits, x, y);
            if st.hover != target {
                st.hover = target;
                redraw(hwnd);
            }
            0
        }
        WM_MOUSELEAVE => {
            let st = state(hwnd);
            st.tracking_mouse = false;
            if st.hover != Target::None {
                st.hover = Target::None;
                redraw(hwnd);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = unpack_xy(lparam);
            let target = hit_test(&state(hwnd).hits, x, y);
            if target != Target::None {
                handle_click(hwnd, target);
            }
            0
        }
        WM_MOUSEWHEEL => {
            let st = state(hwnd);
            let delta = ((wparam >> 16) as u16) as i16;
            if st.view == 4 {
                if delta > 0 {
                    st.log_scroll = st.log_scroll.saturating_add(4);
                } else {
                    st.log_scroll = st.log_scroll.saturating_sub(4);
                }
                redraw(hwnd);
            } else if st.view == 1 {
                let mut cr = RECT::default();
                GetClientRect(hwnd, &mut cr);
                let split_x = 126 + ((cr.right - 142) * 46) / 100;
                if st.last_mouse_x < split_x {
                    if delta > 0 {
                        st.runner_scroll = st.runner_scroll.saturating_sub(1);
                    } else {
                        let max_r = st.runners.len().saturating_sub(1);
                        st.runner_scroll = (st.runner_scroll + 1).min(max_r);
                    }
                } else if delta > 0 {
                    st.portwatch_scroll = st.portwatch_scroll.saturating_sub(2);
                } else {
                    let max_sc = st.portwatch.projects.len().saturating_sub(1);
                    st.portwatch_scroll = (st.portwatch_scroll + 2).min(max_sc);
                }
                redraw(hwnd);
            } else if st.view == 3 {
                let mut cr = RECT::default();
                GetClientRect(hwnd, &mut cr);
                let max = tuning_max_scroll(cr.bottom);
                if delta > 0 {
                    st.tuning_scroll = st.tuning_scroll.saturating_sub(TUNING_SCROLL_STEP);
                } else {
                    st.tuning_scroll = (st.tuning_scroll + TUNING_SCROLL_STEP).min(max);
                }
                redraw(hwnd);
            }
            0
        }
        WM_KEYDOWN => {
            let st = state(hwnd);
            if let Some(slot_idx) = st.listening_keybind {
                let vk = wparam as u8;
                if vk == 0x1B {
                    st.listening_keybind = None;
                    st.set_toast("Keybind capture cancelled");
                    redraw(hwnd);
                    return 0;
                }
                if let Some(kname) = vk_to_bind_name(vk) {
                    if let Some(&(code, label, _)) = crate::config::KEYBIND_SLOTS.get(slot_idx) {
                        st.settings.set_keybind(code, &kname);
                        st.listening_keybind = None;
                        st.save_settings(&format!("Bound {label} -> {}", kname.to_ascii_uppercase()));
                        redraw(hwnd);
                        return 0;
                    }
                }
            }
            let ctrl = (GetKeyState(VK_CONTROL) as u16 & 0x8000) != 0;
            match wparam {
                0x31..=0x37 => {
                    handle_click(hwnd, Target::Nav(wparam - 0x31));
                }
                VK_F5 => handle_click(hwnd, Target::Rescan),
                0x4C if ctrl => handle_launch_or_stop(hwnd), // Ctrl+L
                VK_ESCAPE => {
                    let st = state(hwnd);
                    if st.view != 0 {
                        st.view = 0;
                        redraw(hwnd);
                    }
                }
                _ => {}
            }
            0
        }
        WM_HOTKEY => {
            match wparam {
                901 => handle_click(hwnd, Target::ToggleHudOverlay),
                902 => handle_click(hwnd, Target::CycleHudCorner),
                903 => handle_click(hwnd, Target::CaptureScreenshot),
                904 => {
                    let st = state(hwnd);
                    if st.session.is_some() {
                        handle_launch_or_stop(hwnd);
                    } else {
                        handle_click(hwnd, Target::KillStrayRunners);
                    }
                }
                905 => handle_click(hwnd, Target::ToggleMouseLock),
                _ => {}
            }
            0
        }
        WM_TIMER => {
            on_timer(hwnd);
            0
        }
        WM_CLOSE => {
            let st = state(hwnd);
            if st.session.is_some() {
                let ans = show_msg(
                    hwnd,
                    "A runner session is still active. Closing Game Master will stop it.\n\nClose anyway?",
                    "Active Session",
                    MB_ICONWARNING | MB_YESNO,
                );
                if ans != IDYES {
                    return 0;
                }
            }
            DestroyWindow(hwnd);
            0
        }
        WM_DROPFILES => {
            let paths = read_dropped_paths(wparam as HDROP);
            let st = state(hwnd);
            for p_str in paths {
                let p = PathBuf::from(&p_str);
                if p.is_dir() {
                    let v = p.join("victory.exe");
                    if v.is_file() {
                        st.settings.runner = v.to_string_lossy().into_owned();
                        st.save_settings("Runner set via drag-and-drop");
                    } else {
                        st.settings.game_dir = p_str;
                        st.save_settings("Game folder set via drag-and-drop");
                        st.start_size_scan();
                    }
                } else if p.is_file() && p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
                    let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
                    if fname.contains("victory") {
                        st.settings.runner = p_str;
                        st.save_settings("Runner set via drag-and-drop");
                    } else {
                        st.settings.exe = p_str;
                        if let Some(par) = p.parent() {
                            st.settings.game_dir = par.to_string_lossy().into_owned();
                        }
                        st.save_settings("Game executable set via drag-and-drop");
                        st.start_size_scan();
                    }
                }
            }
            redraw(hwnd);
            0
        }
        WM_DESTROY => {
            UnregisterHotKey(hwnd, 901);
            UnregisterHotKey(hwnd, 902);
            UnregisterHotKey(hwnd, 903);
            UnregisterHotKey(hwnd, 904);
            UnregisterHotKey(hwnd, 905);
            KillTimer(hwnd, TIMER_ID);
            let ptr = state_ptr(hwnd);
            if !ptr.is_null() {
                let mut st = Box::from_raw(ptr);
                let mut rc = RECT::default();
                if IsIconic(hwnd) == 0 && GetWindowRect(hwnd, &mut rc) != 0 {
                    let w = rc.right - rc.left;
                    let h = rc.bottom - rc.top;
                    if w >= MIN_W && h >= MIN_H {
                        st.settings.win_x = rc.left;
                        st.settings.win_y = rc.top;
                        st.settings.win_w = w;
                        st.settings.win_h = h;
                        let _ = st.settings.save();
                    }
                }
                if let Some(ref mut sess) = st.session {
                    launcher::stop(sess);
                }
                if st.settings.auto_clean_strays {
                    let _ = kill_stray_runners();
                }
                if let Some(ref logo) = st.logo {
                    logo.destroy();
                }
                if let Some(ref mut hud) = st.hud {
                    hud.destroy();
                }
                st.fonts.destroy();
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}


pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let text = format!("Game Master v1.2 error:\n\n{info}");
        let dir = Settings::app_dir().join("logs");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("crash.txt"), &text);
        unsafe {
            let a = wide(&text);
            let b = wide("Game Master 6");
            MessageBoxW(std::ptr::null_mut(), a.as_ptr(), b.as_ptr(), MB_ICONERROR | MB_OK);
        }
    }));

    unsafe {
        enable_dpi_awareness();
        CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);

        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--dump-ui") {
            let out_dir = args.get(pos + 1).map(PathBuf::from).unwrap_or_else(|| Settings::app_dir().join("ui_dump"));
            dump_ui_tabs(&out_dir);
            CoUninitialize();
            return;
        }

        let class = wide("GameMasterViceV2Class");
        let title = wide("Game Master 6 v1.2");
        let hinst = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: 0,
            lpfn_wnd_proc: Some(wnd_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: hinst,
            h_icon: std::ptr::null_mut(),
            h_cursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW as *const u16),
            hbr_background: std::ptr::null_mut(),
            lpsz_menu_name: std::ptr::null(),
            lpsz_class_name: class.as_ptr(),
            h_icon_sm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        let saved_s = Settings::load();
        let style = WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let req_w = if saved_s.win_w >= MIN_W { saved_s.win_w } else { DEFAULT_W };
        let req_h = if saved_s.win_h >= MIN_H { saved_s.win_h } else { DEFAULT_H };
        let mut wr = RECT::new(0, 0, req_w.min((sx - 60).max(MIN_W)), req_h.min((sy - 80).max(MIN_H)));
        AdjustWindowRectEx(&mut wr, style, 0, 0);
        let win_w = wr.width().min(sx.max(800));
        let win_h = wr.height().min(sy.max(600));
        let x = if saved_s.win_x >= 0 && saved_s.win_x + 120 < sx { saved_s.win_x } else { ((sx - win_w) / 2).max(16) };
        let y = if saved_s.win_y >= 0 && saved_s.win_y + 120 < sy { saved_s.win_y } else { ((sy - win_h) / 2).max(16) };

        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            style,
            x,
            y,
            win_w,
            win_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            let a = wide("Could not create the Game Master window.");
            let b = wide("Game Master 6");
            MessageBoxW(std::ptr::null_mut(), a.as_ptr(), b.as_ptr(), MB_ICONERROR | MB_OK);
            CoUninitialize();
            return;
        }

        let start_max = Settings::load().start_maximized;
        ShowWindow(hwnd, if start_max { SW_SHOWMAXIMIZED } else { SW_SHOW });
        UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        CoUninitialize();
    }
}


fn commit_pct(sys: &SystemInfo) -> u32 {
    if sys.total_commit_gb > 0.1 {
        (((sys.total_commit_gb - sys.avail_commit_gb) / sys.total_commit_gb) * 100.0).clamp(0.0, 100.0) as u32
    } else {
        0
    }
}


fn push_history(buf: &mut Vec<u32>, val: u32) {
    buf.push(val);
    if buf.len() > 40 {
        buf.remove(0);
    }
}


fn vk_to_bind_name(vk: u8) -> Option<String> {
    match vk {
        0x0D => Some("Enter".into()),
        0x08 => Some("Back".into()),
        0x09 => Some("Tab".into()),
        0x20 => Some("Space".into()),
        0x26 => Some("Up".into()),
        0x28 => Some("Down".into()),
        0x25 => Some("Left".into()),
        0x27 => Some("Right".into()),
        0x10 | 0xA0 | 0xA1 => Some("Shift".into()),
        0x11 | 0xA2 | 0xA3 => Some("Ctrl".into()),
        0x12 | 0xA4 | 0xA5 => Some("Alt".into()),
        b'A'..=b'Z' | b'0'..=b'9' => Some((vk as char).to_string()),
        _ => None,
    }
}


unsafe fn dump_ui_tabs(out_dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(out_dir);
    let settings = Settings::load();
    let sys = SystemInfo::collect(&settings.game_dir);
    let audit = AssetAudit::scan(&settings, &sys);
    let runners = discover_runners(&settings);
    let recent_logs = list_recent_logs(12);
    let log_lines = recent_logs.first().map(|e| read_log_tail(&e.path, 250)).unwrap_or_default();
    let (status_text, status_kind) = if audit.passed == audit.total {
        ("SYSTEM READY".into(), StatusKind::Ready)
    } else {
        (format!("CHECK {}/{}", audit.passed, audit.total), StatusKind::Warning)
    };
    let fonts = Fonts::load(&Settings::app_dir());
    let w = DEFAULT_W;
    let h = DEFAULT_H;
    let mem = CreateCompatibleDC(std::ptr::null_mut());
    let mut bmi = BITMAPINFO::default();
    bmi.bmi_header.bi_size = std::mem::size_of::<BITMAPINFOHEADER>() as DWORD;
    bmi.bmi_header.bi_width = w;
    bmi.bmi_header.bi_height = -h;
    bmi.bmi_header.bi_planes = 1;
    bmi.bmi_header.bi_bit_count = 32;
    bmi.bmi_header.bi_compression = 0;
    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let bmp = CreateDIBSection(mem, &bmi, 0, &mut bits, std::ptr::null_mut(), 0);
    if bmp.is_null() || bits.is_null() {
        fonts.destroy();
        DeleteDC(mem);
        return;
    }
    let old = SelectObject(mem, bmp);
    let logo = LogoSurface::load(mem, &Settings::app_dir().join("logo.bmp"));
    let portwatch = PortWatchSnapshot::default();
    let mods = saves::list_mods(8);
    let slow_shaders = Vec::new();
    let ram_history = vec![sys.mem_load_pct.min(100); 2];
    let commit_history = vec![commit_pct(&sys); 2];
    let recent_sessions = telemetry::load_recent_sessions(8);
    let runner_benches = Vec::new();
    let (ps, pc) = telemetry::total_playtime_summary();
    let playtime_label = format!("PLAYTIME {} ({} RUNS)", telemetry::format_playtime(ps), pc);
    let shader_census = ShaderCensus::default();

    for view in 0..=6 {
        let ctx = RenderContext {
            settings: &settings,
            sys: &sys,
            audit: &audit,
            session: None,
            view,
            hover: Target::None,
            status_text: &status_text,
            status_kind,
            data_label: "49.1 GB GAME DATA",
            file_count_label: "14,200 FILES",
            anim_tick: 0,
            toast: None,
            recent_logs: &recent_logs,
            selected_log: 0,
            log_lines: &log_lines,
            log_scroll: 0,
            fonts: &fonts,
            logo: logo.as_ref(),
            runners: &runners,
            portwatch: &portwatch,
            portwatch_syncing: false,
            portwatch_scroll: 0,
            runner_scroll: 0,
            tuning_scroll: 0,
            log_filter: 0,
            playtime_label: &playtime_label,
            mod_count: mods.len(),
            fps_history: &[],
            runner_benches: &runner_benches,
            mods: &mods,
            backup_count: saves::count_backups(),
            slow_shaders: &slow_shaders,
            ram_history: &ram_history,
            commit_history: &commit_history,
            recent_sessions: &recent_sessions,
            proc_stats: None,
            listening_keybind: None,
            shader_census: &shader_census,
            census_running: false,
            last_crash: None,
            dismissed_warning: None,
            adv_collapsed: [false, false, false, false, true],
            system_tip: &(false, String::new()),
            log_comparison: None,
            show_log_compare: false,
            free_ram_history: &ram_history,
            avail_commit_history: &commit_history,
            vram_headroom_history: &[90, 90],
            adv_filter: "ALL",
            auto_doctor: None,
            cache_hash_mismatch: false,
            beats_baseline: false,
        };
        let _ = paint(mem, w, h, &ctx);
        let px_bytes = (w as usize) * (h as usize) * 4;
        let pixels = std::slice::from_raw_parts(bits as *const u8, px_bytes);
        let file_size = 54 + px_bytes as u32;
        let mut out = Vec::with_capacity(file_size as usize);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&file_size.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&(-h).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(px_bytes as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        out.extend_from_slice(pixels);
        let _ = std::fs::write(out_dir.join(format!("tab{view}.bmp")), out);
    }

    if let Some(ref l) = logo {
        l.destroy();
    }
    fonts.destroy();
    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
}
