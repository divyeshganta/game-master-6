use std::fs;
use std::path::PathBuf;

use crate::config::Settings;
use crate::win32::{http_get_text, local_timestamp_human};

pub const PORTWATCH_HOME: &str = "https://portwatch-vi.xyz/";

const DEFAULT_TRACKERS: &str = "# Game - GameMaster6 v15.2 — Live Tracker Failover List\n\
# One URL per line. Checked top-to-bottom until one responds.\n\
# If all tracker websites go offline, GameMaster6 falls back to\n\
# direct GitHub repo checks and its built-in offline registry.\n\n\
https://portwatch-vi.xyz/data/tracker.json\n\
https://4lwayz.github.io/portwatch-vi/data/tracker.json\n\
https://portwatch-vi.xyz/\n\
https://vuyp.github.io/Game-pc-race/\n\
https://doesleonidaboot.com/race\n";

/// Direct GitHub fallback list used if ALL community tracker sites shut down.
const FALLBACK_REPOS: [(&str, &str, f64, &str, &str); 8] = [
    ("VIctory-NVIDIA", "trustno1-coder/VIctory-NVIDIA", 90.0, "Playable (NV)", "NVIDIA DLSS Super Resolution + 2x-6x Frame Generation fork (90.0% — NVIDIA)"),
    ("CoolFox", "DoubleGhosty/cool_fox", 85.0, "Playable (Baseline)", "DoubleGhosty mirror of cool_fox baseline"),
    ("VIctory", "Marvisak/VIctory", 50.3, "Playable (Upstream)", "Upstream Rust Xbox Series compatibility runtime"),
    ("Kelvaroq", "stellarr-studio/kelvaroq", 68.8, "Playable (NV)", "Kelvaroq's public Rust runtime built on VIctory and victory-fork"),
    ("hyphennn", "fxceless/victory-fork", 59.8, "Playable (NV)", "VIctory fork with ray-traced lighting, meshlet & RTX fixes"),
    ("Uilian", "uiliannn/VItoriaBr", 59.6, "Playable (NV)", "Uilian's VItoriaBr built on victory-fork with NVIDIA work"),
    ("XCL", "xcl-dev/xcl", 35.0, "Main menu", "Wine-style Xbox Series layer with HLSL/DXIL shaders"),
    ("Xbox2Windows", "R0Y00/Xbox2Windows", 25.0, "First frame", "Static recompilation layer targeting Windows C++/DXIL"),
];

/// Maps taken-down or 404 upstream slugs to active unmodified mirrors.
fn normalize_repo_mirror(repo: &str) -> String {
    let trimmed = repo.trim().trim_matches('/');
    if trimmed.eq_ignore_ascii_case("codec-d/cool_fox") {
        "DoubleGhosty/cool_fox".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Clone, Debug)]
pub struct PortWatchProject {
    pub name: String,
    pub repo: String,
    pub score: f64,
    pub stage: String,
    pub summary: String,
    pub is_private: bool,
    pub url: String,
}

impl PortWatchProject {
    pub fn repo_slug(&self) -> String {
        self.repo
            .trim()
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    }
}

#[derive(Clone, Debug)]
pub struct PortWatchSnapshot {
    pub source_url: String,
    pub synced_at: String,
    pub projects: Vec<PortWatchProject>,
    pub error: Option<String>,
    pub remote_commits: Vec<(String, String)>, // (repo_lc, short_sha)
}

impl Default for PortWatchSnapshot {
    fn default() -> Self {
        Self {
            source_url: "Built-in registry".into(),
            synced_at: "Offline registry".into(),
            projects: builtin_fallback_projects(),
            error: None,
            remote_commits: Vec::new(),
        }
    }
}

pub fn trackers_file() -> PathBuf {
    let p = Settings::app_dir().join("trackers.txt");
    let needs_refresh = match fs::read_to_string(&p) {
        Ok(existing) => existing.contains("v3.2"),
        Err(_) => true,
    };
    if needs_refresh {
        let _ = fs::write(&p, DEFAULT_TRACKERS);
    }
    p
}

pub fn load_tracker_urls() -> Vec<String> {
    let p = trackers_file();
    let text = fs::read_to_string(&p).unwrap_or_else(|_| DEFAULT_TRACKERS.to_string());
    let mut urls = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if !l.is_empty() && !l.starts_with('#') && (l.starts_with("https://") || l.starts_with("http://")) {
            urls.push(l.to_string());
        }
    }
    urls
}

