use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::diagnostics::{extract_slow_shaders, logs_dir, read_log_tail, reports_dir, SlowShader};
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

/// Extracts average 3D-world FPS (lines with `draws/frame` >= 500),
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

/// Classifies the primary runtime bottleneck directly from runner log signals.
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

/// Rebuilds and merges `sessions.csv` from all `GM6-*.log` files in `logs/` so manual closes
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

    let Ok(rd) = fs::read_dir(logs_dir()) else {
        return;
    };
    let mut log_files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("GM6-") && n.ends_with(".log"))
                    .unwrap_or(false)
        })
        .collect();
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
        let mut runner = "victory.exe".to_string();
        let mut console = "series-s".to_string();
        let mut is_prepare = false;
        for l in lines.iter().take(20) {
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
                .unwrap_or_else(|| "-".into())
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
                adapter: "baseline".into(),
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
    let stem = fname.strip_prefix("GM6-")?.strip_suffix(".log")?;
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
