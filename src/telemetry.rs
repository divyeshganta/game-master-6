use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::diagnostics::{extract_slow_shaders, logs_dir, read_log_tail, reports_dir, strip_ansi, SlowShader, SystemInfo};
use crate::win32::local_timestamp_human;

#[derive(Clone, Debug)]
pub struct SessionRecord {
    pub timestamp: String,
    pub runner: String,
    pub adapter: String,
    pub console: String,
    pub kind: String,
    pub duration_secs: u64,
    pub exit_code: String,
    pub peak_fps: String,
    pub top_shader: String,
}

pub fn sessions_csv_path() -> PathBuf {
    reports_dir().join("sessions.csv")
}

pub fn append_session(rec: &SessionRecord) {
    let path = sessions_csv_path();
    let needs_header = !path.is_file() || fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        if needs_header {
            let _ = writeln!(
                f,
                "timestamp,runner,adapter,console,kind,duration_secs,exit_code,peak_fps,top_shader"
            );
        }
        let clean = |s: &str| s.replace(',', ";").replace('\n', " ").replace('\r', "");
        let _ = writeln!(
            f,
            "{},{},{},{},{},{},{},{},{}",
            clean(&rec.timestamp),
            clean(&rec.runner),
            clean(&rec.adapter),
            clean(&rec.console),
            clean(&rec.kind),
            rec.duration_secs,
            clean(&rec.exit_code),
            clean(&rec.peak_fps),
            clean(&rec.top_shader),
        );
    }
}

pub fn load_recent_sessions(limit: usize) -> Vec<SessionRecord> {
    let Ok(text) = fs::read_to_string(sessions_csv_path()) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() >= 9 {
            rows.push(SessionRecord {
                timestamp: cols[0].to_string(),
                runner: cols[1].to_string(),
                adapter: cols[2].to_string(),
                console: cols[3].to_string(),
                kind: cols[4].to_string(),
                duration_secs: cols[5].parse().unwrap_or(0),
                exit_code: cols[6].to_string(),
                peak_fps: cols[7].to_string(),
                top_shader: cols[8].to_string(),
            });
        }
    }
    rows.reverse();
    rows.truncate(limit);
    rows
}

/// Extracts average 3D-world FPS (lines with `draws/frame` >= 500, matching victory capture.rs:123),
/// so menu/loading 30 FPS never masks real in-world gameplay FPS.
pub fn extract_world_avg_fps(lines: &[String]) -> Option<f64> {
    let mut sum = 0.0f64;
    let mut cnt = 0usize;
    for line in lines {
        if !line.contains(" fps,") || !line.contains(" draws/frame") {
            continue;
        }
        let draws = line
            .find(" draws/frame")
            .and_then(|di| line[..di].split_whitespace().last())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if draws < 500 {
            continue;
        }
        if let Some(fi) = line.find(" fps,") {
            if let Some(v) = line[..fi].split_whitespace().last().and_then(|s| s.parse::<f64>().ok()) {
                if v > 0.05 && v < 500.0 {
                    sum += v;
                    cnt += 1;
                }
            }
        }
    }
    if cnt > 0 { Some(sum / cnt as f64) } else { None }
}

/// Classifies the primary runtime bottleneck directly from victory log signals.
pub fn classify_bottleneck(lines: &[String]) -> &'static str {
    let mut vram_hit = false;
    let mut commit_hit = false;
    let mut upload_stall = false;
    for line in lines {
        if line.contains("the card has no room for more") {
            vram_hit = true;
        }
        if line.contains("host memory pressure") {
            commit_hit = true;
        }
        if line.contains("waiting for title memory to hold still") {
            upload_stall = true;
        }
    }
    if vram_hit {
        "VRAM BOUND"
    } else if commit_hit {
        "COMMIT BOUND"
    } else if upload_stall {
        "UPLOAD STALL"
    } else {
        "BALANCED"
    }
}