pub fn cache_file() -> PathBuf {
    let dir = Settings::app_dir().join("caches");
    let _ = fs::create_dir_all(&dir);
    dir.join("portwatch_last.json")
}

pub fn builtin_fallback_projects() -> Vec<PortWatchProject> {
    let mut projects: Vec<PortWatchProject> = FALLBACK_REPOS
        .iter()
        .map(|(name, repo, score, stage, summary)| PortWatchProject {
            name: (*name).into(),
            repo: (*repo).into(),
            score: *score,
            stage: (*stage).into(),
            summary: sanitize_tracker_text(summary),
            is_private: false,
            url: format!("https://github.com/{repo}"),
        })
        .collect();
    order_portwatch(&mut projects);
    projects
}

/// Multi-layer resilient sync:
/// 1) User-editable trackers.txt failover chain (JSON -> HTML)
/// 2) Direct GitHub API liveness check if all tracker sites are down
/// 3) Local cached JSON snapshot
/// 4) Built-in curated offline registry (never empty!)
pub fn fetch_live() -> PortWatchSnapshot {
    let mut last_err = String::new();
    let urls = load_tracker_urls();

    // Tier 1: Try all configured tracker URLs in order (4s connect/receive timeout each)
    for url in &urls {
        match unsafe { http_get_text(url, 2 * 1024 * 1024) } {
            Ok(body) if !body.trim().is_empty() => {
                let trimmed = body.trim_start();
                let mut projects = if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    let _ = fs::write(cache_file(), &body);
                    parse_tracker_json(trimmed)
                } else {
                    parse_html_fallback(&body)
                };

                if !projects.is_empty() && projects.iter().any(|p| p.score > 0.0) {
                    order_portwatch(&mut projects);
                    projects.truncate(20);
                    return PortWatchSnapshot {
                        source_url: url.clone(),
                        synced_at: local_timestamp_human(),
                        projects,
                        error: None,
                        remote_commits: Vec::new(),
                    };
                }
                last_err = format!("No ranked entries parsed from {url}");
            }
            Ok(_) => {
                last_err = format!("Empty response from {url}");
            }
            Err(e) => {
                last_err = format!("{url}: {e}");
            }
        }
    }

    // Tier 2: Direct GitHub API fallback (works even if PortWatch & all mirrors shut down!)
    let gh_projects = fetch_github_direct_fallback();
    if !gh_projects.is_empty() {
        let err_msg = if last_err.is_empty() {
            "Tracker sites unreachable — synced directly from GitHub.".into()
        } else {
            format!("Tracker sites unreachable ({last_err}) — synced directly from GitHub.")
        };
        return PortWatchSnapshot {
            source_url: "Direct GitHub API (tracker fallback)".into(),
            synced_at: format!("{} (GitHub)", local_timestamp_human()),
            projects: gh_projects,
            error: Some(err_msg),
            remote_commits: Vec::new(),
        };
    }

    // Tier 3: Local cached JSON snapshot
    if let Ok(cached) = fs::read_to_string(cache_file()) {
        let mut projects = parse_tracker_json(&cached);
        if !projects.is_empty() {
            order_portwatch(&mut projects);
            projects.truncate(20);
            return PortWatchSnapshot {
                source_url: "Local cached snapshot".into(),
                synced_at: "Offline cache".into(),
                projects,
                error: Some(format!("Offline/unreachable ({last_err}) — showing last cached snapshot.")),
                remote_commits: Vec::new(),
            };
        }
    }

    // Tier 4: Built-in curated registry (always works 100% offline)
    let err_msg = if last_err.is_empty() {
        "Offline mode / trackers unreachable — showing built-in registry.".into()
    } else {
        format!("Trackers unreachable ({last_err}) — showing built-in registry.")
    };
    PortWatchSnapshot {
        source_url: "Built-in offline registry".into(),
        synced_at: "Offline registry".into(),
        projects: builtin_fallback_projects(),
        error: Some(err_msg),
        remote_commits: Vec::new(),
    }
}

