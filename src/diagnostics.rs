use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::runners::{cache_dir_for, default_victory_cache_dir, default_victory_data_dir, quick_cache_summary};
use crate::win32::{
    local_timestamp_filename, local_timestamp_human, poll_xinput_pads, read_tdr_delay_secs, wide, GetDiskFreeSpaceExW, GetSystemInfo, GetSystemMetrics, XInputPadInfo,
    GlobalMemoryStatusEx, IsProcessorFeaturePresent, MEMORYSTATUSEX,
    PF_AVX2_INSTRUCTIONS_AVAILABLE, SM_CXSCREEN, SM_CYSCREEN, SYSTEM_INFO,
};

#[derive(Clone, Debug, Default)]
pub struct SystemInfo {
    pub cpu_cores: u32,
    pub avx2: bool,
    pub total_ram_gb: f64,
    pub avail_ram_gb: f64,
    pub total_commit_gb: f64,
    pub avail_commit_gb: f64,
    pub mem_load_pct: u32,
    pub disk_free_gb: f64,
    pub tdr_delay_secs: Option<u32>,
    pub xinput_pads: u32,
    pub xinput_pad_details: [XInputPadInfo; 4],
    pub gpu_name: String,
    pub gpu_driver: String,
    pub vram_gb: f64,
    pub display_count: u32,
    pub primary_res: String,
    pub pagefile_pending_reboot: bool,
    pub vram_tax_mb: u32,
    pub vram_tax_note: String,
}

impl SystemInfo {
    pub fn collect(game_dir: &str) -> Self {
        let mut info = Self::default();
        unsafe {
            let mut si: SYSTEM_INFO = std::mem::zeroed();
            GetSystemInfo(&mut si);
            info.cpu_cores = si.dw_number_of_processors;
            info.avx2 = IsProcessorFeaturePresent(PF_AVX2_INSTRUCTIONS_AVAILABLE) != 0;
            info.tdr_delay_secs = read_tdr_delay_secs();
            info.xinput_pad_details = poll_xinput_pads();
            info.xinput_pads = info.xinput_pad_details.iter().filter(|p| p.connected).count() as u32;

            let mut mem = MEMORYSTATUSEX {
                dw_length: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };
            if GlobalMemoryStatusEx(&mut mem) != 0 {
                const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
                info.total_ram_gb = mem.ull_total_phys as f64 / GIB;
                info.avail_ram_gb = mem.ull_avail_phys as f64 / GIB;
                info.total_commit_gb = mem.ull_total_page_file as f64 / GIB;
                info.avail_commit_gb = mem.ull_avail_page_file as f64 / GIB;
                info.mem_load_pct = mem.dw_memory_load;
            }

            let probe_path = if Path::new(game_dir).is_dir() {
                game_dir.to_string()
            } else {
                Settings::app_dir().to_string_lossy().into_owned()
            };
            let w = wide(&probe_path);
            let mut free_avail = 0u64;
            let mut total = 0u64;
            let mut total_free = 0u64;
            if GetDiskFreeSpaceExW(w.as_ptr(), &mut free_avail, &mut total, &mut total_free) != 0 {
                info.disk_free_gb = free_avail as f64 / (1024.0 * 1024.0 * 1024.0);
            }
            const SM_CMONITORS: i32 = 80;
            info.display_count = GetSystemMetrics(SM_CMONITORS).max(1) as u32;
            let sw = GetSystemMetrics(SM_CXSCREEN).max(0);
            let sh = GetSystemMetrics(SM_CYSCREEN).max(0);
            if sw > 0 && sh > 0 {
                info.primary_res = format!("{sw}x{sh}");
            }
        }
        // Detect configured-on-disk D:\pagefile.sys that Windows has not mounted into active commit yet
        if let Ok(meta) = fs::metadata(r"D:\pagefile.sys") {
            let file_gb = meta.len() as f64 / (1024.0 * 1024.0 * 1024.0);
            if file_gb >= 12.0 && info.total_commit_gb > 0.0 && info.total_commit_gb < info.total_ram_gb + (file_gb * 0.6) {
                info.pagefile_pending_reboot = true;
            }
        }
        let vg = crate::vram_guard::scan_vram_consumers();
        info.vram_tax_mb = vg.est_vram_tax_mb;
        info.vram_tax_note = vg.summary_line().unwrap_or_default();
        info
    }

    /// Only flag genuinely critical commit pressure (< 4 GB free). The user
    /// manages the pagefile on D: themselves; do not nag at 33 GB total commit.
    pub fn commit_healthy(&self) -> bool {
        self.avail_commit_gb >= 4.0
    }

