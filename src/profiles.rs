//! Portable Community Profile Export / Import (`.gmprofile`) for v1.0
//! Allows sharing hardware-tuned presets, Advanced flags, HUD mode, and keymaps.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Settings;
use crate::win32::local_timestamp_filename;

#[allow(dead_code)]
pub fn export_profile(s: &Settings, name: &str) -> Result<PathBuf, String> {
    let dir = Settings::profiles_dir();
    let _ = fs::create_dir_all(&dir);
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let stem = if safe.trim_matches('-').is_empty() {
        format!("profile-{}", local_timestamp_filename())
    } else {
        safe.trim_matches('-').to_string()
    };
    let dest = dir.join(format!("{stem}.gmprofile"));
    let body = format!(
        "# Game Master 6 v1.0 Portable Profile\n\
         active_preset={}\n\
         console={}\n\
         display_mode={}\n\
         borderless={}\n\
         card_buffers_min_mb={}\n\
         card_buffers_mb={}\n\
         texture_room_percent={}\n\
         card_room_for_targets_mb={}\n\
         texture_budget_mb={}\n\
         texture_stage_mb={}\n\
         relay_mode={}\n\
         relay_copy={}\n\
         card_steps={}\n\
         thin_lists={}\n\
         upload_ahead={}\n\
         levels_on_demand={}\n\
         texture_no_wait={}\n\
         async_submit={}\n\
         wide_back={}\n\
         wide_refresh={}\n\
         wide_merge={}\n\
         card_rehash_runs={}\n\
         join_copies={}\n\
         join_parts={}\n\
         indirect_window={}\n\
         indirect_batch={}\n\
         map_threads={}\n\
         card_awake={}\n\
         show_after={}\n\
         fast_cores={}\n\
         ambient_fill={}\n\
         ambient_val={}\n\
         ambient_air={}\n\
         bvh_builder={}\n\
         hud_mode={}\n\
         keymap_preset={}\n\
         keymap={}\n\
         custom_env={}\n",
        s.active_preset.trim(),
        s.console_model.trim(),
        s.display_mode.trim(),
        if s.borderless { "1" } else { "0" },
        s.card_buffers_min_mb.trim(),
        s.card_buffers_mb.trim(),
        s.texture_room_percent.trim(),
        s.card_room_for_targets_mb.trim(),
        s.texture_budget_mb.trim(),
        s.texture_stage_mb.trim(),
        s.relay_mode,
        if s.relay_copy { "1" } else { "0" },
        s.card_steps,
        if s.thin_lists { "1" } else { "0" },
        if s.upload_ahead { "1" } else { "0" },
        if s.levels_on_demand { "1" } else { "0" },
        if s.texture_no_wait { "1" } else { "0" },
        if s.async_submit { "1" } else { "0" },
        if s.wide_back { "1" } else { "0" },
        if s.wide_refresh { "1" } else { "0" },
        if s.wide_merge { "1" } else { "0" },
        if s.card_rehash_runs { "1" } else { "0" },
        if s.join_copies { "1" } else { "0" },
        s.join_parts,
        if s.indirect_window { "1" } else { "0" },
        s.indirect_batch,
        if s.map_threads { "1" } else { "0" },
        if s.card_awake { "1" } else { "0" },
        if s.show_after { "1" } else { "0" },
        if s.fast_cores { "1" } else { "0" },
        if s.ambient_fill { "1" } else { "0" },
        s.ambient_val.trim(),
        s.ambient_air.trim(),
        if s.bvh_builder { "1" } else { "0" },
        s.hud_mode,
        s.keymap_preset.trim(),
        s.keymap.trim(),
        s.custom_env.trim(),
    );
    fs::write(&dest, body).map_err(|e| format!("Failed to export profile: {e}"))?;
    Ok(dest)
}

#[allow(dead_code)]
pub fn import_profile(s: &mut Settings, path: &Path) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("Cannot read profile: {e}"))?;
    let mut applied = 0usize;
    let pb = |v: &str| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim();
            match k.trim() {
                "active_preset" => s.active_preset = v.to_ascii_lowercase(),
                "console" => s.console_model = if v.eq_ignore_ascii_case("series-s") { "series-s".into() } else { "series-x".into() },
                "display_mode" => s.display_mode = v.to_string(),
                "borderless" => s.borderless = pb(v),
                "card_buffers_min_mb" => s.card_buffers_min_mb = v.to_string(),
                "card_buffers_mb" => s.card_buffers_mb = v.to_string(),
                "texture_room_percent" => s.texture_room_percent = v.to_string(),
                "card_room_for_targets_mb" => s.card_room_for_targets_mb = v.to_string(),
                "texture_budget_mb" => s.texture_budget_mb = v.to_string(),
                "texture_stage_mb" => s.texture_stage_mb = v.to_string(),
                "relay_mode" => s.relay_mode = v.parse::<u8>().unwrap_or(2).min(2),
                "relay_copy" => s.relay_copy = pb(v),
                "card_steps" => s.card_steps = v.parse::<u8>().unwrap_or(2).min(2),
                "thin_lists" => s.thin_lists = pb(v),
                "upload_ahead" => s.upload_ahead = pb(v),
                "levels_on_demand" => s.levels_on_demand = pb(v),
                "texture_no_wait" => s.texture_no_wait = pb(v),
                "async_submit" => s.async_submit = pb(v),
                "wide_back" => s.wide_back = pb(v),
                "wide_refresh" => s.wide_refresh = pb(v),
                "wide_merge" => s.wide_merge = pb(v),
                "card_rehash_runs" => s.card_rehash_runs = pb(v),
                "join_copies" => s.join_copies = pb(v),
                "join_parts" => s.join_parts = v.parse::<u8>().unwrap_or(8).clamp(1, 32),
                "indirect_window" => s.indirect_window = pb(v),
                "indirect_batch" => s.indirect_batch = v.parse::<u8>().unwrap_or(2).clamp(1, 4),
                "map_threads" => s.map_threads = pb(v),
                "card_awake" => s.card_awake = pb(v),
                "show_after" => s.show_after = pb(v),
                "fast_cores" => s.fast_cores = pb(v),
                "ambient_fill" => s.ambient_fill = pb(v),
                "ambient_val" => s.ambient_val = v.to_string(),
                "ambient_air" => s.ambient_air = v.to_string(),
                "bvh_builder" => s.bvh_builder = pb(v),
                "hud_mode" => s.hud_mode = v.parse::<u8>().unwrap_or(1) % 3,
                "keymap_preset" => s.keymap_preset = v.to_string(),
                "keymap" => s.keymap = v.to_string(),
                "custom_env" => s.custom_env = v.to_string(),
                _ => continue,
            }
            applied += 1;
        }
    }
    Ok(applied)
}