/// Unified FPS sample parser (returns FPS * 10, e.g. 29.8 fps -> 298) across all telemetry views.
pub fn parse_fps_series(lines: &[String]) -> Vec<u32> {
    let mut series = Vec::new();
    for line in lines {
        let lo = line.to_ascii_lowercase();
        if !lo.contains("fps") {
            continue;
        }
        let toks: Vec<&str> = lo.split_whitespace().collect();
        for (i, tok) in toks.iter().enumerate() {
            let candidate = if let Some(num) = tok.strip_suffix("fps") {
                Some(num)
            } else if (*tok == "fps" || *tok == "fps,") && i > 0 {
                Some(toks[i - 1])
            } else {
                None
            };
            if let Some(raw) = candidate {
                if let Ok(v) = raw.trim_matches(|c: char| !c.is_ascii_digit() && c != '.').parse::<f64>() {
                    if v > 0.1 && v < 500.0 {
                        series.push((v * 10.0).round() as u32);
                        break;
                    }
                }
            }
        }
    }
    if series.len() > 60 {
        series.drain(..series.len() - 60);
    }
    series
}

pub fn extract_peak_fps(lines: &[String]) -> Option<f64> {
    parse_fps_series(lines)
        .into_iter()
        .max()
        .map(|v| v as f64 / 10.0)
}

pub fn build_record(
    runner_name: &str,
    adapter: &str,
    console: &str,
    kind: &str,
    duration_secs: u64,
    exit_code: Option<i32>,
    lines: &[String],
    slow_shaders: &[SlowShader],
) -> SessionRecord {
    let code_str = match exit_code {
        Some(0) => "0 (OK)".into(),
        Some(c) => format!("0x{:08X}", c as u32),
        None => "STOPPED".into(),
    };
    let bn = classify_bottleneck(lines);
    let peak_fps = if let Some(w) = extract_world_avg_fps(lines) {
        format!("{w:.1} fps (world · {bn})")
    } else {
        extract_peak_fps(lines)
            .map(|f| format!("{f:.1} fps"))
            .unwrap_or_else(|| "-".into())
    };
    let top_shader = slow_shaders
        .first()
        .map(|s| {
            let short: String = s.hash.chars().take(8).collect();
            if let Some(ms) = s.ms {
                format!("{short} ({ms:.0}ms)")
            } else {
                short
            }
        })
        .unwrap_or_else(|| "-".into());

    SessionRecord {
        timestamp: local_timestamp_human(),
        runner: runner_name.to_string(),
        adapter: adapter.to_string(),
        console: console.to_string(),
        kind: kind.to_string(),
        duration_secs,
        exit_code: code_str,
        peak_fps,
        top_shader,
    }
}

pub fn best_session(sessions: &[SessionRecord]) -> Option<SessionRecord> {
    sessions
        .iter()
        .max_by_key(|s| {
            let fps_score = s
                .peak_fps
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
                .map(|v| (v * 100.0) as u64)
                .unwrap_or(0);
            (fps_score, s.duration_secs)
        })
        .cloned()
}

/// Sums total recorded playtime across all GAME RUNNER sessions in `sessions.csv`.
pub fn total_playtime_summary() -> (u64, usize) {
    let Ok(text) = fs::read_to_string(sessions_csv_path()) else {
        return (0, 0);
    };
    let mut total_secs = 0u64;
    let mut sessions = 0usize;
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() >= 6 && cols[4].eq_ignore_ascii_case("GAME RUNNER") {
            total_secs += cols[5].parse::<u64>().unwrap_or(0);
            sessions += 1;
        }
    }
    (total_secs, sessions)
}

pub fn format_playtime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {}s", secs % 60)
    } else {
        format!("{secs}s")
    }
}

#[derive(Clone, Debug)]
pub struct RunnerBenchSummary {
    pub runner: String,
    pub runs: usize,
    pub best_fps: f64,
    pub avg_fps: f64,
    pub max_secs: u64,
}