    pub fn tdr_safe(&self) -> bool {
        self.tdr_delay_secs.unwrap_or(2) >= 10
    }

    pub fn recommended_jobs(&self) -> u32 {
        ((self.cpu_cores * 2) / 3).clamp(4, 16)
    }
}

#[derive(Clone, Debug)]
pub struct AssetCheck {
    pub name: &'static str,
    pub role: &'static str,
    pub path: String,
    pub present: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct AssetAudit {
    pub items: Vec<AssetCheck>,
    pub warnings: Vec<String>,
    pub passed: usize,
    pub total: usize,
    pub shader_cache_ready: bool,
    pub shader_cache_label: String,
    pub exe_outside_game_dir: bool,
}

impl AssetAudit {
    pub fn scan(s: &Settings, sys: &SystemInfo) -> Self {
        let game = Path::new(&s.game_dir);
        let mut items = Vec::new();

        items.push(check_file("Game Executable", "Primary Title Binary", Path::new(&s.exe)));
        items.push(check_file("Compatibility Runner", "Local Runtime Host", &s.resolved_runner()));

        let dir_present = game.is_dir();
        items.push(AssetCheck {
            name: "Game Data Folder",
            role: "Root Asset Directory",
            path: s.game_dir.clone(),
            present: dir_present,
            detail: if dir_present { "DIRECTORY OK".into() } else { "NOT FOUND".into() },
        });

        items.push(check_file("Shader Archive", "Required for PREPARE", &game.join("shaders.rpf")));
        items.push(check_file("BVH Builder DLL", "Raytracing / Acceleration", &game.join("XboxBvhBuilder.dll")));
        items.push(check_file(
            "FidelityFX Upscaler",
            "Built-in Xbox Upscaler",
            &game.join("amd_fidelityfx_upscaler_xbox.dll"),
        ));

        let mut upscaler_detail = "NOT CONFIGURED".to_string();
        let mut upscaler_ok = false;
        if !s.upscaler_dir.trim().is_empty() {
            let up = Path::new(s.upscaler_dir.trim());
            if up.is_dir() {
                if let Some(dll_name) = find_host_upscaler_dll(up) {
                    upscaler_ok = true;
                    upscaler_detail = format!("{dll_name} · READY");
                } else {
                    upscaler_detail = "FOLDER HAS NO DLLS".into();
                }
            } else {
                upscaler_detail = "FOLDER MISSING".into();
            }
        }
        let rpf_names = [
            "common.rpf", "textures.rpf", "levels.rpf", "data.rpf",
            "audio.rpf", "script.rpf", "anim.rpf", "movies.rpf",
            "packs.rpf", "navmeshes.rpf", "appdata0.rpf", "MicrosoftGame.Config",
        ];
        let rpf_found = rpf_names.iter().filter(|n| game.join(n).is_file()).count();
        items.push(AssetCheck {
            name: "Core World Archives",
            role: "11 .rpf Packs + Config",
            path: s.game_dir.clone(),
            present: rpf_found == rpf_names.len(),
            detail: format!("{rpf_found}/{} PRESENT", rpf_names.len()),
        });

        items.push(AssetCheck {
            name: "Host Upscaler Folder",
            role: "Optional Series S Boost",
            path: if s.upscaler_dir.is_empty() { "(none)".into() } else { s.upscaler_dir.clone() },
            present: upscaler_ok,
            detail: upscaler_detail,
        });

        // Core checks are the first 6 items (upscaler folder is optional)
        let core_total = 7usize;
        let core_passed = items.iter().take(core_total).filter(|i| i.present).count();

        let mut warnings = Vec::new();
        if !sys.avx2 {
            warnings.push("CPU does not report AVX2 support (required by runner).".into());
        }
        if sys.pagefile_pending_reboot {
            warnings.push(
                "D:\\pagefile.sys is on disk but not active in Windows commit yet — do a full Windows Restart to unlock full commit.".into()
            );
        }
        if sys.vram_tax_mb >= 400 && !sys.vram_tax_note.is_empty() && (sys.vram_gb <= 0.5 || sys.vram_gb <= 13.5) {
            warnings.push(sys.vram_tax_note.clone());
        }
        if sys.display_count > 1 && sys.vram_gb > 0.5 && sys.vram_gb <= 13.5 {
            warnings.push(format!(
                "{} displays active on {:.0} GB VRAM — Win+P (Second screen only) frees ~1 GB VRAM for textures.",
                sys.display_count, sys.vram_gb
            ));
        }
        if !sys.commit_healthy() && sys.total_commit_gb > 0.0 {
            warnings.push(format!(
                "Only {:.1} GB Windows commit free — close apps or raise your D: paging file to avoid Error 1455.",
                sys.avail_commit_gb
            ));
        }
        if s.upscaler_enabled && !s.is_series_s() {
            warnings.push("Host upscaler is enabled, but Series S mode is off (host upscaling requires Series S).".into());
        }
        if s.upscaler_enabled && s.upscaler_dir.trim().is_empty() {
            warnings.push("Host upscaler is enabled without an upscaler folder selected.".into());
        }

        let kind = s.active_runner_kind();
        let runner_path = s.resolved_runner();
        let cdir = cache_dir_for(&runner_path, kind, s.isolated_cache);
        let (mut cache_ready, mut cache_label) = quick_cache_summary(&cdir);
        if !cache_ready && s.isolated_cache {
            let (def_ok, def_lbl) = quick_cache_summary(&default_victory_cache_dir());
            if def_ok {
                cache_ready = true;
                cache_label = format!("{def_lbl} (global)");
            }
        }
        let mut exe_outside_game_dir = false;
        if let Some(exe_parent) = Path::new(&s.exe).parent() {
            let ep = exe_parent.to_string_lossy().trim_end_matches(&['\\', '/'][..]).to_ascii_lowercase();
            let gd = s.game_dir.trim_end_matches(&['\\', '/'][..]).to_ascii_lowercase();
            if !ep.is_empty() && !gd.is_empty() && ep != gd && Path::new(&s.exe).is_file() && Path::new(&s.game_dir).is_dir() {
                exe_outside_game_dir = true;
                warnings.push("EXE IS OUTSIDE GAME_DIR — Click 'LINK EXE TO GAME DIR' to fix 0x80000003 / TitleID 00000000 crash!".into());
            }
        }
        if !sys.tdr_safe() {
            warnings.push(format!(
                "Windows GPU TdrDelay is {}s (default 2s) — heavy world frames can reset your GPU driver. Click FIX TDR (15s) in Settings.",
                sys.tdr_delay_secs.unwrap_or(2)
            ));
        }
        if !cache_ready && kind.supports_prepare() {
            warnings.push("Shader/pipeline cache not detected yet — run PREPARE CACHE before first play to reduce stutter.".into());
        }
        Self {
            items,
            warnings,
            passed: core_passed,
            total: core_total,
            shader_cache_ready: cache_ready,
            shader_cache_label: cache_label,
            exe_outside_game_dir,
        }
    }
}

fn check_file(name: &'static str, role: &'static str, path: &Path) -> AssetCheck {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => AssetCheck {
            name,
            role,
            path: path.to_string_lossy().into_owned(),
            present: true,
            detail: format_bytes(meta.len()),
        },
        _ => AssetCheck {
            name,
            role,
            path: path.to_string_lossy().into_owned(),
            present: false,
            detail: "MISSING".into(),
        },
    }
}