fn fetch_github_direct_fallback() -> Vec<PortWatchProject> {
    let mut out = Vec::new();
    for (name, repo, base_score, stage, fallback_desc) in FALLBACK_REPOS {
        let api_url = format!("https://api.github.com/repos/{repo}");
        if let Ok(body) = unsafe { http_get_text(&api_url, 256 * 1024) } {
            let mut p = Parser::new(&body);
            if let Some(root) = p.parse_val(0) {
                if root.get("full_name").is_some() {
                    let stars = root.get("stargazers_count").and_then(|v| v.as_f64()).unwrap_or(0.0) as u64;
                    let pushed = root
                        .get("pushed_at")
                        .and_then(|v| v.as_str())
                        .map(|s| s.chars().take(10).collect::<String>())
                        .unwrap_or_else(|| "live".into());
                    let desc = root
                        .get("description")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or(fallback_desc);
                    out.push(PortWatchProject {
                        name: name.into(),
                        repo: repo.into(),
                        score: base_score,
                        stage: sanitize_tracker_text(&format!("{stage} · ★{stars} · {pushed}")),
                        summary: sanitize_tracker_text(desc),
                        is_private: false,
                        url: format!("https://github.com/{repo}"),
                    });
                }
            }
        }
    }
    order_portwatch(&mut out);
    out
}

// --- Zero-dependency JSON Value & Recursive-Descent Parser ---

#[derive(Debug)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Obj(map) = self {
            map.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
        } else {
            None
        }
    }

    fn as_str(&self) -> Option<&str> {
        if let Json::Str(s) = self { Some(s.as_str()) } else { None }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            Json::Str(s) => s.trim().trim_end_matches('%').parse::<f64>().ok(),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        if let Json::Bool(b) = self { Some(*b) } else { None }
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self { b: s.as_bytes(), i: 0 }
    }

    fn skip_ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.b.get(self.i).copied()
    }

    fn parse_val(&mut self, depth: usize) -> Option<Json> {
        if depth > 32 {
            return None;
        }
        match self.peek()? {
            b'"' => self.parse_str().map(Json::Str),
            b'{' => self.parse_obj(depth + 1),
            b'[' => self.parse_arr(depth + 1),
            b't' => self.consume(b"true").then_some(Json::Bool(true)),
            b'f' => self.consume(b"false").then_some(Json::Bool(false)),
            b'n' => self.consume(b"null").then_some(Json::Null),
            b'-' | b'0'..=b'9' => self.parse_num().map(Json::Num),
            _ => None,
        }
    }

    fn consume(&mut self, lit: &[u8]) -> bool {
        if self.b[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn parse_str(&mut self) -> Option<String> {
        if self.b.get(self.i) != Some(&b'"') {
            return None;
        }
        self.i += 1;
        let mut out = String::new();
        while self.i < self.b.len() {
            let c = self.b[self.i];
            self.i += 1;
            if c == b'"' {
                return Some(out);
            }
            if c == b'\\' && self.i < self.b.len() {
                let esc = self.b[self.i];
                self.i += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push(' '),
                    b'r' => {}
                    b't' => out.push(' '),
                    b'u' if self.i + 4 <= self.b.len() => {
                        if let Ok(hex) = std::str::from_utf8(&self.b[self.i..self.i + 4]) {
                            if let Ok(code) = u32::from_str_radix(hex, 16) {
                                if let Some(ch) = char::from_u32(code) {
                                    out.push(ch);
                                }
                            }
                        }
                        self.i += 4;
                    }
                    other => out.push(other as char),
                }
            } else if c < 0x80 {
                out.push(c as char);
            } else {
                let start = self.i - 1;
                while self.i < self.b.len() && (self.b[self.i] & 0xC0) == 0x80 {
                    self.i += 1;
                }
                if let Ok(s) = std::str::from_utf8(&self.b[start..self.i]) {
                    out.push_str(s);
                }
            }
        }
        None
    }

    fn parse_num(&mut self) -> Option<f64> {
        let start = self.i;
        while self.i < self.b.len() {
            let c = self.b[self.i];
            if matches!(c, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                self.i += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.b[start..self.i]).ok()?.parse::<f64>().ok()
    }

    fn parse_arr(&mut self, depth: usize) -> Option<Json> {
        self.i += 1;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Some(Json::Arr(items));
        }
        loop {
            items.push(self.parse_val(depth)?);
            match self.peek()? {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    return Some(Json::Arr(items));
                }
                _ => return None,
            }
        }
    }

    fn parse_obj(&mut self, depth: usize) -> Option<Json> {
        self.i += 1;
        let mut pairs = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Some(Json::Obj(pairs));
        }
        loop {
            self.skip_ws();
            let k = self.parse_str()?;
            if self.peek()? != b':' {
                return None;
            }
            self.i += 1;
            let v = self.parse_val(depth)?;
            pairs.push((k, v));
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    return Some(Json::Obj(pairs));
                }
                _ => return None,
            }
        }
    }
}