/// Rebuilds and merges `sessions.csv` from all `Game-*.log` files in `logs/` so manual closes
/// and non-zero exits are never missed across Leaderboard, A/B Scorecard, and Playtime.
pub fn sync_sessions_from_logs() {
    let mut existing: Vec<SessionRecord> = Vec::new();
    if let Ok(text) = fs::read_to_string(sessions_csv_path()) {
        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() >= 9 {
                existing.push(SessionRecord {
                    timestamp: cols[0].to_string(),
                    runner: cols[1].to_string(),
                    adapter: cols[2].to_string(),
                    console: cols[3].to_string(),
                    kind: cols[4].to_string(),
                    duration_secs: cols[5].parse().unwrap_or(0),
                    exit_code: cols[6].to_string(),
                    peak_fps: cols[7].to_string(),
                    top_shader: cols[8].to_string(),
                });
            }
        }
    }

    let mut log_files: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = fs::read_dir(logs_dir()) {
        for p in rd.flatten().map(|e| e.path()) {
            if p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("Game-") && n.ends_with(".log"))
                    .unwrap_or(false)
            {
                log_files.push(p);
            }
        }
    }
    if let Some(local_app) = std::env::var_os("LOCALAPPDATA") {
        let vlogs = PathBuf::from(local_app).join("victory").join("logs");
        if let Ok(rd) = fs::read_dir(&vlogs) {
            for p in rd.flatten().map(|e| e.path()) {
                if p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("victory-nvidia-") && n.ends_with(".log"))
                        .unwrap_or(false)
                {
                    log_files.push(p);
                }
            }
        }
    }
    log_files.sort();

    let mut changed = false;
    for path in log_files {
        let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let ts = parse_log_filename_ts(fname).unwrap_or_else(|| fname.to_string());
        let lines = read_log_tail(&path, 8000);
        if lines.is_empty() {
            continue;
        }
        let mut runner = if fname.starts_with("victory-nvidia-") {
            "victory-nvidia".to_string()
        } else {
            "victory.exe".to_string()
        };
        let mut console = String::new();
        let mut is_prepare = false;
        for l in lines.iter().take(20) {
            let lo = l.to_ascii_lowercase();
            if lo.contains("victory_menu_console=series-x") || lo.contains("--console series-x") || lo.contains("console=series-x") {
                console = "series-x".into();
            } else if lo.contains("victory_menu_console=series-s") || lo.contains("--console series-s") || lo.contains("console=series-s") {
                console = "series-s".into();
            }
            if let Some((_, v)) = l.split_once(':') {
                let k = l.split(':').next().unwrap_or("").trim();
                let val = v.trim();
                if k == "runner" && !val.is_empty() {
                    runner = val.rsplit(['\\', '/']).next().unwrap_or(val).to_string();
                } else if k == "console" && !val.is_empty() {
                    console = val.to_string();
                } else if k == "prepare_cache" {
                    is_prepare = true;
                }
            }
        }
        if console.is_empty() {
            console = if runner.to_ascii_lowercase().contains("victory-nvidia") {
                "series-x".into()
            } else {
                "series-s".into()
            };
        }
        if is_prepare {
            continue;
        }

        let slow = extract_slow_shaders(&lines, 4);
        let bn = classify_bottleneck(&lines);
        let peak_fps = if let Some(w) = extract_world_avg_fps(&lines) {
            format!("{w:.1} fps (world · {bn})")
        } else {
            extract_peak_fps(&lines)
                .map(|f| format!("{f:.1} fps"))
                .unwrap_or_else(|| "N/A".into())
        };
        let top_shader = slow
            .first()
            .map(|s| {
                let short: String = s.hash.chars().take(8).collect();
                if let Some(ms) = s.ms {
                    format!("{short} ({ms:.0}ms)")
                } else {
                    short
                }
            })
            .unwrap_or_else(|| "-".into());

        if let Some(pos) = existing.iter().position(|r| r.timestamp == ts) {
            if (existing[pos].peak_fps == "-" || existing[pos].peak_fps.is_empty()) && peak_fps != "-" {
                existing[pos].peak_fps = peak_fps;
                existing[pos].top_shader = top_shader;
                changed = true;
            }
        } else {
            let duration_secs = estimate_log_duration_secs(&lines);
            existing.push(SessionRecord {
                timestamp: ts,
                runner,
                adapter: "victory".into(),
                console,
                kind: "GAME RUNNER".into(),
                duration_secs,
                exit_code: "LOG".into(),
                peak_fps,
                top_shader,
            });
            changed = true;
        }
    }

    if changed {
        existing.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        let path = sessions_csv_path();
        if let Ok(mut f) = fs::File::create(&path) {
            let _ = writeln!(f, "timestamp,runner,adapter,console,kind,duration_secs,exit_code,peak_fps,top_shader");
            let clean = |s: &str| s.replace(',', ";").replace('\n', " ").replace('\r', "");
            for rec in &existing {
                let _ = writeln!(
                    f,
                    "{},{},{},{},{},{},{},{},{}",
                    clean(&rec.timestamp),
                    clean(&rec.runner),
                    clean(&rec.adapter),
                    clean(&rec.console),
                    clean(&rec.kind),
                    rec.duration_secs,
                    clean(&rec.exit_code),
                    clean(&rec.peak_fps),
                    clean(&rec.top_shader),
                );
            }
        }
    }
}