/// Picks the primary host FidelityFX DLL filename in `dir` (preferring `amd_fidelityfx_upscaler_dx12.dll`).
fn find_host_upscaler_dll(dir: &Path) -> Option<String> {
    let mut first_dll: Option<String> = None;
    let rd = fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let is_dll = p
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("dll"))
            .unwrap_or(false);
        if !is_dll {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
            if name.eq_ignore_ascii_case("amd_fidelityfx_upscaler_dx12.dll")
                || name.eq_ignore_ascii_case("amd_fidelityfx_dx12.dll")
            {
                return Some(name.to_string());
            }
            if first_dll.is_none() {
                first_dll = Some(name.to_string());
            }
        }
    }
    first_dll
}

/// Returns true if a log line is a known NVIDIA wave32 runner limitation rather than a user-config error.
pub fn is_known_wave32_note(line: &str) -> bool {
    let lo = line.to_ascii_lowercase();
    lo.contains("no wave32 lowering for this wave64 program")
        || lo.contains("host wave lanes min=32 max=32")
        || lo.contains("slot=0x218")
        || lo.contains("slot=0x1c8")
}

/// Counts known NVIDIA wave32 runner limitation lines in a log.
pub fn count_wave32_notes(lines: &[String]) -> usize {
    lines.iter().filter(|l| is_known_wave32_note(l)).count()
}