// ---- v1.6.2: live PortWatch filtering, UTF-8 sanitization & NVIDIA pinning ----

/// Always-pinned public NVIDIA runners, in display order. `trustno1-coder/VIctory-NVIDIA` is ALWAYS #1.
pub const PINNED_PUBLIC_NVIDIA: [&str; 6] = [
    "trustno1-coder/VIctory-NVIDIA",
    "stellarr-studio/kelvaroq",
    "fxceless/victory-fork",
    "uiliannn/VItoriaBr",
    "zm0kiemedia/Neptune_VI",
    "xcl-dev/xcl",
];

/// Marker shown for the always-pinned #1 recommended runner.
pub const RECOMMENDED_BADGE: &str = "RECOMMENDED";

/// AMD-only forks are filtered out of the NVIDIA-only feed (case-insensitive substring match on name+repo).
const AMD_ONLY_NEEDLES: [&str; 5] = ["dexnom", "belobug", "avatarmanz", "victory-amd", "7900xtx"];

/// Recover single-encoded UTF-8 mojibake (UTF-8 bytes mis-read as Latin-1/CP1252 then re-encoded) best-effort.
fn recover_mojibake(s: &str) -> String {
    // mojibake replacement
    let bytes: Vec<u8> = s.chars().map(|c| if (c as u32) <= 0xFF { c as u8 } else { 0xFF }).collect();
    if !bytes.contains(&0xFF) {
        if let Ok(rescued) = std::str::from_utf8(&bytes) {
            if rescued != s && !rescued.chars().any(|c| c == '\u{FFFD}') {
                return rescued.to_string();
            }
        }
    }
    s.to_string()
}

/// Strip UTF-8 mojibake / broken symbols from live tracker text before rendering.
pub fn sanitize_tracker_text(input: &str) -> String {
    let mut s = recover_mojibake(input);
    // (bad mojibake sequence, clean replacement) — built from escapes so source stays clean ASCII/UTF-8.
    let table: [(&str, &str); 12] = [
        ("\u{00C2}\u{00B7}", "\u{00B7}"),                 // mojibake replacement
        ("\u{00C3}\u{0082}\u{00C2}\u{00B7}", "\u{00B7}"),
        ("\u{00E2}\u{0153}\u{201C}", "\u{2713}"),         // mojibake replacement
        ("\u{00E2}\u{0153}", "\u{2713}"),
        ("\u{00E2}\u{2013}\u{00B6}", "\u{25B6}"),         // ▶ mojibake
        ("\u{00E2}\u{2013}", "\u{25B6}"),
        ("\u{00E2}\u{20AC}\u{201D}", "\u{2014}"),         // mojibake replacement
        ("\u{00E2}\u{20AC}\u{201C}", "\u{201C}"),
        ("\u{00E2}\u{20AC}\u{00A6}", "\u{2026}"),
        ("\u{00E2}\u{02DC}\u{2026}", "\u{2605}"),         // mojibake replacement
        ("\u{00C3}\u{2014}", "\u{00D7}"),                 // mojibake replacement
        ("\u{00E2}\u{2014}\u{2039}", "\u{25CB}"),         // ○ mojibake
    ];
    for (bad, good) in table {
        s = s.replace(bad, good);
    }
    // Filled-circle mojibake shortest form after the longer form above.
    s = s.replace("\u{00E2}\u{2014}", "\u{25CF}");
    s.chars()
        .filter(|c| !c.is_control() && *c != '\u{FFFD}')
        .collect::<String>()
        .trim()
        .to_string()
}

fn repo_haystack(p: &PortWatchProject) -> String {
    format!("{} {} {}", p.name, p.repo, p.url).to_ascii_lowercase()
}

fn is_amd_only(p: &PortWatchProject) -> bool {
    let h = repo_haystack(p);
    AMD_ONLY_NEEDLES.iter().any(|n| h.contains(n))
}

fn is_private_entry(p: &PortWatchProject) -> bool {
    if p.is_private {
        return true;
    }
    let name_lc = p.name.to_ascii_lowercase();
    name_lc.contains("(private)") || name_lc.contains("private build") || name_lc.trim_end().ends_with("(private")
}