fn parse_log_filename_ts(fname: &str) -> Option<String> {
    let stem = fname.strip_prefix("Game-")?.strip_suffix(".log")?;
    let (d, t) = stem.split_once('-')?;
    if d.len() == 8 && t.len() == 6 {
        Some(format!(
            "{}-{}-{} {}:{}:{}",
            &d[0..4], &d[4..6], &d[6..8], &t[0..2], &t[2..4], &t[4..6]
        ))
    } else {
        None
    }
}

fn estimate_log_duration_secs(lines: &[String]) -> u64 {
    let parse_iso_hms = |l: &str| -> Option<u64> {
        let t_pos = l.find('T')?;
        let rest = l.get(t_pos + 1..t_pos + 9)?;
        let h: u64 = rest.get(0..2)?.parse().ok()?;
        let m: u64 = rest.get(3..5)?.parse().ok()?;
        let s: u64 = rest.get(6..8)?.parse().ok()?;
        Some(h * 3600 + m * 60 + s)
    };
    let first = lines.iter().find_map(|l| parse_iso_hms(l));
    let last = lines.iter().rev().find_map(|l| parse_iso_hms(l));
    match (first, last) {
        (Some(a), Some(b)) if b >= a => b - a,
        (Some(a), Some(b)) => (86400 - a) + b,
        _ => 0,
    }
}