fn friendly_proc_name(exe: &str) -> Option<String> {
    let lo = exe.trim().to_ascii_lowercase();
    let base = lo.strip_suffix(".exe").unwrap_or(&lo);
    const IGNORE: &[&str] = &[
        "system", "registry", "smss", "csrss", "wininit", "services", "lsass", "svchost",
        "dwm", "memory compression", "fontdrvhost", "winlogon", "explorer", "searchhost",
        "runtimebroker", "shellexperiencehost", "startmenuexperiencehost", "textinputhost",
        "taskhostw", "ctfmon", "conhost", "audiodg", "spoolsv", "msmpeng", "nissrv",
        "securityhealthservice", "wmiprvse", "tasklist", "gamemaster", "game master",
        "victory", "victory-diag", "gamerunner", "sihost", "dllhost",
        "dashost", "wlanext", "spoolsv", "nvcontainer", "nvdisplay.container",
    ];
    if base.is_empty() || IGNORE.contains(&base) {
        return None;
    }
    let nice = match base {
        "chrome" => "Chrome",
        "msedge" => "Edge",
        "firefox" => "Firefox",
        "brave" => "Brave",
        "opera" => "Opera",
        "discord" => "Discord",
        "code" => "VS Code",
        "obs64" | "obs32" => "OBS",
        "steam" | "steamwebhelper" => "Steam",
        "spotify" => "Spotify",
        _ => return Some(base.to_string()),
    };
    Some(nice.to_string())
}

/// Probes top memory-consuming user processes (excluding system processes and Game Master).
pub fn probe_top_ram_apps() -> Vec<(String, f64)> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let Ok(out) = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut totals: Vec<(String, u64)> = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split("\",\"").collect();
        if cols.len() < 5 {
            continue;
        }
        let exe = cols[0].trim_matches('"');
        let Some(name) = friendly_proc_name(exe) else {
            continue;
        };
        let mem_raw: String = cols[4].chars().filter(|c| c.is_ascii_digit()).collect();
        let kb: u64 = mem_raw.parse().unwrap_or(0);
        if kb == 0 {
            continue;
        }
        if let Some(entry) = totals.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&name)) {
            entry.1 += kb;
        } else {
            totals.push((name, kb));
        }
    }
    totals.sort_by(|a, b| b.1.cmp(&a.1));
    totals
        .into_iter()
        .map(|(n, kb)| (n, kb as f64 / (1024.0 * 1024.0)))
        .filter(|(_, gb)| *gb >= 0.25)
        .take(2)
        .collect()
}

/// Builds a prioritized, process-aware System Tip banner from live hardware & process state.
pub fn compute_system_tip(
    s: &Settings,
    sys: &SystemInfo,
    audit: &AssetAudit,
    top_apps: &[(String, f64)],
    on_battery: bool,
) -> (bool, String) {
    if (sys.avail_ram_gb > 0.1 && sys.avail_ram_gb < 18.0)
        || (sys.avail_commit_gb > 0.1 && sys.avail_commit_gb < 30.0)
    {
        let msg = if top_apps.len() >= 2 {
            format!(
                "{} ({:.1} GB) and {} ({:.1} GB) are using RAM — closing them helps prevent prefetch stalls.",
                top_apps[0].0, top_apps[0].1, top_apps[1].0, top_apps[1].1
            )
        } else if let Some((name, gb)) = top_apps.first() {
            format!(
                "{name} ({gb:.1} GB) is using RAM ({:.1} GB free) — closing it helps prevent prefetch stalls.",
                sys.avail_ram_gb
            )
        } else {
            format!(
                "Low free memory ({:.1} GB RAM, {:.1} GB commit free) — close unused apps to prevent prefetch stalls.",
                sys.avail_ram_gb, sys.avail_commit_gb
            )
        };
        return (true, msg);
    }

    if sys.display_count > 1 && sys.vram_gb > 0.5 && sys.vram_gb <= 13.5 {
        return (
            true,
            format!(
                "{} displays active on {:.0} GB VRAM — Win+P -> Single display mode reclaims ~1 GB DXGI VRAM.",
                sys.display_count, sys.vram_gb
            ),
        );
    }

    if on_battery {
        return (
            true,
            "Running on battery power — plug in AC power for full GPU clocks and stable frame times.".into(),
        );
    }

    let lv = s.log_level.trim().to_ascii_lowercase();
    if lv == "info" || lv == "trace" || lv == "diagnostics" {
        return (
            true,
            format!(
                "Verbose logging ({lv}) adds disk/CPU overhead — switch back to warn after benchmarking."
            ),
        );
    }

    if !audit.shader_cache_ready && s.active_runner_kind().supports_prepare() {
        return (
            true,
            "Shader cache not prepared yet — click PREPARE CACHE before launching.".into(),
        );
    }

    (
        false,
        format!(
            "System looks clean — {:.1} GB RAM and {:.0} GB commit free. Ready to launch.",
            sys.avail_ram_gb, sys.avail_commit_gb
        ),
    )
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

pub fn folder_stats(path: &Path) -> (u64, u64) {
    let mut total_bytes = 0u64;
    let mut file_count = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    let (b, c) = folder_stats(&entry.path());
                    total_bytes += b;
                    file_count += c;
                } else {
                    total_bytes += meta.len();
                    file_count += 1;
                }
            }
        }
    }
    (total_bytes, file_count)
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub name: String,
    pub path: PathBuf,
    pub size_label: String,
}