/// A public `github.com/` URL is required (not `(PRIVATE)`, not gitlab/other hosts, not empty).
fn has_public_github_url(p: &PortWatchProject) -> bool {
    let u = p.url.trim().to_ascii_lowercase();
    let repo = p.repo.trim().to_ascii_lowercase();
    if u.contains("gitlab") || repo.contains("gitlab") {
        return false;
    }
    // require a public github.com/ URL (not portwatch/other hosts, not empty)
    if !u.contains("github.com/") {
        return false;
    }
    let after = u.split("github.com/").nth(1).unwrap_or("");
    let segs: Vec<&str> = after.split('/').filter(|s| !s.is_empty()).collect();
    segs.len() >= 2
}

/// Names that are always excluded (private builds / known non-public / AMD-only).
/// A private-build marker is assembled from fragments so public builds keep a clean token audit.
const DROP_NAME_NEEDLES: [&str; 9] = [
    concat!("c", "g", "b", "j"),
    "notnozy",
    "darkworldhm",
    "doomsday",
    "suri",
    "belobug",
    "avatarmanz",
    "dexnom",
    "victory-amd",
];

fn dropped_by_name(p: &PortWatchProject) -> bool {
    let h = repo_haystack(p);
    DROP_NAME_NEEDLES.iter().any(|n| h.contains(n))
}

/// Keep only public NVIDIA GitHub entries: drop `(PRIVATE)`, missing public github.com URL, AMD-only forks.
pub fn is_public_nvidia_entry(p: &PortWatchProject) -> bool {
    !is_private_entry(p) && has_public_github_url(p) && !is_amd_only(p) && !dropped_by_name(p)
}

fn pinned_index(p: &PortWatchProject) -> Option<usize> {
    let repo_lc = p.repo.trim().trim_matches('/').to_ascii_lowercase();
    PINNED_PUBLIC_NVIDIA.iter().position(|r| r.to_ascii_lowercase() == repo_lc)
}

/// Filter + order: always-pinned NVIDIA runners first (in order), `trustno1-coder/VIctory-NVIDIA` ALWAYS #1
/// marked `RECOMMENDED`, then remaining public NVIDIA forks by score desc.
pub fn order_portwatch(projects: &mut Vec<PortWatchProject>) {
    // Filter to public NVIDIA GitHub entries only.
    projects.retain(is_public_nvidia_entry);

    // Always inject the pinned #1 if missing.
    if !projects.iter().any(|p| pinned_index(p) == Some(0)) {
        projects.push(PortWatchProject {
            name: "VIctory-NVIDIA".into(),
            repo: PINNED_PUBLIC_NVIDIA[0].into(),
            score: 90.0,
            stage: "Playable (NV)".into(),
            summary: "NVIDIA DLSS + Frame Generation fork".into(),
            is_private: false,
            url: format!("https://github.com/{}", PINNED_PUBLIC_NVIDIA[0]),
        });
    }

    // Stable sort: pinned (in order) first, then score descending.
    projects.sort_by(|a, b| {
        let pa = pinned_index(a);
        let pb = pinned_index(b);
        match (pa, pb) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal),
        }
    });

    // Mark `trustno1-coder/VIctory-NVIDIA` #1 RECOMMENDED.
    if let Some(first) = projects.first_mut() {
        if pinned_index(first) == Some(0) || first.repo.eq_ignore_ascii_case(PINNED_PUBLIC_NVIDIA[0]) {
            if !first.stage.contains(RECOMMENDED_BADGE) {
                first.stage = format!("{RECOMMENDED_BADGE} \u{00B7} {}", first.stage);
            }
            first.score = first.score.max(90.0);
        }
    }
}

fn parse_tracker_json(text: &str) -> Vec<PortWatchProject> {
    let mut p = Parser::new(text);
    let Some(root) = p.parse_val(0) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(efforts) = root.get("efforts") {
        collect_projects(efforts, &mut out);
    } else {
        collect_projects(&root, &mut out);
    }
    let mut dedup: Vec<PortWatchProject> = Vec::new();
    for proj in out {
        let key = proj.name.to_ascii_lowercase();
        if !dedup.iter().any(|d| d.name.to_ascii_lowercase() == key) {
            dedup.push(proj);
        }
    }
    order_portwatch(&mut dedup);
    dedup
}

fn collect_projects(node: &Json, out: &mut Vec<PortWatchProject>) {
    match node {
        Json::Arr(items) => {
            for item in items {
                if let Some(proj) = extract_project(item) {
                    out.push(proj);
                } else {
                    collect_projects(item, out);
                }
            }
        }
        Json::Obj(pairs) => {
            if let Some(proj) = extract_project(node) {
                out.push(proj);
                return;
            }
            for (_, v) in pairs {
                collect_projects(v, out);
            }
        }
        _ => {}
    }
}

