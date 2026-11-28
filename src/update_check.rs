//! Non-blocking 1-Click Update Check.
//!
//! Background check on startup (never freezes UI, suspended while the game runs) against BOTH:
//! - GameMaster6 GitHub Releases (this app)
//! - `trustno1-coder/VIctory-NVIDIA` latest upstream commit vs local runner commit
//!
//! One-click runner update backs up saves first, backs up `victory.exe` -> `victory.exe.bak`,
//! updates/rebuilds, and warms shaders while keeping all user saves and settings intact.

use std::sync::mpsc::{channel, Receiver};
use std::thread;

pub const VICTORY_NVIDIA_REPO: &str = "trustno1-coder/VIctory-NVIDIA";
/// Known upstream baseline commit tracked by this build (Phase 1).
pub const VICTORY_NVIDIA_BASELINE_COMMIT: &str = "f3534e6b09";

#[derive(Clone, Debug, Default)]
pub struct UpdateStatus {
    pub app_update_available: bool,
    pub app_latest_version: String,
    pub app_current_version: String,
    pub runner_upstream_commit: String,
    pub runner_local_commit: String,
    pub runner_update_available: bool,
    pub checked_offline: bool,
    pub error: Option<String>,
}

impl UpdateStatus {
    pub fn update_available_pill(&self) -> bool {
        self.app_update_available || self.runner_update_available
    }
}

/// App releases repo (GameMaster6 GitHub Releases). Read from env to stay generic.
pub fn self_releases_repo() -> String {
    std::env::var("GM_SELF_RELEASES_REPO")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gamemaster/game-master-6".to_string())
}

/// Spawns a NON-BLOCKING background thread. Caller must NOT call while game is running (suspended).
/// Returns a Receiver; UI polls `try_recv` without blocking.
pub fn check_updates_nonblocking(
    current_version: String,
    runner_local_commit: String,
    offline_mode: bool,
) -> Receiver<UpdateStatus> {
    let (tx, rx) = channel();
    if offline_mode {
        let _ = tx.send(UpdateStatus {
            app_current_version: current_version,
            checked_offline: true,
            ..Default::default()
        });
        return rx;
    }
    thread::spawn(move || {
        let mut st = UpdateStatus {
            app_current_version: current_version.clone(),
            runner_local_commit: runner_local_commit.clone(),
            ..Default::default()
        };

        // 1) App update: GitHub Releases "latest" tag.
        let repo = self_releases_repo();
        let rel_url = format!("https://api.github.com/repos/{repo}/releases/latest");
        match http_get_text(&rel_url, 32 * 1024) {
            Ok(body) => {
                if let Some(tag) = json_str_field(&body, "tag_name") {
                    let latest = tag.trim_start_matches('v').to_string();
                    st.app_latest_version = latest.clone();
                    st.app_update_available = version_gt(&latest, &current_version);
                }
            }
            Err(e) => st.error = Some(e),
        }

        // 2) Runner update: latest upstream commit vs local.
        if let Some(sha) = crate::portwatch::fetch_remote_commits(&[VICTORY_NVIDIA_REPO.to_string()])
            .into_iter()
            .next()
            .map(|(_, s)| s)
        {
            st.runner_upstream_commit = sha.clone();
            st.runner_update_available = !sha.is_empty()
                && !runner_local_commit.is_empty()
                && !sha.eq_ignore_ascii_case(&runner_local_commit)
                && !runner_local_commit.to_ascii_lowercase().starts_with(&sha.to_ascii_lowercase());
        }

        let _ = tx.send(st);
    });
    rx
}

fn version_gt(a: &str, b: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.split(|c| c == '.' || c == '-').filter_map(|p| p.trim_start_matches('v').parse::<u64>().ok()).collect()
    }
    let pa = parts(a);
    let pb = parts(b);
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

fn json_str_field(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = body.find(&needle)?;
    let rest = &body[idx + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(target_os = "windows")]
fn http_get_text(url: &str, max_bytes: usize) -> Result<String, String> {
    unsafe { crate::win32::http_get_text(url, max_bytes) }
}

#[cfg(not(target_os = "windows"))]
fn http_get_text(_url: &str, _max_bytes: usize) -> Result<String, String> {
    Err("offline".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_gt_compares_semver() {
        assert!(version_gt("1.6.0", "1.5.1"));
        assert!(version_gt("1.6.0", "1.5.0"));
        assert!(!version_gt("1.6.0", "1.6.0"));
        assert!(!version_gt("1.5.0", "1.6.0"));
    }

    #[test]
    fn offline_check_is_nonblocking_and_immediate() {
        let rx = check_updates_nonblocking("1.6.0".into(), "f3534e6b09".into(), true);
        let st = rx.recv().unwrap();
        assert!(st.checked_offline);
        assert!(!st.update_available_pill());
    }

    #[test]
    fn victory_nvidia_repo_is_primary() {
        assert_eq!(VICTORY_NVIDIA_REPO, "trustno1-coder/VIctory-NVIDIA");
        assert_eq!(VICTORY_NVIDIA_BASELINE_COMMIT, "f3534e6b09");
    }

    #[test]
    fn json_str_field_extracts_tag() {
        assert_eq!(json_str_field(r#"{"tag_name":"v1.6.0"}"#, "tag_name").as_deref(), Some("v1.6.0"));
    }
}
