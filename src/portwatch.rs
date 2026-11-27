use std::fs;
use std::path::PathBuf;

use crate::config::Settings;
use crate::win32::{http_get_text, local_timestamp_human};

pub const PORTWATCH_HOME: &str = "https://portwatch-vi.xyz/";

const DEFAULT_TRACKERS: &str = "# Game Master 6 v1.0 — Live Tracker Failover List\n\
# One URL per line. Checked top-to-bottom until one responds.\n\
# If all tracker websites go offline, Game Master 6 falls back to\n\
# direct GitHub repo checks and its built-in offline registry.\n\n\
https://portwatch-vi.xyz/data/tracker.json\n\
https://4lwayz.github.io/portwatch-vi/data/tracker.json\n\
https://portwatch-vi.xyz/\n\
https://doesleonidaboot.com/race\n";

/// Direct GitHub fallback list used if ALL community tracker sites shut down (Nov 26, 2026 standings).
const FALLBACK_REPOS: [(&str, &str, f64, &str, &str); 11] = [
    ("Belobug", "Belobug/VIctory-Belobug-AMD", 91.3, "Playable (AMD)", "Belobug's AMD-focused VIctory fork (91.3% — AMD)"),
    ("VIctory-NVIDIA", "trustno1-coder/VIctory-NVIDIA", 90.0, "Playable (NV)", "NVIDIA DLSS Super Resolution + 2x-6x Frame Generation fork (90.0% — NVIDIA)"),
    ("CoolFox", "DoubleGhosty/cool_fox", 85.0, "Playable (Baseline)", "DoubleGhosty mirror of cool_fox baseline"),
    ("VIctory", "Marvisak/VIctory", 50.3, "Playable (Upstream)", "Upstream Rust Xbox Series compatibility runtime"),
    ("AvatarManz", "avatarmanz/VIctory-7900XTX", 83.1, "Playable (AMD)", "AvatarManz's 7900 XTX / AMD VIctory fork"),
    ("DexNom", "DexNom/VIctory", 78.0, "Playable (AMD)", "DexNom's AMD VIctory fork with ray-traced lighting"),
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
    FALLBACK_REPOS
        .iter()
        .map(|(name, repo, score, stage, summary)| PortWatchProject {
            name: (*name).into(),
            repo: (*repo).into(),
            score: *score,
            stage: (*stage).into(),
            summary: (*summary).into(),
            is_private: false,
            url: format!("https://github.com/{repo}"),
        })
        .collect()
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
                    projects.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
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
            projects.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
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
                        stage: format!("{stage} · ★{stars} · {pushed}"),
                        summary: desc.to_string(),
                        is_private: false,
                        url: format!("https://github.com/{repo}"),
                    });
                }
            }
        }
    }
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

/// Computes the exact PortWatch 0..100 score and milestone label from `groundwork`, `ladder`, and `manualScore`.
fn compute_portwatch_score(obj: &Json) -> Option<(f64, &'static str)> {
    let ladder_key = "ladder";
    let has_model = obj.get(ladder_key).is_some()
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

    let ladder_obj = obj.get(ladder_key);
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
        let v = ladder_val(ladder_obj.and_then(|g| g.get(key)));
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
        let story = clamp01(ladder_obj.and_then(|g| g.get("story")).and_then(|v| v.as_f64()).unwrap_or(0.0));
        let render = clamp01(ladder_obj.and_then(|g| g.get("render")).and_then(|v| v.as_f64()).unwrap_or(0.0));
        let fps = ladder_obj.and_then(|g| g.get("fps")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let perf = clamp01(fps / 30.0);
        let stability = match ladder_obj.and_then(|g| g.get("stability")) {
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

    let name = obj
        .get("name")
        .or_else(|| obj.get("title"))
        .or_else(|| obj.get("id"))
        .and_then(|v| v.as_str())?
        .trim()
        .to_string();
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
    let stage = if let Some(gpu) = gpu_suffix(obj) {
        if !base_stage.contains(gpu) {
            format!("{base_stage} ({gpu})")
        } else {
            base_stage
        }
    } else {
        base_stage
    };

    let summary = obj
        .get("summary")
        .or_else(|| obj.get("description"))
        .or_else(|| obj.get("note"))
        .or_else(|| obj.get("blurb"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    let is_private = obj
        .get("private")
        .or_else(|| obj.get("is_private"))
        .and_then(|v| v.as_bool())
        .unwrap_or(!has_repo);

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
    fn parse_tracker_json_parses_nonzero_scores() {
        let sample = r#"{"efforts":[{"name":"RunnerA","repo":"org/runner-a","progress":91.5,"stage":"Playable"},{"name":"RunnerB","repo":"org/runner-b","progress":50.3,"stage":"Playable"}]}"#;
        let parsed = parse_tracker_json(sample);
        assert_eq!(parsed.len(), 2);
        assert!(parsed.iter().any(|p| p.score > 80.0));
    }
}