/// Groups `sessions.csv` GAME RUNNER entries by runner for Head-to-Head A/B benchmarking,
/// filtering to currently installed runners when `installed_runners` is non-empty.
pub fn aggregate_by_runner(installed_runners: &[String]) -> Vec<RunnerBenchSummary> {
    let all = load_recent_sessions(200);
    let mut out: Vec<RunnerBenchSummary> = Vec::new();
    for rec in all {
        if !rec.kind.eq_ignore_ascii_case("GAME RUNNER") {
            continue;
        }
        if !installed_runners.is_empty()
            && !installed_runners.iter().any(|ir| {
                let ir_lo = ir.to_ascii_lowercase();
                let rec_lo = rec.runner.to_ascii_lowercase();
                ir_lo == rec_lo || ir_lo.ends_with(&rec_lo) || rec_lo.ends_with(&ir_lo)
            })
        {
            continue;
        }
        let fps = rec
            .peak_fps
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        if let Some(entry) = out.iter_mut().find(|e| e.runner.eq_ignore_ascii_case(&rec.runner)) {
            let prev_total = entry.avg_fps * (entry.runs as f64);
            entry.runs += 1;
            entry.avg_fps = (prev_total + fps) / (entry.runs as f64);
            if fps > entry.best_fps {
                entry.best_fps = fps;
            }
            if rec.duration_secs > entry.max_secs {
                entry.max_secs = rec.duration_secs;
            }
        } else {
            out.push(RunnerBenchSummary {
                runner: rec.runner,
                runs: 1,
                best_fps: fps,
                avg_fps: fps,
                max_secs: rec.duration_secs,
            });
        }
    }
    out.sort_by(|a, b| {
        b.best_fps
            .partial_cmp(&a.best_fps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(4);
    out
}

#[derive(Clone, Debug, Default)]
pub struct LogRunStats {
    pub name: String,
    pub console: String,
    pub resolution: String,
    pub flags_summary: String,
    pub avg_fps: Option<f64>,
    pub min_fps: Option<f64>,
    pub max_fps: Option<f64>,
    pub avg_gpu_ms: Option<f64>,
    pub host_mem_warns: usize,
    pub card_copy_warns: usize,
    pub no_room_warns: usize,
}

#[derive(Clone, Debug)]
pub struct LogComparison {
    pub run_a: LogRunStats,
    pub run_b: LogRunStats,
    pub winner: &'static str,
    pub delta_summary: String,
}

fn extract_env_token(line: &str, key: &str) -> Option<String> {
    let idx = line.find(key)?;
    let rest = line.get(idx + key.len()..)?;
    let val = rest.split_whitespace().next()?.trim();
    if val.is_empty() { None } else { Some(val.to_string()) }
}

/// Parses a single log (plain or ANSI-colored) into structured A/B comparison telemetry.
pub fn parse_log_run_stats(name: &str, raw_lines: &[String]) -> LogRunStats {
    let mut st = LogRunStats {
        name: name.to_string(),
        console: "series-s".into(),
        resolution: "auto".into(),
        ..Default::default()
    };
    let mut relay = "-";
    let mut steps = "-";
    let mut tex = "-";
    let mut bmin = "-";
    let mut relay_s = String::new();
    let mut steps_s = String::new();
    let mut tex_s = String::new();
    let mut bmin_s = String::new();

    let mut world_fps: Vec<f64> = Vec::new();
    let mut all_fps: Vec<f64> = Vec::new();
    let mut gpu_ms_samples: Vec<f64> = Vec::new();

    for raw in raw_lines {
        let line = strip_ansi(raw);
        let lo = line.to_ascii_lowercase();

        if lo.starts_with("console") && line.contains(':') {
            if let Some((_, v)) = line.split_once(':') {
                let cv = v.trim();
                if !cv.is_empty() {
                    st.console = cv.to_string();
                }
            }
        }
        if line.contains("VICTORY_DISPLAY_MODE=") {
            if let Some(v) = extract_env_token(&line, "VICTORY_DISPLAY_MODE=") {
                st.resolution = v;
            }
        }
        if let Some(v) = extract_env_token(&line, "VICTORY_RELAY=") {
            relay_s = v;
            relay = &relay_s;
        }
        if let Some(v) = extract_env_token(&line, "VICTORY_CARD_STEPS=") {
            steps_s = v;
            steps = &steps_s;
        }
        if let Some(v) = extract_env_token(&line, "VICTORY_TEXTURE_ROOM_PERCENT=") {
            tex_s = v;
            tex = &tex_s;
        }
        if let Some(v) = extract_env_token(&line, "VICTORY_CARD_BUFFERS_MIN_MB=") {
            bmin_s = v;
            bmin = &bmin_s;
        }

        if lo.contains("host memory pressure") {
            st.host_mem_warns += 1;
        }
        if lo.contains("no card copy for a title buffer") {
            st.card_copy_warns += 1;
        }
        if lo.contains("the card has no room for more") {
            st.no_room_warns += 1;
        }

        if let Some(fi) = line.find(" fps,") {
            if let Some(v) = line[..fi]
                .split_whitespace()
                .last()
                .and_then(|s| s.parse::<f64>().ok())
            {
                if v > 0.05 && v < 500.0 {
                    all_fps.push(v);
                    let draws = line
                        .find(" draws/frame")
                        .and_then(|di| line[..di].split_whitespace().last())
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0);
                    if v < 20.0 || draws >= 500 {
                        world_fps.push(v);
                    }
                }
            }
        }
        if let Some(gi) = line.find(" on the GPU") {
            if let Some(ms) = line[..gi]
                .split_whitespace()
                .last()
                .and_then(|s| s.parse::<f64>().ok())
            {
                if ms > 0.0 && ms < 60000.0 {
                    gpu_ms_samples.push(ms);
                }
            }
        }
    }

    st.flags_summary = format!("R{relay} S{steps} T{tex}% B{bmin}");
    let fps_set = if !world_fps.is_empty() { &world_fps } else { &all_fps };
    if !fps_set.is_empty() {
        let sum: f64 = fps_set.iter().sum();
        let min = fps_set.iter().copied().fold(f64::INFINITY, f64::min);
        let max = fps_set.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        st.avg_fps = Some(sum / fps_set.len() as f64);
        st.min_fps = Some(min);
        st.max_fps = Some(max);
    }
    if !gpu_ms_samples.is_empty() {
        let gsum: f64 = gpu_ms_samples.iter().sum();
        st.avg_gpu_ms = Some(gsum / gpu_ms_samples.len() as f64);
    }
    st
}

pub fn compare_two_stats(run_a: LogRunStats, run_b: LogRunStats) -> LogComparison {
    let fps_a = run_a.avg_fps.unwrap_or(0.0);
    let fps_b = run_b.avg_fps.unwrap_or(0.0);
    let dfps = fps_a - fps_b;
    let dgpu = match (run_a.avg_gpu_ms, run_b.avg_gpu_ms) {
        (Some(ga), Some(gb)) => Some(ga - gb),
        _ => None,
    };
    let winner = if dfps > 0.15 {
        "NEW"
    } else if dfps < -0.15 {
        "PREV"
    } else if let Some(dg) = dgpu {
        if dg < -5.0 { "NEW" } else if dg > 5.0 { "PREV" } else { "TIE" }
    } else {
        "TIE"
    };
    let gpu_txt = dgpu
        .map(|d| format!(" · {:+.1} ms GPU", d))
        .unwrap_or_default();
    let delta_summary = format!("WINNER: {winner} ({:+.1} FPS{gpu_txt})", dfps);
    LogComparison {
        run_a,
        run_b,
        winner,
        delta_summary,
    }
}

/// Parses and compares the two most recent `Game-*.log` files in `logs/`.
pub fn compare_last_two_logs() -> Option<LogComparison> {
    let rd = fs::read_dir(logs_dir()).ok()?;
    let mut files: Vec<(std::time::SystemTime, String, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_str()?.to_string();
            if p.is_file() && name.starts_with("Game-") && name.ends_with(".log") {
                let m = e.metadata().ok()?.modified().ok()?;
                Some((m, name, p))
            } else {
                None
            }
        })
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    if files.len() < 2 {
        return None;
    }
    let lines_a = read_log_tail(&files[0].2, 8000);
    let lines_b = read_log_tail(&files[1].2, 8000);
    let st_a = parse_log_run_stats(&files[0].1, &lines_a);
    let st_b = parse_log_run_stats(&files[1].1, &lines_b);
    Some(compare_two_stats(st_a, st_b))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoctorFix {
    LowerTexRoom90,
    EnableMemTrim,
    PrepareCache,
    SwitchBalanced1080p,
}

impl DoctorFix {
    pub fn label(self) -> &'static str {
        match self {
            DoctorFix::LowerTexRoom90 => "APPLY FIX",
            DoctorFix::EnableMemTrim => "APPLY FIX",
            DoctorFix::PrepareCache => "APPLY FIX",
            DoctorFix::SwitchBalanced1080p => "APPLY FIX",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AutoDoctorReport {
    pub runner_name: String,
    pub console: String,
    pub gpu_label: String,
    pub vram_gb: f64,
    pub ram_gb: f64,
    pub avg_fps: f64,
    pub min_fps: f64,
    pub max_fps: f64,
    pub gpu_ms: f64,
    pub prep_ms: f64,
    pub diagnosis: String,
    pub advice: String,
    pub recommended_fix: Option<DoctorFix>,
    pub crashed: bool,
}

impl AutoDoctorReport {
    pub fn to_anonymized_markdown(&self) -> String {
        format!(
            "### Benchmark Summary\n\
             - **Runner:** `{}` ({})\n\
             - **Hardware:** {} ({:.0} GB VRAM, {:.0} GB RAM)\n\
             - **3D Open-World FPS:** {:.2} avg (min {:.2} / max {:.2})\n\
             - **Frame Breakdown:** GPU {:.1} ms | CPU Prepare {:.1} ms\n\
             - **Auto-Doctor:** {} — {}\n",
            self.runner_name,
            self.console,
            self.gpu_label,
            self.vram_gb,
            self.ram_gb,
            self.avg_fps,
            self.min_fps,
            self.max_fps,
            self.gpu_ms,
            self.prep_ms,
            self.diagnosis,
            self.advice,
        )
    }
}

/// 5% Benchmark Gate: returns true only when candidate beats baseline by >5% with zero crashes.
pub fn benchmark_beats_baseline(candidate_fps: f64, baseline_fps: f64, crashed: bool) -> bool {
    !crashed && baseline_fps > 0.0 && candidate_fps >= baseline_fps * 1.05
}

/// Parses a finished session log into a plain-English Auto-Doctor report.
pub fn analyze_session_log(runner_name: &str, log_path: &Path, sys: &SystemInfo, crashed: bool) -> AutoDoctorReport {
    let lines = read_log_tail(log_path, 6000);
    let stats = parse_log_run_stats(runner_name, &lines);
    let mut no_room = false;
    let mut pressure = false;
    let mut missing_pipe = 0usize;
    for raw in &lines {
        let l = strip_ansi(raw);
        if l.contains("the card has no room for more") {
            no_room = true;
        }
        if l.contains("host memory pressure") {
            pressure = true;
        }
        if l.contains("without a host pipeline") && !l.contains("dropped 0 without a host pipeline") {
            missing_pipe += 1;
        }
    }
    let avg_fps = stats.avg_fps.unwrap_or(0.0);
    let min_fps = stats.min_fps.unwrap_or(0.0);
    let max_fps = stats.max_fps.unwrap_or(0.0);
    let gpu_ms = stats.avg_gpu_ms.unwrap_or(0.0);
    let prep_ms = 0.0_f64;

    let is_streamline = runner_name.to_ascii_lowercase().contains("victory-nvidia")
        || runner_name.to_ascii_lowercase().contains("streamline");
    let (diagnosis, advice, recommended_fix) = if crashed {
        (
            "Session Crashed".to_string(),
            if is_streamline {
                "Switch to BALANCED 1440p DLSS or rollback to pinned baseline.".to_string()
            } else {
                "Switch to BALANCED 1080p or rollback to pinned baseline.".to_string()
            },
            Some(DoctorFix::SwitchBalanced1080p),
        )
    } else if missing_pipe > 10 && avg_fps < 5.0 {
        (
            "Missing Pipeline Cache".to_string(),
            "Run Prepare Cache (shaders --jobs + warm) before launching.".to_string(),
            Some(DoctorFix::PrepareCache),
        )
    } else if no_room {
        if is_streamline {
            (
                "VRAM Ceiling Reached".to_string(),
                "Lower texture room to 72% (or DLSS Performance) to preserve Streamline FG headroom.".to_string(),
                Some(DoctorFix::LowerTexRoom90),
            )
        } else {
            (
                "VRAM Ceiling Reached".to_string(),
                "Cap texture room to 90% and target room to 2048 MB to avoid WDDM spillover.".to_string(),
                Some(DoctorFix::LowerTexRoom90),
            )
        }
    } else if pressure {
        (
            "Host RAM Pressure".to_string(),
            "Enable idle allocator/upload memory trimming and close background RAM hogs.".to_string(),
            Some(DoctorFix::EnableMemTrim),
        )
    } else {
        (
            "Clean Run".to_string(),
            "No VRAM exhaustion or host memory pressure detected.".to_string(),
            None,
        )
    };

    AutoDoctorReport {
        runner_name: runner_name.to_string(),
        console: if stats.console.is_empty() { "series-s".into() } else { stats.console },
        gpu_label: if sys.gpu_name.is_empty() { sys.gpu_arch.label().into() } else { sys.gpu_name.clone() },
        vram_gb: sys.vram_gb,
        ram_gb: sys.total_ram_gb,
        avg_fps,
        min_fps,
        max_fps,
        gpu_ms,
        prep_ms,
        diagnosis,
        advice,
        recommended_fix,
        crashed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_comparator_parses_ansi_and_plain_open_world_metrics() {
        let log_new = vec![
            "console          : series-s".into(),
            "gpu_offload_env  : VICTORY_DISPLAY_MODE=1600x900 VICTORY_RELAY=2 VICTORY_CARD_STEPS=2 VICTORY_TEXTURE_ROOM_PERCENT=94 VICTORY_CARD_BUFFERS_MIN_MB=4".into(),
            "\x1b[32m2026-11-26T12:00:00Z INFO victory_gpu::host: 6.4 fps, 155.0 ms/frame (12.0 preparing; 120.0 on the GPU; 10.0 copying back), 3200 draws/frame\x1b[0m".into(),
            "2026-11-26T12:00:05Z INFO victory_gpu::host: 7.2 fps, 138.0 ms/frame (12.0 preparing; 100.0 on the GPU; 10.0 copying back), 3400 draws/frame".into(),
        ];
        let log_old = vec![
            "console          : series-s".into(),
            "gpu_offload_env  : VICTORY_DISPLAY_MODE=1280x720 VICTORY_RELAY=1 VICTORY_CARD_STEPS=1 VICTORY_TEXTURE_ROOM_PERCENT=82 VICTORY_CARD_BUFFERS_MIN_MB=1".into(),
            "2026-11-26T11:00:00Z INFO victory_gpu::host: 1.2 fps, 833.0 ms/frame (40.0 preparing; 780.0 on the GPU; 20.0 copying back), 3100 draws/frame".into(),
            "WARN victory_gpu::residency: host memory pressure: limiting texture prefetch".into(),
            "WARN victory_gpu::draw: no card copy for a title buffer".into(),
            "WARN victory_gpu::draw: the card has no room for more".into(),
        ];
        let a = parse_log_run_stats("Game-new.log", &log_new);
        let b = parse_log_run_stats("Game-old.log", &log_old);
        assert_eq!(a.resolution, "1600x900");
        assert_eq!(a.flags_summary, "R2 S2 T94% B4");
        assert!((a.avg_fps.unwrap() - 6.8).abs() < 0.01);
        assert!((a.avg_gpu_ms.unwrap() - 110.0).abs() < 0.01);
        assert_eq!(b.host_mem_warns, 1);
        assert_eq!(b.card_copy_warns, 1);
        assert_eq!(b.no_room_warns, 1);
        let cmp = compare_two_stats(a, b);
        assert_eq!(cmp.winner, "NEW");
    }
}