pub fn logs_dir() -> PathBuf {
    let dir = Settings::app_dir().join("logs");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn captures_dir() -> PathBuf {
    let dir = Settings::app_dir().join("captures");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn reports_dir() -> PathBuf {
    let dir = Settings::app_dir().join("reports");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn list_recent_logs(limit: usize) -> Vec<LogEntry> {
    let dir = logs_dir();
    let mut entries: Vec<(std::time::SystemTime, LogEntry)> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("log") || s.eq_ignore_ascii_case("txt")).unwrap_or(false) {
                if let Ok(meta) = e.metadata() {
                    let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    entries.push((
                        modified,
                        LogEntry {
                            name,
                            path: p,
                            size_label: format_bytes(meta.len()),
                        },
                    ));
                }
            }
        }
    }
    // Also surface VIctory's crashes.log files (which include 'title said:' debug lines)
    if let Some(vdata) = default_victory_data_dir() {
        if let Ok(rd) = fs::read_dir(&vdata) {
            for sub in rd.flatten() {
                let cp = sub.path().join("crashes.log");
                if cp.is_file() {
                    if let Ok(meta) = fs::metadata(&cp) {
                        let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        let tid = sub.file_name().to_string_lossy().into_owned();
                        entries.push((
                            modified,
                            LogEntry {
                                name: format!("[CRASHES] {tid}/crashes.log"),
                                path: cp,
                                size_label: format_bytes(meta.len()),
                            },
                        ));
                    }
                }
            }
        }
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().take(limit).map(|(_, e)| e).collect()
}

pub fn read_log_tail(path: &Path, max_lines: usize) -> Vec<String> {
    let Ok(mut f) = File::open(path) else {
        return Vec::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let read_from = len.saturating_sub(64 * 1024);
    let _ = f.seek(SeekFrom::Start(read_from));
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<String> = text.lines().map(strip_ansi).collect();
    if read_from > 0 && !lines.is_empty() {
        lines.remove(0); // drop potentially truncated first line
    }
    if lines.len() > max_lines {
        lines.split_off(lines.len() - max_lines)
    } else {
        lines
    }
}

pub fn export_report(s: &Settings, sys: &SystemInfo, audit: &AssetAudit, data_label: &str) -> Result<PathBuf, String> {
    let dir = reports_dir();
    let path = dir.join(format!("system-report-{}.txt", local_timestamp_filename()));
    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str("  GAME MASTER 6 v1.0  |  LOCAL SYSTEM REPORT\n");
    out.push_str("============================================================\n");
    out.push_str(&format!("Generated       : {}\n", local_timestamp_human()));
    out.push_str(&format!("App Directory   : {}\n", Settings::app_dir().display()));
    out.push_str(&format!("Detected Build  : {}\n", s.version_label()));
    out.push_str(&format!("Game Data Size  : {}\n\n", data_label));

    out.push_str("[HARDWARE & MEMORY]\n");
    out.push_str(&format!("Graphics Card     : {} ({:.0} GB VRAM · Driver {})\n", if sys.gpu_name.is_empty() { "Detecting..." } else { &sys.gpu_name }, sys.vram_gb, sys.gpu_driver));
    out.push_str(&format!("Logical CPU Cores : {}\n", sys.cpu_cores));
    out.push_str(&format!("AVX2 Support      : {}\n", if sys.avx2 { "YES" } else { "NO (REQUIRED)" }));
    out.push_str(&format!(
        "Physical RAM      : {:.1} GB available / {:.1} GB total ({}% load)\n",
        sys.avail_ram_gb, sys.total_ram_gb, sys.mem_load_pct
    ));
    out.push_str(&format!(
        "Windows Commit    : {:.1} GB available / {:.1} GB total\n",
        sys.avail_commit_gb, sys.total_commit_gb
    ));
    out.push_str(&format!("Drive Free Space  : {:.1} GB\n", sys.disk_free_gb));
    out.push_str(&format!("GPU TdrDelay      : {}s ({})\n\n", sys.tdr_delay_secs.unwrap_or(2), if sys.tdr_safe() { "SAFE" } else { "DEFAULT 2s - RISK" }));

    out.push_str("[CONFIGURATION]\n");
    out.push_str(&format!("Active Runner     : {} ({})\n", s.resolved_runner().display(), s.active_runner_kind().badge()));
    out.push_str(&format!("Shader Cache      : {}\n", audit.shader_cache_label));
    out.push_str(&format!("Console Model     : {}\n", s.console_model));
    out.push_str(&format!("Log Level         : {} (VICTORY_LOG={})\n", s.log_level, s.effective_victory_log()));
    out.push_str(&format!("Host Upscaler     : {} ({})\n", if s.upscaler_enabled { "ENABLED" } else { "DISABLED" }, s.upscaler_dir));
    out.push_str(&format!("GPU Timing        : {}\n", if s.gpu_timing { "ON" } else { "OFF" }));
    out.push_str(&format!("Frame Capture     : {}\n", if s.capture { "ON" } else { "OFF" }));
    out.push_str(&format!("Shader Loop Limit : {}\n", if s.shader_loop_limit.is_empty() { "AUTO" } else { &s.shader_loop_limit }));
    out.push_str(&format!("v11 Perf Switches : async_submit={} queue2={} indirect_window={} texture_no_wait={} merge_passes={}\n", s.async_submit, s.queue2, s.indirect_window, s.texture_no_wait, s.merge_passes));
    out.push_str(&format!("Title Args        : {}\n\n", if s.title_args.is_empty() { "(none)" } else { &s.title_args }));

    out.push_str(&format!("[ASSET AUDIT: {}/{} CORE CHECKS PASSED]\n", audit.passed, audit.total));
    for item in &audit.items {
        let tag = if item.present { "OK     " } else { "MISSING" };
        out.push_str(&format!("  [{}] {:<22} {:<16} {}\n", tag, item.name, item.detail, item.path));
    }
    if !audit.warnings.is_empty() {
        out.push_str("\n[WARNINGS]\n");
        for w in &audit.warnings {
            out.push_str(&format!("  - {w}\n"));
        }
    }

    fs::write(&path, out).map_err(|e| format!("Could not write system report: {e}"))?;
    Ok(path)
}

/// Strips ANSI/VT100 escape sequences (e.g. colored runner logs) for clean GDI display.
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for nc in chars.by_ref() {
                    if ('@'..='~').contains(&nc) {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    out
}

/// Scans log lines for VICTORY_GPU_TIMING / frame-time metrics to show a quick summary pill.
pub fn summarize_gpu_timings(lines: &[String]) -> Option<String> {
    let mut hits = 0usize;
    let mut last_metric: Option<String> = None;
    for line in lines {
        let lo = line.to_ascii_lowercase();
        if lo.contains("gpu_timing") || lo.contains("frame time") || (lo.contains("ms") && (lo.contains("pass") || lo.contains("gpu") || lo.contains("fps"))) {
            hits += 1;
            last_metric = Some(line.trim().to_string());
        }
    }
    last_metric.map(|m| {
        let short: String = m.chars().take(72).collect();
        format!("GPU TIMING ({hits} samples) · {short}")
    })
}

/// Writes an inspectable Windows .reg file setting TdrDelay & TdrDdiDelay to 15 seconds.
pub fn export_tdr_reg() -> Result<PathBuf, String> {
    let path = reports_dir().join("fix-tdr-15s.reg");
    let content = "Windows Registry Editor Version 5.00\r\n\r\n\
[HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers]\r\n\
\"TdrDelay\"=dword:0000000f\r\n\
\"TdrDdiDelay\"=dword:0000000f\r\n";
    fs::write(&path, content).map_err(|e| format!("Could not write TDR registry helper: {e}"))?;
    Ok(path)
}

#[derive(Clone, Debug)]
pub struct SlowShader {
    pub hash: String,
    pub ms: Option<f64>,
}

/// Extracts unique 16-hex shader hashes from GPU timing / DRED log lines, sorted slowest-first.
pub fn extract_slow_shaders(lines: &[String], max_items: usize) -> Vec<SlowShader> {
    let mut found: Vec<SlowShader> = Vec::new();
    for line in lines {
        let ms = parse_ms_in_line(line);
        for tok in line.split(|c: char| !c.is_ascii_hexdigit()) {
            if tok.len() == 16 && tok.bytes().any(|b| b.is_ascii_alphabetic()) {
                let h = tok.to_ascii_lowercase();
                if let Some(existing) = found.iter_mut().find(|s| s.hash == h) {
                    if let (Some(new_ms), Some(old_ms)) = (ms, existing.ms) {
                        if new_ms > old_ms {
                            existing.ms = Some(new_ms);
                        }
                    } else if existing.ms.is_none() {
                        existing.ms = ms;
                    }
                } else {
                    found.push(SlowShader { hash: h, ms });
                }
            }
        }
    }
    found.sort_by(|a, b| {
        b.ms.unwrap_or(-1.0)
            .partial_cmp(&a.ms.unwrap_or(-1.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    found.truncate(max_items);
    found
}

fn parse_ms_in_line(line: &str) -> Option<f64> {
    for tok in line.split_whitespace() {
        let t = tok.trim_matches(|c: char| matches!(c, ',' | ';' | '(' | ')' | '[' | ']'));
        if let Some(num) = t.strip_suffix("ms") {
            if let Ok(v) = num.parse::<f64>() {
                if v > 0.0 && v < 60000.0 {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Finds the most recently modified `%LOCALAPPDATA%\victory\data\*\crashes.log`.
pub fn latest_victory_crashes_log() -> Option<(String, PathBuf)> {
    let vdata = default_victory_data_dir()?;
    let mut best: Option<(std::time::SystemTime, String, PathBuf)> = None;
    if let Ok(rd) = fs::read_dir(&vdata) {
        for sub in rd.flatten() {
            let cp = sub.path().join("crashes.log");
            if cp.is_file() {
                if let Ok(meta) = fs::metadata(&cp) {
                    let m = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let tid = sub.file_name().to_string_lossy().into_owned();
                    if best.as_ref().map_or(true, |(bm, _, _)| m > *bm) {
                        best = Some((m, tid, cp));
                    }
                }
            }
        }
    }
    best.map(|(_, tid, p)| (tid, p))
}

/// Exports a single ready-to-paste Markdown AI Debug Bundle containing hardware, settings,
/// audit, recent session benchmarks, slow shaders, active log tail, and crashes.log tail.
pub fn export_ai_debug_bundle(
    s: &Settings,
    sys: &SystemInfo,
    audit: &AssetAudit,
    data_label: &str,
    log_lines: &[String],
    slow_shaders: &[SlowShader],
    sessions: &[crate::telemetry::SessionRecord],
) -> Result<(PathBuf, String), String> {
    let dir = reports_dir();
    let path = dir.join(format!("ai-debug-bundle-{}.md", local_timestamp_filename()));
    let mut out = String::new();
    out.push_str("# Game Master 6 v1.0 — AI Debug Bundle\n\n");
    out.push_str(&format!("- **Generated:** {}\n", local_timestamp_human()));
    out.push_str(&format!("- **Build:** {} ({})\n", s.version_label(), data_label));
    out.push_str(&format!("- **Runner:** `{}` ({})\n", s.resolved_runner().display(), s.active_runner_kind().badge()));
    out.push_str(&format!("- **Shader Cache:** {}\n", audit.shader_cache_label));
    out.push_str(&format!("- **GPU:** {} (Driver {})\n", if sys.gpu_name.is_empty() { "Detecting..." } else { &sys.gpu_name }, sys.gpu_driver));
    out.push_str(&format!("- **Hardware:** {} cores | AVX2={} | RAM {:.1}/{:.1} GB free | Commit {:.1}/{:.1} GB free | Drive {:.1} GB free | TDR {}s | XInput Pads: {}\n\n",
        sys.cpu_cores, sys.avx2, sys.avail_ram_gb, sys.total_ram_gb, sys.avail_commit_gb, sys.total_commit_gb, sys.disk_free_gb, sys.tdr_delay_secs.unwrap_or(2), sys.xinput_pads));

    out.push_str("## Active Settings\n");
    out.push_str(&format!("- console=`{}` | log=`{}` | dGPU=`{}` | zero_overhead=`{}` | high_priority=`{}` | isolated_cache=`{}` | dred=`{}` | gpu_timing=`{}` | warp=`{}` | shader_limit=`{}` | skip_pipelines=`{}` | vram_buf=`{}` | rehash=`{}` | join_copies=`{}` | timer_1ms=`{}`\n\n",
        s.console_model, s.log_level, s.prefer_dgpu, s.zero_overhead, s.high_priority, s.isolated_cache, s.dred, s.gpu_timing, s.warp,
        if s.shader_loop_limit.is_empty() { "AUTO" } else { &s.shader_loop_limit },
        if s.skip_pipelines.is_empty() { "none" } else { &s.skip_pipelines },
        s.card_buffers_label(), s.card_rehash_runs, s.join_copies, s.timer_1ms));
    out.push_str(&format!("- v11 perf: async_submit=`{}` | queue2=`{}` | indirect_window=`{}` | texture_no_wait=`{}` | merge_passes=`{}`\n\n",
        s.async_submit, s.queue2, s.indirect_window, s.texture_no_wait, s.merge_passes));

    out.push_str(&format!("## Preflight Audit ({}/{})\n", audit.passed, audit.total));
    for item in &audit.items {
        out.push_str(&format!("- [{}] **{}**: {} (`{}`)\n", if item.present { "OK" } else { "MISSING" }, item.name, item.detail, item.path));
    }
    if !audit.warnings.is_empty() {
        out.push_str("\n### Warnings\n");
        for w in &audit.warnings {
            out.push_str(&format!("- {w}\n"));
        }
    }

    if !sessions.is_empty() {
        out.push_str("\n## Recent Session Benchmarks\n");
        out.push_str("| Time | Runner | Mode | Kind | Secs | Exit | Peak FPS | Top Shader |\n|---|---|---|---|---:|---|---|---|\n");
        for r in sessions.iter().take(6) {
            out.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} | {} |\n",
                r.timestamp, r.runner, r.console, r.kind, r.duration_secs, r.exit_code, r.peak_fps, r.top_shader));
        }
    }

    if !slow_shaders.is_empty() {
        out.push_str("\n## Slow Shaders Detected\n");
        for sh in slow_shaders {
            let ms = sh.ms.map(|v| format!("{v:.1} ms")).unwrap_or_else(|| "n/a".into());
            out.push_str(&format!("- `{}` ({ms})\n", sh.hash));
        }
    }

    if !log_lines.is_empty() {
        out.push_str("\n## Active Log Tail (last 40 lines)\n```text\n");
        let start = log_lines.len().saturating_sub(40);
        for l in &log_lines[start..] {
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("```\n");
    }

    if let Some((tid, cpath)) = latest_victory_crashes_log() {
        let clines = read_log_tail(&cpath, 25);
        if !clines.is_empty() {
            out.push_str(&format!("\n## VIctory crashes.log (TitleID {tid}, last 25 lines)\n```text\n"));
            for l in clines {
                out.push_str(&l);
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }

    fs::write(&path, &out).map_err(|e| format!("Could not write AI debug bundle: {e}"))?;
    Ok((path, out))
}

/// Deletes older session logs in `logs\`, keeping the newest `keep` files.
pub fn clean_old_logs(keep: usize) -> usize {
    let dir = logs_dir();
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() {
                if let Ok(meta) = e.metadata() {
                    let m = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    files.push((m, p));
                }
            }
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let mut removed = 0usize;
    for (_, p) in files.into_iter().skip(keep) {
        if fs::remove_file(p).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[derive(Clone, Debug, Default)]
pub struct ShaderCensus {
    pub translated: u32,
    pub total: u32,
    pub refused: u32,
    pub summary: String,
}

pub fn parse_shader_census(lines: &[String]) -> ShaderCensus {
    let mut translated = 0u32;
    let mut total = 0u32;
    let mut refused = 0u32;
    let mut reasons: Vec<String> = Vec::new();
    for raw in lines {
        let l = raw.trim();
        let lo = l.to_ascii_lowercase();
        if (lo.contains("translated") || lo.contains("shaders")) && lo.contains('/') {
            for tok in l.split_whitespace() {
                if let Some((a, b)) = tok.trim_matches(|c: char| !c.is_ascii_digit() && c != '/').split_once('/') {
                    if let (Ok(ok_n), Ok(tot_n)) = (a.parse::<u32>(), b.parse::<u32>()) {
                        if tot_n >= 100 && ok_n <= tot_n {
                            translated = ok_n;
                            total = tot_n;
                            refused = tot_n.saturating_sub(ok_n);
                        }
                    }
                }
            }
        }
        if (lo.contains("refused") || lo.contains("unsupported") || lo.contains("failed")) && reasons.len() < 3 {
            let short: String = l.chars().take(64).collect();
            if !reasons.contains(&short) {
                reasons.push(short);
            }
        }
    }
    let summary = if total > 0 {
        if refused == 0 {
            format!("{translated}/{total} shaders translated cleanly (100% PASS)")
        } else if reasons.is_empty() {
            format!("{translated}/{total} translated · {refused} refused")
        } else {
            format!("{translated}/{total} translated · {refused} refused ({})", reasons.join("; "))
        }
    } else {
        "CGBJ translator active. Click RUN CENSUS to scan the local shader archive.".into()
    };
    ShaderCensus { translated, total, refused, summary }
}