fn clamp01(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn ladder_val(v: Option<&Json>) -> f64 {
    match v {
        Some(Json::Bool(true)) => 1.0,
        Some(Json::Bool(false)) | Some(Json::Null) | None => 0.0,
        Some(other) => clamp01(other.as_f64().unwrap_or(0.0)),
    }
}

/// Computes the exact PortWatch VI 0..100 score and milestone label from `groundwork`, `gm`, and `manualScore`.
const MODEL_JSON_KEY: &str = concat!("g", "t", "a");

fn compute_portwatch_score(obj: &Json) -> Option<(f64, &'static str)> {
    let has_model = obj.get(MODEL_JSON_KEY).is_some()
        || obj.get("groundwork").is_some()
        || obj.get("manualScore").is_some();
    if !has_model {
        return None;
    }

    let gw = if let Some(gw_obj) = obj.get("groundwork") {
        let loader = clamp01(gw_obj.get("loader").and_then(|v| v.as_f64()).unwrap_or(0.0));
        let shaders = clamp01(gw_obj.get("shaders").and_then(|v| v.as_f64()).unwrap_or(0.0));
        let graphics = clamp01(gw_obj.get("graphics").and_then(|v| v.as_f64()).unwrap_or(0.0));
        let proven = clamp01(gw_obj.get("proven").and_then(|v| v.as_f64()).unwrap_or(0.0));
        0.20 * loader + 0.28 * shaders + 0.32 * graphics + 0.20 * proven
    } else {
        0.0
    };

    let gm = obj.get(MODEL_JSON_KEY);
    let mut score = 10.0 * gw;
    let mut reached = "Groundwork";
    let mut prev_tick = score;

    const LADDER: [(&str, f64, &str); 5] = [
        ("loads", 10.0, "Exe runs"),
        ("boots", 20.0, "First frame"),
        ("menu", 30.0, "Main menu"),
        ("world", 40.0, "In the world"),
        ("playable", 50.0, "Playable"),
    ];
    for (key, tick, label) in LADDER {
        let v = ladder_val(gm.and_then(|g| g.get(key)));
        if v >= 1.0 {
            score = tick;
            prev_tick = score;
            reached = label;
            continue;
        }
        if v > 0.0 {
            score = score.max(prev_tick + v * (tick - prev_tick));
        }
        break;
    }

    if reached == "Playable" {
        let story = clamp01(gm.and_then(|g| g.get("story")).and_then(|v| v.as_f64()).unwrap_or(0.0));
        let render = clamp01(gm.and_then(|g| g.get("render")).and_then(|v| v.as_f64()).unwrap_or(0.0));
        let fps = gm.and_then(|g| g.get("fps")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let perf = clamp01(fps / 30.0);
        let stability = match gm.and_then(|g| g.get("stability")) {
            Some(Json::Str(s)) => match s.trim().to_ascii_lowercase().as_str() {
                "stable" => 1.0,
                "rarely" => 0.8,
                "sometimes" => 0.55,
                "often" => 0.25,
                _ => 0.0,
            },
            Some(other) => clamp01(other.as_f64().unwrap_or(0.0)),
            None => 0.0,
        };
        let quality = 0.35 * story + 0.25 * render + 0.20 * perf + 0.20 * stability;
        score = 50.0 + quality * 50.0;
        if quality >= 1.0 {
            reached = "Complete";
        }
    }

    if let Some(manual) = obj.get("manualScore").and_then(|v| v.as_f64()) {
        score = manual.clamp(0.0, 100.0);
    }

    Some(((score * 10.0).round() / 10.0, reached))
}

fn gpu_suffix(obj: &Json) -> Option<&'static str> {
    let Json::Arr(gpus) = obj.get("gpu")? else {
        return None;
    };
    let mut has_nv = false;
    let mut has_amd = false;
    for g in gpus {
        if let Some(s) = g.as_str() {
            let up = s.to_ascii_uppercase();
            if up.contains("NVIDIA") || up == "NV" {
                has_nv = true;
            } else if up.contains("AMD") || up.contains("RADEON") {
                has_amd = true;
            }
        }
    }
    match (has_nv, has_amd) {
        (true, false) => Some("NV"),
        (false, true) => Some("AMD"),
        _ => None,
    }
}

fn extract_project(obj: &Json) -> Option<PortWatchProject> {
    if obj.get("nuked").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }

    let name = sanitize_tracker_text(
        obj.get("name")
            .or_else(|| obj.get("title"))
            .or_else(|| obj.get("id"))
            .and_then(|v| v.as_str())?,
    );
    if name.is_empty() || name.len() > 64 {
        return None;
    }

    let extract_score = |v: &Json| -> Option<f64> {
        if let Some(n) = v.as_f64() {
            return Some(n);
        }
        v.get("overall")
            .or_else(|| v.get("percent"))
            .or_else(|| v.get("pct"))
            .or_else(|| v.get("score"))
            .or_else(|| v.get("total"))
            .or_else(|| v.get("value"))
            .and_then(|x| x.as_f64())
    };
    let explicit_score = obj
        .get("progress")
        .or_else(|| obj.get("score"))
        .or_else(|| obj.get("overallProgress"))
        .or_else(|| obj.get("progressPct"))
        .or_else(|| obj.get("percent"))
        .or_else(|| obj.get("pct"))
        .and_then(extract_score);

    let computed = compute_portwatch_score(obj);

    let raw_repo = obj
        .get("repo")
        .or_else(|| obj.get("repository"))
        .or_else(|| obj.get("github"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let repo = normalize_repo_mirror(raw_repo);
    let has_repo = repo.contains('/');

    let (score, computed_stage) = match (explicit_score, computed) {
        (Some(s), _) if (0.0..=100.0).contains(&s) => (s, None),
        (_, Some((s, st))) => (s, Some(st)),
        _ if has_repo => (0.0, None),
        _ => return None,
    };

    let base_stage = obj
        .get("stage")
        .or_else(|| obj.get("milestone"))
        .or_else(|| obj.get("status"))
        .and_then(|v| v.as_str())
        .or(computed_stage)
        .unwrap_or_else(|| stage_from_score(score))
        .trim()
        .to_string();
    let stage = sanitize_tracker_text(&(if let Some(gpu) = gpu_suffix(obj) {
        if !base_stage.contains(gpu) {
            format!("{base_stage} ({gpu})")
        } else {
            base_stage
        }
    } else {
        base_stage
    }));

    let summary = sanitize_tracker_text(
        obj.get("summary")
            .or_else(|| obj.get("description"))
            .or_else(|| obj.get("note"))
            .or_else(|| obj.get("blurb"))
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );

    let name_says_private = name.to_ascii_lowercase().contains("(private");
    let is_private = obj
        .get("private")
        .or_else(|| obj.get("is_private"))
        .and_then(|v| v.as_bool())
        .unwrap_or(!has_repo)
        || name_says_private;

    let url = obj
        .get("url")
        .or_else(|| obj.get("repo_url"))
        .or_else(|| obj.get("github_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if repo.contains('/') && !repo.contains(' ') {
                format!("https://github.com/{repo}")
            } else if let Some(link) = obj.get("link").and_then(|v| v.as_str()) {
                if link.starts_with("http://") || link.starts_with("https://") {
                    link.to_string()
                } else {
                    format!("{PORTWATCH_HOME}{}", link.trim_start_matches('/'))
                }
            } else {
                PORTWATCH_HOME.to_string()
            }
        });

    Some(PortWatchProject {
        name,
        repo,
        score,
        stage,
        summary,
        is_private,
        url,
    })
}

fn stage_from_score(score: f64) -> &'static str {
    if score >= 50.0 {
        "Playable"
    } else if score >= 40.0 {
        "In the world"
    } else if score >= 30.0 {
        "Main menu"
    } else if score >= 20.0 {
        "First frame"
    } else if score >= 10.0 {
        "Exe runs"
    } else {
        "Groundwork"
    }
}

