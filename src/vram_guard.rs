//! Native Zero-Lag Background VRAM Vampire Scanner (v15.2)
//! Uses pure Win32 Toolhelp32 snapshot (<2ms, zero PowerShell, strictly read-only)
//! to detect active GPU-accelerated desktop apps eating dedicated VRAM before launch.

use crate::win32::{CloseHandle, BOOL, DWORD, HANDLE};

const TH32CS_SNAPPROCESS: DWORD = 0x0000_0002;
const INVALID_HANDLE_VALUE: HANDLE = (-1isize) as HANDLE;

#[repr(C)]
struct ProcessEntry32W {
    dw_size: DWORD,
    cnt_usage: DWORD,
    th32_process_id: DWORD,
    th32_default_heap_id: usize,
    th32_module_id: DWORD,
    cnt_threads: DWORD,
    th32_parent_process_id: DWORD,
    pc_pri_class_base: i32,
    dw_flags: DWORD,
    sz_exe_file: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(dw_flags: DWORD, th32_process_id: DWORD) -> HANDLE;
    fn Process32FirstW(h_snapshot: HANDLE, lppe: *mut ProcessEntry32W) -> BOOL;
    fn Process32NextW(h_snapshot: HANDLE, lppe: *mut ProcessEntry32W) -> BOOL;
}

#[derive(Clone, Debug, Default)]
pub struct VramGuardReport {
    pub active_apps: Vec<String>,
    pub est_vram_tax_mb: u32,
}

impl VramGuardReport {
    pub fn summary_line(&self) -> Option<String> {
        if self.active_apps.is_empty() {
            return None;
        }
        let preview: Vec<&str> = self.active_apps.iter().take(3).map(|s| s.as_str()).collect();
        let more = if self.active_apps.len() > 3 {
            format!(" +{}", self.active_apps.len() - 3)
        } else {
            String::new()
        };
        Some(format!(
            "~{} MB VRAM tax from {}{} — close before playing for extra texture room",
            self.est_vram_tax_mb,
            preview.join(", "),
            more
        ))
    }
}

/// Scans running processes natively in <2ms and identifies known GPU/VRAM-consuming apps.
/// Strictly read-only — never kills or modifies any user process.
pub fn scan_vram_consumers() -> VramGuardReport {
    let known: &[(&str, &str, u32)] = &[
        ("nvidia overlay", "NVIDIA Overlay", 260),
        ("aico parallax", "AICO Parallax Wallpaper", 320),
        ("wallpaper32", "Wallpaper Engine", 380),
        ("wallpaper64", "Wallpaper Engine", 380),
        ("obs64", "OBS Studio", 420),
        ("discord", "Discord (HW Accel)", 180),
        ("brave", "Brave Browser", 260),
        ("chrome", "Chrome Browser", 280),
        ("msedge", "Edge Browser", 220),
        ("firefox", "Firefox Browser", 220),
        ("predatorsense", "PredatorSense", 90),
    ];

    let mut found: Vec<(&'static str, u32)> = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap == INVALID_HANDLE_VALUE {
            return VramGuardReport::default();
        }
        let mut pe: ProcessEntry32W = std::mem::zeroed();
        pe.dw_size = std::mem::size_of::<ProcessEntry32W>() as DWORD;
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                let len = pe.sz_exe_file.iter().position(|&c| c == 0).unwrap_or(pe.sz_exe_file.len());
                let exe = String::from_utf16_lossy(&pe.sz_exe_file[..len]).to_ascii_lowercase();
                for &(needle, label, est_mb) in known {
                    if exe.contains(needle) && !found.iter().any(|(l, _)| *l == label) {
                        found.push((label, est_mb));
                    }
                }
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }

    let est_vram_tax_mb = found.iter().map(|(_, mb)| *mb).sum();
    let active_apps = found.into_iter().map(|(l, _)| l.to_string()).collect();
    VramGuardReport {
        active_apps,
        est_vram_tax_mb,
    }
}
