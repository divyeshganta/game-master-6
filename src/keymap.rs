//! Universal-Standard PC Open-World Keybinding & Remapping Engine (v1.2)
//! Supports Standard PC Standard, FPS Standard, Left-Handed (IJKL), and Pad-Only layouts,
//! with context-aware conflict detection and clean `Settings::keymap` serialization.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionContext {
    OnFoot,
    Combat,
    Vehicle,
    Global,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeymapPreset {
    PcStandard,
    FpsStandard,
    LeftHanded,
    PadOnly,
}

impl KeymapPreset {
    pub fn id(self) -> &'static str {
        match self {
            KeymapPreset::PcStandard => "pc-std",
            KeymapPreset::FpsStandard => "fps",
            KeymapPreset::LeftHanded => "left-handed",
            KeymapPreset::PadOnly => "pad-only",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            KeymapPreset::PcStandard => "Standard PC",
            KeymapPreset::FpsStandard => "FPS / SHOOTER",
            KeymapPreset::LeftHanded => "LEFT-HANDED (IJKL)",
            KeymapPreset::PadOnly => "CONTROLLER ONLY",
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            KeymapPreset::PcStandard => KeymapPreset::FpsStandard,
            KeymapPreset::FpsStandard => KeymapPreset::LeftHanded,
            KeymapPreset::LeftHanded => KeymapPreset::PadOnly,
            KeymapPreset::PadOnly => KeymapPreset::PcStandard,
        }
    }

    pub fn from_id(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "fps" => KeymapPreset::FpsStandard,
            "left-handed" | "ijkl" => KeymapPreset::LeftHanded,
            "pad-only" | "pad" => KeymapPreset::PadOnly,
            _ => KeymapPreset::PcStandard,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActionBinding {
    pub id: &'static str,
    pub label: &'static str,
    pub context: ActionContext,
    pub key: String,
}

#[derive(Clone, Debug)]
pub struct KeyConflict {
    pub key: String,
    pub action_a: String,
    pub action_b: String,
}

/// Returns the canonical universal-standard action list for a given preset.
pub fn preset_bindings(preset: KeymapPreset) -> Vec<ActionBinding> {
    let mk = |id: &'static str, label: &'static str, context: ActionContext, key: &str| ActionBinding {
        id,
        label,
        context,
        key: key.to_string(),
    };
    match preset {
        KeymapPreset::PcStandard => vec![
            mk("move_fwd", "Move Forward", ActionContext::OnFoot, "W"),
            mk("move_back", "Move Back", ActionContext::OnFoot, "S"),
            mk("move_left", "Move Left", ActionContext::OnFoot, "A"),
            mk("move_right", "Move Right", ActionContext::OnFoot, "D"),
            mk("sprint", "Sprint", ActionContext::OnFoot, "LShift"),
            mk("crouch", "Crouch / Stealth", ActionContext::OnFoot, "LCtrl"),
            mk("jump", "Jump / Climb", ActionContext::OnFoot, "Space"),
            mk("camera", "Change Camera", ActionContext::Global, "V"),
            mk("look_back", "Look Behind", ActionContext::Global, "C"),
            mk("fire", "Fire / Attack", ActionContext::Combat, "LMB"),
            mk("aim", "Aim / Lock-On", ActionContext::Combat, "RMB"),
            mk("reload", "Reload", ActionContext::Combat, "R"),
            mk("cover", "Take Cover", ActionContext::Combat, "Q"),
            mk("weapon_wheel", "Weapon Wheel", ActionContext::Combat, "Tab"),
            mk("enter_veh", "Enter / Exit Vehicle", ActionContext::Vehicle, "F"),
            mk("handbrake", "Handbrake", ActionContext::Vehicle, "Space"),
            mk("interact", "Interact / Context", ActionContext::Global, "E"),
            mk("pause_map", "Pause / Map", ActionContext::Global, "M"),
        ],
        KeymapPreset::FpsStandard => vec![
            mk("move_fwd", "Move Forward", ActionContext::OnFoot, "W"),
            mk("move_back", "Move Back", ActionContext::OnFoot, "S"),
            mk("move_left", "Move Left", ActionContext::OnFoot, "A"),
            mk("move_right", "Move Right", ActionContext::OnFoot, "D"),
            mk("sprint", "Sprint", ActionContext::OnFoot, "LShift"),
            mk("crouch", "Crouch / Stealth", ActionContext::OnFoot, "C"),
            mk("jump", "Jump / Climb", ActionContext::OnFoot, "Space"),
            mk("camera", "Change Camera", ActionContext::Global, "V"),
            mk("look_back", "Look Behind", ActionContext::Global, "B"),
            mk("fire", "Fire / Attack", ActionContext::Combat, "LMB"),
            mk("aim", "Aim / Lock-On", ActionContext::Combat, "RMB"),
            mk("reload", "Reload", ActionContext::Combat, "R"),
            mk("cover", "Take Cover", ActionContext::Combat, "LCtrl"),
            mk("weapon_wheel", "Weapon Wheel", ActionContext::Combat, "Tab"),
            mk("enter_veh", "Enter / Exit Vehicle", ActionContext::Vehicle, "F"),
            mk("handbrake", "Handbrake", ActionContext::Vehicle, "Space"),
            mk("interact", "Interact / Context", ActionContext::Global, "E"),
            mk("pause_map", "Pause / Map", ActionContext::Global, "M"),
        ],
        KeymapPreset::LeftHanded => vec![
            mk("move_fwd", "Move Forward", ActionContext::OnFoot, "I"),
            mk("move_back", "Move Back", ActionContext::OnFoot, "K"),
            mk("move_left", "Move Left", ActionContext::OnFoot, "J"),
            mk("move_right", "Move Right", ActionContext::OnFoot, "L"),
            mk("sprint", "Sprint", ActionContext::OnFoot, "RShift"),
            mk("crouch", "Crouch / Stealth", ActionContext::OnFoot, "RCtrl"),
            mk("jump", "Jump / Climb", ActionContext::OnFoot, "Space"),
            mk("camera", "Change Camera", ActionContext::Global, "N"),
            mk("look_back", "Look Behind", ActionContext::Global, "B"),
            mk("fire", "Fire / Attack", ActionContext::Combat, "LMB"),
            mk("aim", "Aim / Lock-On", ActionContext::Combat, "RMB"),
            mk("reload", "Reload", ActionContext::Combat, "U"),
            mk("cover", "Take Cover", ActionContext::Combat, "O"),
            mk("weapon_wheel", "Weapon Wheel", ActionContext::Combat, "Backspace"),
            mk("enter_veh", "Enter / Exit Vehicle", ActionContext::Vehicle, "H"),
            mk("handbrake", "Handbrake", ActionContext::Vehicle, "Space"),
            mk("interact", "Interact / Context", ActionContext::Global, "P"),
            mk("pause_map", "Pause / Map", ActionContext::Global, "M"),
        ],
        KeymapPreset::PadOnly => vec![],
    }
}

/// Serializes bindings into a compact semicolon-separated `action=key` string.
pub fn serialize_bindings(bindings: &[ActionBinding]) -> String {
    bindings
        .iter()
        .map(|b| format!("{}={}", b.id, b.key.trim()))
        .collect::<Vec<_>>()
        .join(";")
}

/// Detects genuine key conflicts within the same context (or against Global actions).
/// Context-safe shared keys (e.g. `Space` for Jump on foot + Handbrake in vehicle) are allowed.
#[allow(dead_code)]
pub fn find_conflicts(bindings: &[ActionBinding]) -> Vec<KeyConflict> {
    let mut conflicts = Vec::new();
    for i in 0..bindings.len() {
        let a = &bindings[i];
        let ka = a.key.trim();
        if ka.is_empty() || ka.eq_ignore_ascii_case("none") {
            continue;
        }
        for b in bindings.iter().skip(i + 1) {
            if !ka.eq_ignore_ascii_case(b.key.trim()) {
                continue;
            }
            let same_or_global = a.context == b.context
                || a.context == ActionContext::Global
                || b.context == ActionContext::Global;
            if same_or_global {
                conflicts.push(KeyConflict {
                    key: ka.to_string(),
                    action_a: a.label.to_string(),
                    action_b: b.label.to_string(),
                });
            }
        }
    }
    conflicts
}