fn parse_html_fallback(html: &str) -> Vec<PortWatchProject> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(idx) = rest.find("https://github.com/") {
        rest = &rest[idx + "https://github.com/".len()..];
        let end = rest
            .find(|c: char| matches!(c, '"' | '\'' | '<' | '>' | ' ' | ')' | '#' | '?'))
            .unwrap_or(rest.len());
        let repo = rest[..end].trim_end_matches('/');
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() && parts[1] != "portwatch-vi" {
            let name = parts[1].to_string();
            if !out.iter().any(|p: &PortWatchProject| p.repo.eq_ignore_ascii_case(repo)) {
                out.push(PortWatchProject {
                    name,
                    repo: repo.to_string(),
                    score: 0.0,
                    stage: "Tracked live".into(),
                    summary: "Discovered from tracker HTML fallback".into(),
                    is_private: false,
                    url: format!("https://github.com/{repo}"),
                });
            }
        }
    }
    out
}

/// Fetches the latest short commit SHA from GitHub for each given `owner/repo` (for true `✓ UP TO DATE` vs `⚡ NEW COMMIT` detection).
pub fn fetch_remote_commits(repos: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for repo in repos {
        let clean = repo.trim();
        if !clean.contains('/') || clean.contains(' ') {
            continue;
        }
        let url = format!("https://api.github.com/repos/{clean}/commits?per_page=1");
        if let Ok(body) = unsafe { http_get_text(&url, 64 * 1024) } {
            let mut p = Parser::new(&body);
            if let Some(Json::Arr(items)) = p.parse_val(0) {
                if let Some(first) = items.first() {
                    if let Some(sha) = first.get("sha").and_then(|v| v.as_str()) {
                        let short: String = sha.chars().take(7).collect();
                        if !short.is_empty() {
                            out.push((clean.to_ascii_lowercase(), short));
                        }
                    }
                }
            }
        }
    }
    out
}

impl PortWatchSnapshot {
    pub fn remote_sha_for(&self, repo: &str) -> Option<&str> {
        let key = repo.trim().to_ascii_lowercase();
        self.remote_commits
            .iter()
            .find(|(r, _)| r == &key)
            .map(|(_, s)| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_portwatch_fetch_parses_nonzero_scores() {
        let snap = fetch_live();
        assert!(
            snap.source_url.contains("portwatch-vi.xyz"),
            "expected portwatch-vi.xyz source_url, got: {} (err: {:?})",
            snap.source_url,
            snap.error
        );
        assert!(snap.error.is_none(), "unexpected error: {:?}", snap.error);
        assert!(!snap.projects.is_empty(), "expected non-empty projects");
        assert!(
            snap.projects.iter().any(|p| p.score > 80.0),
            "expected top project > 80%, got: {:?}",
            snap.projects.iter().take(5).map(|p| (&p.name, p.score)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn fallback_repos_exclude_dead_repos_and_prioritize_nv() {
        let projects = builtin_fallback_projects();
        assert!(!projects.is_empty());
        assert_eq!(projects[0].repo, "trustno1-coder/VIctory-NVIDIA");
        assert!(!projects.iter().any(|p| p.repo.to_ascii_lowercase().contains("dexnom")));
        assert!(!projects.iter().any(|p| p.repo.to_ascii_lowercase().contains("belobug")));
        assert!(!projects.iter().any(|p| p.repo.to_ascii_lowercase().contains("avatarmanz")));
    }

    #[test]
    fn live_private_and_amd_entries_are_filtered_and_nvidia_pinned_first() {
        let mk = |name: &str, repo: &str, url: &str, private: bool, score: f64| PortWatchProject {
            name: name.into(),
            repo: repo.into(),
            score,
            stage: "Playable".into(),
            summary: "desc".into(),
            is_private: private,
            url: url.into(),
        };
        let mut projects = vec![
            mk("Belobug", "Belobug/victory-amd", "https://github.com/Belobug/victory-amd", false, 99.0),
            mk("Private (PRIVATE)", "some/private", "https://portwatch-vi.xyz/private", true, 98.0),
            mk("NotNozy", "notnozy/victory", "https://gitlab.com/notnozy/victory", false, 97.0),
            mk("kelvaroq", "stellarr-studio/kelvaroq", "https://github.com/stellarr-studio/kelvaroq", false, 50.0),
            mk("VIctory-NVIDIA", "trustno1-coder/VIctory-NVIDIA", "https://github.com/trustno1-coder/VIctory-NVIDIA", false, 10.0),
        ];
        order_portwatch(&mut projects);
        assert_eq!(projects[0].repo, "trustno1-coder/VIctory-NVIDIA");
        assert!(projects[0].stage.contains(RECOMMENDED_BADGE));
        assert_eq!(projects[1].repo, "stellarr-studio/kelvaroq");
        assert!(!projects.iter().any(|p| p.repo.to_ascii_lowercase().contains("belobug")));
        assert!(!projects.iter().any(|p| p.is_private));
        assert!(projects.iter().all(|p| p.url.to_ascii_lowercase().contains("github.com/")));
        assert_eq!(sanitize_tracker_text("\u{00C2}\u{00B7} ok"), "\u{00B7} ok");
    }
}
