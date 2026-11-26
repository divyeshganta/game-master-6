use std::ffi::{c_int, c_void};

use crate::config::Settings;
use crate::diagnostics::{summarize_gpu_timings, AssetAudit, LogEntry, ShaderCensus, SlowShader, SystemInfo};
use crate::portwatch::PortWatchSnapshot;
use crate::runners::{runners_dir, DiscoveredRunner};
use crate::monitor::ProcessStats;
use crate::saves::ModItem;
use crate::telemetry::{best_session, RunnerBenchSummary, SessionRecord};
use crate::launcher::ActiveSession;
use crate::win32::*;

// Studio Obsidian-Vice Color System (RGB -> Win32 BGR COLORREF)
pub const COL_BG_LEFT: DWORD = rgb(9, 10, 18);
pub const COL_BG_RIGHT: DWORD = rgb(16, 14, 28);
pub const COL_SIDEBAR: DWORD = rgb(12, 13, 22);
pub const COL_PANEL: DWORD = rgb(18, 20, 32);
pub const COL_PANEL_ALT: DWORD = rgb(23, 26, 42);
pub const COL_PANEL_HOVER: DWORD = rgb(31, 35, 56);
pub const COL_BORDER: DWORD = rgb(42, 46, 68);
pub const COL_BORDER_HI: DWORD = rgb(84, 94, 136);

pub const COL_PINK: DWORD = rgb(255, 42, 133);
pub const COL_ORANGE: DWORD = rgb(255, 140, 42);
pub const COL_CYAN: DWORD = rgb(0, 240, 255);
pub const COL_CYAN_BG: DWORD = rgb(14, 52, 64);
pub const COL_EMERALD: DWORD = rgb(34, 211, 138);
pub const COL_EMERALD_BG: DWORD = rgb(14, 54, 42);
pub const COL_AMBER: DWORD = rgb(255, 190, 60);
pub const COL_AMBER_BG: DWORD = rgb(62, 46, 16);
pub const COL_CORAL: DWORD = rgb(255, 77, 109);
pub const COL_CORAL_BG: DWORD = rgb(66, 20, 32);

pub const COL_WHITE: DWORD = rgb(248, 250, 252);
pub const COL_TEXT: DWORD = rgb(226, 224, 240);
pub const COL_MUTED: DWORD = rgb(152, 144, 182);
pub const COL_DIM: DWORD = rgb(108, 100, 138);

pub const COL_HERO_L: DWORD = rgb(46, 16, 72);
pub const COL_HERO_R: DWORD = rgb(72, 30, 26);
pub const COL_DOCK: DWORD = rgb(13, 14, 24);

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    None,
    Nav(usize),
    Prepare,
    LaunchOrStop,
    Rescan,
    OpenGameDir,
    OpenAppDir,
    OpenLogsDir,
    OpenCurrentLog,
    ExportReport,
    OpenSettingsFile,
    ResetDefaults,
    ToggleUpscaler,
    ToggleGpuTiming,
    ToggleCapture,
    ToggleStartMaximized,
    ToggleZeroOverhead,
    TogglePreferDgpu,
    ToggleHighPriority,
    ToggleDred,
    ToggleIsolatedCache,
    ToggleWarp,
    ToggleOfflineMode,
    OpenVirtualMem,
    FixTdrDelay,
    LinkExeToGameDir,
    ViewLogsQuick,
    TuneQuick,
    RunnersQuick,
    SelectRunner(usize),
    CycleRunnerKind,
    RescanRunners,
    OpenRunnersDir,
    OpenCacheDir,
    ClearActiveCache,
    SyncPortWatch,
    OpenTrackersFile,
    OpenPortWatchRepo(usize),
    BuildPortWatchRepo(usize),
    BackupSaves,
    OpenBackupsDir,
    ApplyPreset(usize),
    SetConsole(bool),
    SetLogLevel(usize),
    SetShaderLimit(usize),
    BrowseExe,
    BrowseGameDir,
    BrowseRunner,
    BrowseUpscalerDir,
    ClearUpscalerDir,
    SelectLog(usize),
    RollbackRunner(usize),
    DeleteRunner(usize),
    ToggleSkipShader(usize),
    ClearSkipShaders,
    ShowKeymap,
    ExportAiBundle,
    OpenSessionsCsv,
    CycleTheme,
    CleanOldLogs,
    CycleCardBuffers,
    ToggleCardRehash,
    ToggleJoinCopies,
    ToggleTimer1ms,
    CleanJunkRunners,
    SeedIsolatedCache,
    KillStrayRunners,
    AutoEnhance,
    RumblePad(u32),
    SetLogFilter(usize),
    CopyLogSlice,
    ToggleConsoleQuick,
    ToggleHudOverlay,
    CycleHudMode,
    CycleKeymapPreset,
    CycleAbExperiment,
    SafeBootRollback,
    ToggleAutoBackup,
    RestoreLatestSave,
    OpenModsDir,
    CreateDesktopShortcut,
    CycleHudCorner,
    EditCustomEnv,
    EditTitleArgs,
    ExportCacheZip,
    ToggleMod(usize),
    CaptureScreenshot,
    CycleInputMode,
    RemapKeybind(usize),
    ResetKeybinds,
    CycleRelayMode,
    ToggleWideBack,
    CyclePipelineWait,
    ToggleBvhBuilder,
    ToggleMouseAim,
    CycleMouseSens,
    ToggleMouseLock,
    RunShaderCensus,
    ToggleAsyncSubmit,
    ToggleQueue2,
    ToggleIndirectWindow,
    ToggleTextureNoWait,
    ToggleMergePasses,
    ResetV11Perf,
    ToggleDisplay1080p,
    ToggleWideRefresh,
    ToggleWideMerge,
    ToggleInplaceBuf,
    ToggleKbmCrosshair,
    ToggleProfiler,
    ToggleAmbientFill,
    ToggleHoistClamps,
    ToggleFastCores,
    ToggleNoWaitResident,
    CycleHomeRunner,
    ImportRunner,
    CycleDisplayMode,
    ToggleBorderless,
    ToggleThinLists,
    CycleQueue2Landing,
    CycleTextureRoom,
    ToggleAdvSection(usize),
    DismissCrash,
    DismissWarning,
}

pub const SHADER_PRESETS: [&str; 6] = ["AUTO", "64", "128", "256", "512", "1024"];
pub const LOG_LEVELS: [&str; 4] = ["info", "warn", "diagnostics", "trace"];
pub const TUNING_SCROLL_STEP: usize = 32;

pub fn tuning_max_scroll(client_h: c_int) -> usize {
    let r_bottom = client_h - 152;
    let view_h = (r_bottom - 166).max(300);
    // 5 sections (~1100px total when expanded); allow smooth scrolling across all sections
    1180i32.saturating_sub(view_h).max(0) as usize
}

/// Short badge for the active compatibility runner, used in status text.
pub fn runner_badge(settings: &Settings) -> &'static str {
    let r = settings.resolved_runner().to_string_lossy().to_ascii_lowercase();
    if r.contains("baseline") {
        "BASELINE"
    } else if r.contains("ztrx") {
        "ZTRX"
    } else if r.contains("diag") {
        "DIAG"
    } else {
        "VICTORY"
    }
}

pub struct Fonts {
    pub hero: HGDIOBJ,
    pub title: HGDIOBJ,
    pub heading: HGDIOBJ,
    pub card: HGDIOBJ,
    pub body: HGDIOBJ,
    pub body_bold: HGDIOBJ,
    pub small: HGDIOBJ,
    pub small_bold: HGDIOBJ,
    pub mono: HGDIOBJ,
}

impl Fonts {
    pub unsafe fn load(dir: &std::path::Path) -> Self {
        let oswald_path = wide(&dir.join("Oswald.ttf").to_string_lossy());
        let inter_path = wide(&dir.join("Inter.ttf").to_string_lossy());
        AddFontResourceExW(oswald_path.as_ptr(), FR_PRIVATE, std::ptr::null_mut());
        AddFontResourceExW(inter_path.as_ptr(), FR_PRIVATE, std::ptr::null_mut());

        Self {
            hero: make_font("Oswald", 46, 700, true),
            title: make_font("Oswald", 32, 700, true),
            heading: make_font("Oswald", 24, 700, false),
            card: make_font("Oswald", 20, 700, false),
            body: make_font("Inter", 16, 400, false),
            body_bold: make_font("Inter", 16, 600, false),
            small: make_font("Inter", 13, 400, false),
            small_bold: make_font("Inter", 13, 700, false),
            mono: make_font("Consolas", 14, 400, false),
        }
    }

    pub unsafe fn destroy(&self) {
        DeleteObject(self.hero);
        DeleteObject(self.title);
        DeleteObject(self.heading);
        DeleteObject(self.card);
        DeleteObject(self.body);
        DeleteObject(self.body_bold);
        DeleteObject(self.small);
        DeleteObject(self.small_bold);
        DeleteObject(self.mono);
    }
}

unsafe fn make_font(face: &str, height: c_int, weight: c_int, italic: bool) -> HGDIOBJ {
    let w = wide(face);
    let qual = if height >= 30 { ANTIALIASED_QUALITY } else { CLEARTYPE_QUALITY };
    CreateFontW(
        -height,
        0,
        0,
        0,
        weight,
        if italic { 1 } else { 0 },
        0,
        0,
        DEFAULT_CHARSET,
        0,
        0,
        qual,
        0,
        w.as_ptr(),
    )
}

pub struct LogoSurface {
    pub dc: HDC,
    pub bmp: HBITMAP,
    pub old: HGDIOBJ,
    pub w: i32,
    pub h: i32,
    pub has_alpha: bool,
}

impl LogoSurface {
    pub unsafe fn load(hdc_ref: HDC, path: &std::path::Path) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        if data.len() < 54 || &data[0..2] != b"BM" {
            return None;
        }
        let offset = u32::from_le_bytes(data[10..14].try_into().ok()?) as usize;
        let w = i32::from_le_bytes(data[18..22].try_into().ok()?);
        let h_raw = i32::from_le_bytes(data[22..26].try_into().ok()?);
        let bpp = u16::from_le_bytes(data[28..30].try_into().ok()?);
        let h = h_raw.abs();
        if w <= 0 || h <= 0 || (bpp != 32 && bpp != 24) {
            return None;
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmi_header.bi_size = std::mem::size_of::<BITMAPINFOHEADER>() as DWORD;
        bmi.bmi_header.bi_width = w;
        bmi.bmi_header.bi_height = h_raw;
        bmi.bmi_header.bi_planes = 1;
        bmi.bmi_header.bi_bit_count = 32;
        bmi.bmi_header.bi_compression = 0;

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let dc = CreateCompatibleDC(hdc_ref);
        let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, std::ptr::null_mut(), 0);
        if bmp.is_null() || bits_ptr.is_null() {
            DeleteDC(dc);
            return None;
        }
        let old = SelectObject(dc, bmp);
        let dst = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (w * h * 4) as usize);

        let mut has_alpha = false;
        if bpp == 32 && offset + dst.len() <= data.len() {
            dst.copy_from_slice(&data[offset..offset + dst.len()]);
            // Check if alpha channel contains transparency
            for px in dst.chunks_exact(4) {
                if px[3] < 250 {
                    has_alpha = true;
                    break;
                }
            }
            if !has_alpha {
                // Force opaque alpha if 32-bit BMP had zeroed alpha bytes
                for px in dst.chunks_exact_mut(4) {
                    px[3] = 255;
                }
            }
        } else if bpp == 24 {
            let row_stride = ((w * 3 + 3) & !3) as usize;
            for y in 0..h as usize {
                let src_row = offset + y * row_stride;
                let dst_row = y * (w as usize) * 4;
                if src_row + (w as usize) * 3 <= data.len() {
                    for x in 0..w as usize {
                        dst[dst_row + x * 4] = data[src_row + x * 3];
                        dst[dst_row + x * 4 + 1] = data[src_row + x * 3 + 1];
                        dst[dst_row + x * 4 + 2] = data[src_row + x * 3 + 2];
                        dst[dst_row + x * 4 + 3] = 255;
                    }
                }
            }
        }

        Some(Self { dc, bmp, old, w, h, has_alpha })
    }

    pub unsafe fn draw(&self, hdc: HDC, r: RECT, opacity: u8) {
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, std::ptr::null_mut());
        let bf = BLENDFUNCTION {
            blend_op: AC_SRC_OVER,
            blend_flags: 0,
            source_constant_alpha: opacity,
            alpha_format: if self.has_alpha { AC_SRC_ALPHA } else { 0 },
        };
        AlphaBlend(
            hdc,
            r.left,
            r.top,
            r.width(),
            r.height(),
            self.dc,
            0,
            0,
            self.w,
            self.h,
            bf,
        );
    }

    pub unsafe fn destroy(&self) {
        SelectObject(self.dc, self.old);
        DeleteObject(self.bmp);
        DeleteDC(self.dc);
    }
}

pub struct RenderContext<'a> {
    pub settings: &'a Settings,
    pub sys: &'a SystemInfo,
    pub audit: &'a AssetAudit,
    pub session: Option<&'a ActiveSession>,
    pub view: usize,
    pub hover: Target,
    pub status_text: &'a str,
    pub status_kind: StatusKind,
    pub data_label: &'a str,
    pub file_count_label: &'a str,
    pub anim_tick: u32,
    pub toast: Option<&'a str>,
    pub recent_logs: &'a [LogEntry],
    pub selected_log: usize,
    pub log_lines: &'a [String],
    pub log_scroll: usize,
    pub fonts: &'a Fonts,
    pub logo: Option<&'a LogoSurface>,
    pub runners: &'a [DiscoveredRunner],
    pub portwatch: &'a PortWatchSnapshot,
    pub portwatch_syncing: bool,
    pub portwatch_scroll: usize,
    pub runner_scroll: usize,
    pub tuning_scroll: usize,
    pub log_filter: usize,
    pub playtime_label: &'a str,
    pub mod_count: usize,
    pub fps_history: &'a [u32],
    pub runner_benches: &'a [RunnerBenchSummary],
    pub mods: &'a [ModItem],
    pub backup_count: usize,
    pub slow_shaders: &'a [SlowShader],
    pub ram_history: &'a [u32],
    pub commit_history: &'a [u32],
    pub recent_sessions: &'a [SessionRecord],
    pub proc_stats: Option<ProcessStats>,
    pub listening_keybind: Option<usize>,
    pub shader_census: &'a ShaderCensus,
    pub census_running: bool,
    pub last_crash: Option<&'a (String, String, std::path::PathBuf)>,
    /// Warning text the user has already dismissed (banner stays hidden until it changes).
    pub dismissed_warning: Option<&'a str>,
    pub adv_collapsed: [bool; 5],
    pub system_tip: &'a (bool, String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Ready,
    Running,
    Preparing,
    Warning,
    Error,
}

// --- Low-level GDI drawing helpers ---

unsafe fn fill_gradient(hdc: HDC, r: RECT, from: DWORD, to: DWORD, vertical: bool) {
    if r.width() <= 0 || r.height() <= 0 {
        return;
    }
    let ch = |c: DWORD| -> (u16, u16, u16) {
        (((c & 0xff) as u16) << 8, (((c >> 8) & 0xff) as u16) << 8, (((c >> 16) & 0xff) as u16) << 8)
    };
    let (r1, g1, b1) = ch(from);
    let (r2, g2, b2) = ch(to);
    let verts = [
        TRIVERTEX { x: r.left, y: r.top, red: r1, green: g1, blue: b1, alpha: 0xff00 },
        TRIVERTEX { x: r.right, y: r.bottom, red: r2, green: g2, blue: b2, alpha: 0xff00 },
    ];
    let gr = GRADIENT_RECT { upper: 0, lower: 1 };
    GradientFill(
        hdc,
        verts.as_ptr(),
        2,
        &gr,
        1,
        if vertical { GRADIENT_FILL_RECT_V } else { GRADIENT_FILL_RECT_H },
    );
}

unsafe fn fill_rect_solid(hdc: HDC, r: RECT, color: DWORD) {
    let br = CreateSolidBrush(color);
    FillRect(hdc, &r, br);
    DeleteObject(br);
}

unsafe fn rounded_rect(hdc: HDC, r: RECT, fill: DWORD, border: DWORD, radius: c_int) {
    let br = CreateSolidBrush(fill);
    let pen = CreatePen(PS_SOLID, 1, border);
    let old_b = SelectObject(hdc, br);
    let old_p = SelectObject(hdc, pen);
    RoundRect(hdc, r.left, r.top, r.right, r.bottom, radius, radius);
    SelectObject(hdc, old_b);
    SelectObject(hdc, old_p);
    DeleteObject(pen);
    DeleteObject(br);
}

unsafe fn draw_line(hdc: HDC, x1: c_int, y1: c_int, x2: c_int, y2: c_int, color: DWORD, width: c_int) {
    let pen = CreatePen(PS_SOLID, width, color);
    let old = SelectObject(hdc, pen);
    MoveToEx(hdc, x1, y1, std::ptr::null_mut());
    LineTo(hdc, x2, y2);
    SelectObject(hdc, old);
    DeleteObject(pen);
}

unsafe fn draw_circle(hdc: HDC, cx: c_int, cy: c_int, rad: c_int, fill: DWORD, border: DWORD) {
    let br = CreateSolidBrush(fill);
    let pen = CreatePen(PS_SOLID, 1, border);
    let old_b = SelectObject(hdc, br);
    let old_p = SelectObject(hdc, pen);
    Ellipse(hdc, cx - rad, cy - rad, cx + rad, cy + rad);
    SelectObject(hdc, old_b);
    SelectObject(hdc, old_p);
    DeleteObject(pen);
    DeleteObject(br);
}

unsafe fn draw_text(hdc: HDC, text: &str, mut r: RECT, font: HGDIOBJ, color: DWORD, flags: DWORD) {
    let old = SelectObject(hdc, font);
    SetTextColor(hdc, color);
    SetBkMode(hdc, TRANSPARENT);
    let w = wide(text);
    DrawTextW(hdc, w.as_ptr(), -1, &mut r, flags | DT_NOPREFIX);
    SelectObject(hdc, old);
}

unsafe fn draw_pill(hdc: HDC, r: RECT, label: &str, font: HGDIOBJ, bg: DWORD, fg: DWORD, border: DWORD) {
    rounded_rect(hdc, r, bg, border, 12);
    draw_text(hdc, label, RECT::new(r.left + 3, r.top, r.right - 3, r.bottom), font, fg, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

unsafe fn draw_button(
    hdc: HDC,
    r: RECT,
    label: &str,
    font: HGDIOBJ,
    hovered: bool,
    accent: DWORD,
    target: Target,
    hits: &mut Vec<(RECT, Target)>,
) {
    let bg = if hovered { COL_PANEL_HOVER } else { COL_PANEL_ALT };
    let bd = if hovered { accent } else { COL_BORDER };
    rounded_rect(hdc, r, bg, bd, 10);
    draw_text(hdc, label, r, font, if hovered { COL_WHITE } else { accent }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    hits.push((r, target));
}

unsafe fn draw_sparkline(hdc: HDC, r: RECT, samples: &[u32], col: DWORD) {
    if samples.len() < 2 || r.width() < 20 || r.height() < 8 {
        return;
    }
    let pen = CreatePen(PS_SOLID, 2, col);
    let old = SelectObject(hdc, pen);
    let n = samples.len();
    let step_x = r.width() as f32 / (n - 1) as f32;
    for (i, &val) in samples.iter().enumerate() {
        let px = r.left + (i as f32 * step_x) as i32;
        let py = r.bottom - ((val.min(100) as i32 * r.height()) / 100);
        if i == 0 {
            MoveToEx(hdc, px, py, std::ptr::null_mut());
        } else {
            LineTo(hdc, px, py);
        }
    }
    SelectObject(hdc, old);
    DeleteObject(pen);
}

unsafe fn draw_progress_bar(hdc: HDC, r: RECT, pct: u32, fill_col: DWORD) {
    rounded_rect(hdc, r, rgb(32, 26, 52), COL_BORDER, 6);
    let inner_w = (r.width() - 2).max(0);
    let fill_w = (inner_w as u64 * pct.min(100) as u64 / 100) as i32;
    if fill_w > 2 {
        let fr = RECT::new(r.left + 1, r.top + 1, r.left + 1 + fill_w, r.bottom - 1);
        rounded_rect(hdc, fr, fill_col, fill_col, 4);
    }
}

unsafe fn draw_nav_icon(hdc: HDC, idx: usize, cx: c_int, cy: c_int, col: DWORD) {
    match idx {
        // 0: HOME
        0 => {
            draw_line(hdc, cx - 10, cy, cx, cy - 9, col, 2);
            draw_line(hdc, cx, cy - 9, cx + 10, cy, col, 2);
            draw_line(hdc, cx - 7, cy - 1, cx - 7, cy + 9, col, 2);
            draw_line(hdc, cx + 7, cy - 1, cx + 7, cy + 9, col, 2);
            draw_line(hdc, cx - 7, cy + 9, cx + 7, cy + 9, col, 2);
            draw_line(hdc, cx - 2, cy + 9, cx - 2, cy + 3, col, 2);
            draw_line(hdc, cx + 2, cy + 9, cx + 2, cy + 3, col, 2);
            draw_line(hdc, cx - 2, cy + 3, cx + 2, cy + 3, col, 2);
        }
        // 1: RUNNERS (fork / engine branch icon)
        1 => {
            draw_circle(hdc, cx - 6, cy - 6, 3, col, col);
            draw_circle(hdc, cx + 6, cy - 6, 3, col, col);
            draw_circle(hdc, cx, cy + 7, 3, col, col);
            draw_line(hdc, cx - 6, cy - 3, cx - 6, cy + 1, col, 2);
            draw_line(hdc, cx + 6, cy - 3, cx + 6, cy + 1, col, 2);
            draw_line(hdc, cx - 6, cy + 1, cx + 6, cy + 1, col, 2);
            draw_line(hdc, cx, cy + 1, cx, cy + 5, col, 2);
        }
        // 2: LIBRARY
        2 => {
            for dx in [-8, -2, 4] {
                let r = RECT::new(cx + dx, cy - 9, cx + dx + 5, cy + 9);
                let br = CreateSolidBrush(col);
                FrameRect(hdc, &r, br);
                DeleteObject(br);
            }
            draw_line(hdc, cx - 9, cy + 10, cx + 10, cy + 10, col, 2);
        }
        // 3: ADVANCED / ENGINEERING
        3 => {
            draw_line(hdc, cx - 10, cy - 6, cx + 10, cy - 6, col, 2);
            draw_circle(hdc, cx - 3, cy - 6, 3, col, col);
            draw_line(hdc, cx - 10, cy, cx + 10, cy, col, 2);
            draw_circle(hdc, cx + 4, cy, 3, col, col);
            draw_line(hdc, cx - 10, cy + 6, cx + 10, cy + 6, col, 2);
            draw_circle(hdc, cx - 1, cy + 6, 3, col, col);
        }
        // 4: LOGS
        4 => {
            let r = RECT::new(cx - 10, cy - 8, cx + 10, cy + 9);
            let br = CreateSolidBrush(col);
            FrameRect(hdc, &r, br);
            DeleteObject(br);
            draw_line(hdc, cx - 6, cy - 3, cx - 2, cy + 1, col, 2);
            draw_line(hdc, cx - 2, cy + 1, cx - 6, cy + 5, col, 2);
            draw_line(hdc, cx, cy + 5, cx + 6, cy + 5, col, 2);
        }
        // 5: BENCH & INPUT (gamepad / trophy icon)
        5 => {
            rounded_rect(hdc, RECT::new(cx - 10, cy - 6, cx + 10, cy + 6), COL_SIDEBAR, col, 6);
            draw_line(hdc, cx - 6, cy, cx - 2, cy, col, 2);
            draw_line(hdc, cx - 4, cy - 2, cx - 4, cy + 2, col, 2);
            draw_circle(hdc, cx + 4, cy - 2, 1, col, col);
            draw_circle(hdc, cx + 6, cy + 1, 1, col, col);
        }
        // 6: SETTINGS
        _ => {
            let pen = CreatePen(PS_SOLID, 2, col);
            let old_p = SelectObject(hdc, pen);
            let old_b = SelectObject(hdc, GetStockObject(NULL_BRUSH));
            Ellipse(hdc, cx - 6, cy - 6, cx + 6, cy + 6);
            SelectObject(hdc, old_b);
            SelectObject(hdc, old_p);
            DeleteObject(pen);
            let teeth = [
                (0, -10, 0, -6),
                (0, 6, 0, 10),
                (-10, 0, -6, 0),
                (6, 0, 10, 0),
                (-7, -7, -4, -4),
                (4, 4, 7, 7),
                (7, -7, 4, -4),
                (-4, 4, -7, 7),
            ];
            for (x1, y1, x2, y2) in teeth {
                draw_line(hdc, cx + x1, cy + y1, cx + x2, cy + y2, col, 2);
            }
        }
    }
}


struct ThemePalette {
    bg_l: DWORD,
    bg_r: DWORD,
    sidebar: DWORD,
    panel: DWORD,
    border: DWORD,
    accent: DWORD,
    hero_l: DWORD,
    hero_r: DWORD,
}

fn theme_palette(name: &str) -> ThemePalette {
    match name {
        "sunset" => ThemePalette {
            bg_l: rgb(24, 10, 18),
            bg_r: rgb(38, 18, 12),
            sidebar: rgb(26, 12, 20),
            panel: rgb(36, 18, 28),
            border: rgb(78, 42, 54),
            accent: COL_ORANGE,
            hero_l: rgb(92, 24, 42),
            hero_r: rgb(104, 56, 16),
        },
        "carbon" => ThemePalette {
            bg_l: rgb(11, 14, 20),
            bg_r: rgb(18, 22, 30),
            sidebar: rgb(14, 18, 26),
            panel: rgb(21, 26, 36),
            border: rgb(48, 58, 76),
            accent: COL_CYAN,
            hero_l: rgb(18, 36, 56),
            hero_r: rgb(22, 48, 52),
        },
        "midnight" => ThemePalette {
            bg_l: rgb(8, 16, 28),
            bg_r: rgb(14, 30, 44),
            sidebar: rgb(10, 20, 34),
            panel: rgb(16, 30, 48),
            border: rgb(36, 72, 98),
            accent: COL_EMERALD,
            hero_l: rgb(10, 44, 68),
            hero_r: rgb(16, 74, 86),
        },
        _ => ThemePalette {
            bg_l: COL_BG_LEFT,
            bg_r: COL_BG_RIGHT,
            sidebar: COL_SIDEBAR,
            panel: COL_PANEL,
            border: COL_BORDER,
            accent: COL_CYAN,
            hero_l: COL_HERO_L,
            hero_r: COL_HERO_R,
        },
    }
}

pub unsafe fn paint(hdc: HDC, w: c_int, h: c_int, ctx: &RenderContext) -> Vec<(RECT, Target)> {
    let mut hits = Vec::with_capacity(64);
    let pal = theme_palette(&ctx.settings.theme);

    // 1. Theme-aware Background gradient
    fill_gradient(hdc, RECT::new(0, 0, w, h), pal.bg_l, pal.bg_r, false);

    // 2. Sidebar (left rail)
    let sb_w = 96;
    let sb_rect = RECT::new(14, 14, 14 + sb_w, h - 14);
    rounded_rect(hdc, sb_rect, pal.sidebar, pal.border, 18);

    if let Some(logo) = ctx.logo {
        logo.draw(hdc, RECT::new(28, 22, 96, 90), 255);
    }

    let nav_labels = ["PLAY", "RUNNERS", "TOOLS", "ADVANCED", "LOGS", "INPUT", "SETTINGS"];
    for (i, label) in nav_labels.iter().enumerate() {
        let top = 98 + i as c_int * 58;
        let r = RECT::new(24, top, 100, top + 52);
        let active = ctx.view == i;
        let hovered = ctx.hover == Target::Nav(i);
        let fill = if active {
            COL_CYAN_BG
        } else if hovered {
            COL_PANEL_HOVER
        } else {
            pal.panel
        };
        let border = if active {
            pal.accent
        } else if hovered {
            COL_BORDER_HI
        } else {
            pal.border
        };
        rounded_rect(hdc, r, fill, border, 12);
        if active {
            rounded_rect(hdc, RECT::new(15, top + 10, 20, top + 42), pal.accent, pal.accent, 3);
        }
        draw_nav_icon(hdc, i, 62, top + 19, if active { pal.accent } else if hovered { COL_WHITE } else { COL_MUTED });
        draw_text(
            hdc,
            label,
            RECT::new(24, top + 33, 100, top + 49),
            ctx.fonts.small_bold,
            if active { COL_WHITE } else { COL_MUTED },
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        hits.push((r, Target::Nav(i)));
    }

    // Sidebar footer: Live Hardware Tier pill + Version badge
    let sb_tier = crate::presets::HardwareTier::from_vram_gb(ctx.sys.vram_gb);
    draw_pill(
        hdc,
        RECT::new(22, h - 68, 102, h - 46),
        sb_tier.label(),
        ctx.fonts.small_bold,
        COL_PANEL_ALT,
        pal.accent,
        COL_BORDER,
    );
    draw_text(
        hdc,
        "v1.0",
        RECT::new(20, h - 40, 104, h - 20),
        ctx.fonts.small_bold,
        COL_DIM,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    // 3. Main layout bounds
    let main_l = 126;
    let main_r = w - 16;
    let header_b = 76;
    let dock_h = if ctx.view == 0 { 0 } else { 126 };
    let dock_t = h - dock_h - 14;
    let content_rect = RECT::new(main_l, header_b + 8, main_r, dock_t - 12);

    // 4. Top Header Bar
    draw_text(
        hdc,
        "GAME MASTER",
        RECT::new(main_l, 16, main_l + 260, 50),
        ctx.fonts.title,
        COL_WHITE,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );
    let sub_max_r = if main_r - main_l > 820 { main_r - 556 } else { main_r - 336 };
    let phase_txt = if ctx.audit.shader_cache_ready {
        format!("CACHE READY · {}", runner_badge(ctx.settings))
    } else if ctx.shader_census.total > 0 {
        format!("{}/{} SHADER TRANSLATED", ctx.shader_census.translated, ctx.shader_census.total)
    } else {
        "CACHE NOT PREPARED".into()
    };
    let header_sub = format!("v1.0 · {phase_txt}");
    draw_text(
        hdc,
        &header_sub,
        RECT::new(main_l, 50, sub_max_r.max(main_l + 200), 70),
        ctx.fonts.small_bold,
        COL_CYAN,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    // Right side of header: Status Pill + Prepare Button + Commit Pill
    let stat_r = RECT::new(main_r - 148, 22, main_r, 60);
    let (s_bg, s_fg, s_bd) = match ctx.status_kind {
        StatusKind::Ready => (COL_EMERALD_BG, COL_EMERALD, COL_EMERALD),
        StatusKind::Running => (COL_CYAN_BG, COL_CYAN, COL_CYAN),
        StatusKind::Preparing => (COL_AMBER_BG, COL_AMBER, COL_AMBER),
        StatusKind::Warning => (COL_AMBER_BG, COL_AMBER, COL_AMBER),
        StatusKind::Error => (COL_CORAL_BG, COL_CORAL, COL_CORAL),
    };
    if ctx.view == 0 {
        let pf = crate::preflight::evaluate(ctx.settings, ctx.sys, ctx.audit);
        let (pf_bg, pf_fg, pf_bd) = match pf.grade {
            "S" | "A" => (COL_EMERALD_BG, COL_EMERALD, COL_EMERALD),
            "B" => (COL_CYAN_BG, COL_CYAN, COL_CYAN),
            _ => (COL_AMBER_BG, COL_AMBER, COL_AMBER),
        };
        let pf_r = RECT::new(main_r - 192, 22, main_r, 58);
        draw_pill(hdc, pf_r, &pf.summary, ctx.fonts.small_bold, pf_bg, pf_fg, pf_bd);
    }
    if ctx.view != 0 {
        draw_pill(hdc, stat_r, ctx.status_text, ctx.fonts.small_bold, s_bg, s_fg, s_bd);

    let prep_r = RECT::new(stat_r.left - 168, 22, stat_r.left - 12, 60);
    let cache_ready = ctx.audit.shader_cache_ready;
    if cache_ready {
        let hov = ctx.hover == Target::Prepare;
        rounded_rect(
            hdc,
            prep_r,
            if hov { rgb(18, 72, 54) } else { COL_EMERALD_BG },
            COL_EMERALD,
            10,
        );
        draw_text(
            hdc,
            "✓ CACHE PREPARED",
            prep_r,
            ctx.fonts.small_bold,
            if hov { COL_WHITE } else { COL_EMERALD },
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        hits.push((prep_r, Target::Prepare));
    } else {
        draw_button(
            hdc,
            prep_r,
            "PREPARE CACHE",
            ctx.fonts.small_bold,
            ctx.hover == Target::Prepare,
            COL_CYAN,
            Target::Prepare,
            &mut hits,
        );
    }

    if ctx.view != 0 && main_r - main_l > 820 {
        let mem_r = RECT::new(prep_r.left - 210, 22, prep_r.left - 12, 60);
        let commit_ok = ctx.sys.commit_healthy();
        let mem_lbl = format!(
            "COMMIT {:.1} GB  ·  {}C",
            ctx.sys.avail_commit_gb, ctx.sys.cpu_cores
        );
        draw_pill(
            hdc,
            mem_r,
            &mem_lbl,
            ctx.fonts.small,
            COL_PANEL,
            if commit_ok { COL_TEXT } else { COL_AMBER },
            if commit_ok { COL_BORDER } else { COL_AMBER },
        );
    }
    }

    // 5. Active View Stage
    match ctx.view {
        0 => draw_home(hdc, content_rect, ctx, &mut hits),
        1 => draw_runners(hdc, content_rect, ctx, &mut hits),
        2 => draw_library(hdc, content_rect, ctx, &mut hits),
        3 => draw_tuning(hdc, content_rect, ctx, &mut hits),
        4 => draw_logs(hdc, content_rect, ctx, &mut hits),
        5 => draw_bench_input(hdc, content_rect, ctx, &mut hits),
        _ => draw_settings(hdc, content_rect, ctx, &mut hits),
    }

    // 6. Bottom Dock (Left: Live Console/Telemetry Strip | Right: Launch Command Deck)
    if ctx.view != 0 {
        let dock_rect = RECT::new(main_l, dock_t, main_r, h - 14);
        draw_dock(hdc, dock_rect, ctx, &mut hits);
    }

    hits
}

unsafe fn draw_home(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    let hero_r = RECT::new(r.left, r.top + 10, r.right, r.top + 178);

    // ── Hero ────────────────────────────────────────────────────────────────
    rounded_rect(hdc, hero_r, COL_PANEL, COL_BORDER, 18);
    let inner = RECT::new(hero_r.left + 2, hero_r.top + 2, hero_r.right - 2, hero_r.bottom - 2);
    let pal = theme_palette(&ctx.settings.theme);
    fill_gradient(hdc, inner, pal.hero_l, pal.hero_r, false);
    fill_gradient(hdc, RECT::new(inner.left + 14, inner.top, inner.right - 14, inner.top + 3), COL_PINK, COL_CYAN, false);

    if let Some(logo) = ctx.logo {
        let side = (hero_r.height() - 24).clamp(108, 148);
        let lx = hero_r.right - side - 28;
        let ly = hero_r.top + (hero_r.height() - side) / 2;
        logo.draw(hdc, RECT::new(lx, ly, lx + side, ly + side), 245);
    }

    let tx = hero_r.left + 28;
    let tw = (hero_r.width() - 260).max(280);
    let gpu_eyebrow = if ctx.sys.gpu_name.is_empty() {
        format!("DETECTING GPU  ·  {}  ·  v1.0", ctx.settings.active_runner_kind().badge())
    } else if ctx.sys.vram_gb > 0.5 {
        format!(
            "{}  ·  {:.0} GB VRAM  ·  {}",
            ctx.sys.gpu_name.to_ascii_uppercase(),
            ctx.sys.vram_gb,
            ctx.settings.active_runner_kind().badge()
        )
    } else {
        format!("{}  ·  {}", ctx.sys.gpu_name.to_ascii_uppercase(), ctx.settings.active_runner_kind().badge())
    };
    draw_text(
        hdc,
        &gpu_eyebrow,
        RECT::new(tx, hero_r.top + 12, tx + tw, hero_r.top + 34),
        ctx.fonts.small_bold,
        COL_CYAN,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );
    // Generous rects (70px) give Oswald's descenders room so neither line clips.
    draw_text(
        hdc,
        "GAME",
        RECT::new(tx + 2, hero_r.top + 38, tx + tw + 4, hero_r.top + 108),
        ctx.fonts.hero,
        rgb(16, 6, 28),
        DT_LEFT | DT_SINGLELINE,
    );
    draw_text(
        hdc,
        "GAME",
        RECT::new(tx - 1, hero_r.top + 36, tx + tw, hero_r.top + 106),
        ctx.fonts.hero,
        COL_WHITE,
        DT_LEFT | DT_SINGLELINE,
    );
    draw_text(
        hdc,
        "MASTER 6",
        RECT::new(tx + 2, hero_r.top + 98, tx + tw + 4, hero_r.top + 168),
        ctx.fonts.hero,
        rgb(16, 6, 28),
        DT_LEFT | DT_SINGLELINE,
    );
    draw_text(
        hdc,
        "MASTER 6",
        RECT::new(tx - 1, hero_r.top + 96, tx + tw, hero_r.top + 166),
        ctx.fonts.hero,
        COL_PINK,
        DT_LEFT | DT_SINGLELINE,
    );

    // ── Live Hardware & Readiness Telemetry Strip ───────────────────────────
    let strip_y = hero_r.bottom + 14;
    let strip_h = 46;
    let s_gap = 14;
    let s_w = (r.width() - s_gap * 2) / 3;

    // 1. Input readiness card
    let pad_r = RECT::new(r.left, strip_y, r.left + s_w, strip_y + strip_h);
    let pad_ok = ctx.sys.xinput_pads > 0;
    rounded_rect(hdc, pad_r, if pad_ok { COL_EMERALD_BG } else { COL_PANEL }, if pad_ok { COL_EMERALD } else { COL_BORDER }, 12);
    let pad_lbl = if pad_ok {
        format!(
            "●  PAD #{} CONNECTED",
            ctx.sys.xinput_pad_details.iter().position(|p| p.connected).map(|i| i + 1).unwrap_or(1)
        )
    } else {
        "○  NO CONTROLLER DETECTED".to_string()
    };
    draw_text(hdc, &pad_lbl, RECT::new(pad_r.left + 12, pad_r.top + 6, pad_r.right - 12, pad_r.top + 26), ctx.fonts.small_bold, if pad_ok { COL_EMERALD } else { COL_MUTED }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, &format!("Console: {} · {}C CPU", ctx.settings.console_model.to_ascii_uppercase(), ctx.sys.cpu_cores), RECT::new(pad_r.left + 12, pad_r.top + 24, pad_r.right - 12, pad_r.bottom - 4), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // 2. Shader cache readiness card
    let cache_r = RECT::new(r.left + s_w + s_gap, strip_y, r.left + s_w * 2 + s_gap, strip_y + strip_h);
    let needs_refresh = ctx.settings.cache_needs_refresh(ctx.audit.shader_cache_ready);
    rounded_rect(hdc, cache_r, if needs_refresh { COL_AMBER_BG } else { COL_EMERALD_BG }, if needs_refresh { COL_AMBER } else { COL_EMERALD }, 12);
    let cache_lbl = if !ctx.audit.shader_cache_ready {
        "○  SHADER CACHE: NEEDS PREPARE".to_string()
    } else if needs_refresh {
        "●  SHADER CACHE: NEEDS REFRESH".to_string()
    } else {
        "●  SHADER CACHE: READY".to_string()
    };
    draw_text(hdc, &cache_lbl, RECT::new(cache_r.left + 12, cache_r.top + 6, cache_r.right - 12, cache_r.top + 26), ctx.fonts.small_bold, if needs_refresh { COL_AMBER } else { COL_EMERALD }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let cache_sub = if ctx.shader_census.total > 0 {
        format!("{}/{} shaders · {}", ctx.shader_census.translated, ctx.shader_census.total, ctx.audit.shader_cache_label)
    } else {
        ctx.audit.shader_cache_label.clone()
    };
    draw_text(hdc, &cache_sub, RECT::new(cache_r.left + 12, cache_r.top + 24, cache_r.right - 12, cache_r.bottom - 4), ctx.fonts.small, COL_TEXT, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // 3. Live Memory, Commit & Display card with utilization bar
    let mem_r = RECT::new(r.left + (s_w + s_gap) * 2, strip_y, r.right, strip_y + strip_h);
    let commit_ok = ctx.sys.commit_healthy() && !ctx.sys.pagefile_pending_reboot;
    rounded_rect(hdc, mem_r, if commit_ok { COL_PANEL } else { COL_AMBER_BG }, if commit_ok { COL_BORDER } else { COL_AMBER }, 12);
    let disp_tag = if ctx.sys.primary_res.is_empty() {
        format!("{}D", ctx.sys.display_count.max(1))
    } else {
        format!("{} ({}D)", ctx.sys.primary_res, ctx.sys.display_count.max(1))
    };
    let mem_lbl = format!("RAM {:.1}G · COMMIT {:.1}G · {}", ctx.sys.avail_ram_gb, ctx.sys.avail_commit_gb, disp_tag);
    draw_text(hdc, &mem_lbl, RECT::new(mem_r.left + 12, mem_r.top + 5, mem_r.right - 12, mem_r.top + 25), ctx.fonts.small_bold, if commit_ok { COL_WHITE } else { COL_AMBER }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let commit_pct = if ctx.sys.total_commit_gb > 0.1 {
        (((ctx.sys.total_commit_gb - ctx.sys.avail_commit_gb) / ctx.sys.total_commit_gb) * 100.0).clamp(2.0, 100.0) as u32
    } else {
        0
    };
    let bar_col = if commit_pct >= 88 { COL_CORAL } else if commit_pct >= 72 { COL_AMBER } else { COL_CYAN };
    draw_progress_bar(hdc, RECT::new(mem_r.left + 12, mem_r.bottom - 13, mem_r.right - 12, mem_r.bottom - 6), commit_pct, bar_col);

    // ── Three Clean Selectors: Runner Profile, Display, Performance Preset ──
    let ctl_y = strip_y + strip_h + 16;
    let ctl_h = 136;
    let col_gap = 16;
    let col_w = (r.width() - col_gap * 2) / 3;

    // 1. Runner Profile + Import Runner...
    let run_r = RECT::new(r.left, ctl_y, r.left + col_w, ctl_y + ctl_h);
    rounded_rect(hdc, run_r, COL_PANEL, COL_BORDER, 14);
    draw_text(hdc, "RUNNER PROFILE", RECT::new(run_r.left + 16, run_r.top + 12, run_r.right - 16, run_r.top + 32), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let r_name = ctx.settings.resolved_runner().parent().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "ztrx".into());
    let cyc_r = RECT::new(run_r.left + 16, run_r.top + 38, run_r.right - 16, run_r.top + 78);
    draw_button(hdc, cyc_r, &format!("{} ({} found)", r_name, ctx.runners.len().max(1)), ctx.fonts.body_bold, ctx.hover == Target::CycleHomeRunner, COL_CYAN, Target::CycleHomeRunner, hits);
    let imp_r = RECT::new(run_r.left + 16, run_r.top + 86, run_r.right - 16, run_r.bottom - 14);
    draw_button(hdc, imp_r, "IMPORT RUNNER...", ctx.fonts.small_bold, ctx.hover == Target::ImportRunner, COL_ORANGE, Target::ImportRunner, hits);

    // 2. Display (Resolution + Borderless/Windowed)
    let disp_r = RECT::new(r.left + col_w + col_gap, ctl_y, r.left + col_w * 2 + col_gap, ctl_y + ctl_h);
    rounded_rect(hdc, disp_r, COL_PANEL, COL_BORDER, 14);
    draw_text(hdc, "DISPLAY", RECT::new(disp_r.left + 16, disp_r.top + 12, disp_r.right - 16, disp_r.top + 32), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let res_btn = RECT::new(disp_r.left + 16, disp_r.top + 38, disp_r.right - 16, disp_r.top + 78);
    draw_button(hdc, res_btn, ctx.settings.display_mode_label(), ctx.fonts.body_bold, ctx.hover == Target::CycleDisplayMode, COL_CYAN, Target::CycleDisplayMode, hits);
    let win_btn = RECT::new(disp_r.left + 16, disp_r.top + 86, disp_r.right - 16, disp_r.bottom - 14);
    let win_lbl = if ctx.settings.borderless { "Borderless Fullscreen" } else { "Windowed" };
    draw_button(hdc, win_btn, win_lbl, ctx.fonts.small_bold, ctx.hover == Target::ToggleBorderless, COL_EMERALD, Target::ToggleBorderless, hits);

    // 3. Hardware-Adaptive Preset (LOW | BALANCED | HIGH + live tier / CUSTOM badge + summary)
    let preset_r = RECT::new(r.left + (col_w + col_gap) * 2, ctl_y, r.right, ctl_y + ctl_h);
    rounded_rect(hdc, preset_r, COL_PANEL, COL_BORDER, 14);
    fill_gradient(hdc, RECT::new(preset_r.left + 14, preset_r.top + 1, preset_r.right - 14, preset_r.top + 3), COL_PINK, COL_CYAN, false);
    let hw_tier = crate::presets::HardwareTier::from_vram_gb(ctx.sys.vram_gb);
    let active_preset = crate::presets::detect_active_preset(ctx.settings, ctx.sys);
    let preset_hdr = if active_preset == crate::presets::PresetKind::Custom {
        format!("PRESET · {} (CUSTOM)", hw_tier.label())
    } else {
        format!("PRESET · {}", hw_tier.label())
    };
    let con_r = RECT::new(preset_r.right - 102, preset_r.top + 10, preset_r.right - 14, preset_r.top + 32);
    let con_lbl = if ctx.settings.is_series_s() { "SERIES-S" } else { "SERIES-X" };
    let con_col = if ctx.settings.is_series_s() { COL_CYAN } else { COL_PINK };
    draw_button(hdc, con_r, con_lbl, ctx.fonts.small_bold, ctx.hover == Target::ToggleConsoleQuick, con_col, Target::ToggleConsoleQuick, hits);
    draw_text(hdc, &preset_hdr, RECT::new(preset_r.left + 16, preset_r.top + 12, con_r.left - 8, preset_r.top + 32), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let p_gap = 8;
    let p_w = (preset_r.width() - 32 - p_gap * 2) / 3;
    let presets = [
        ("LOW", 2usize, COL_EMERALD, active_preset == crate::presets::PresetKind::Low),
        ("BALANCED", 1usize, COL_CYAN, active_preset == crate::presets::PresetKind::Balanced),
        ("HIGH", 0usize, COL_PINK, active_preset == crate::presets::PresetKind::High),
    ];
    for (i, (lbl, idx, col, active)) in presets.iter().enumerate() {
        let px = preset_r.left + 16 + i as i32 * (p_w + p_gap);
        let pr = RECT::new(px, preset_r.top + 38, px + p_w, preset_r.top + 88);
        let hov = ctx.hover == Target::ApplyPreset(*idx);
        let bg = if *active { COL_CYAN_BG } else if hov { COL_PANEL_HOVER } else { COL_SIDEBAR };
        let bd = if *active { *col } else if hov { COL_BORDER_HI } else { COL_BORDER };
        rounded_rect(hdc, pr, bg, bd, 10);
        draw_text(hdc, lbl, pr, ctx.fonts.small_bold, if *active { *col } else { COL_TEXT }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        hits.push((pr, Target::ApplyPreset(*idx)));
    }
    let preset_meta = format!(
        "{} · {}% tex room · {} MB min buf",
        ctx.settings.console_model,
        if ctx.settings.texture_room_percent.trim().is_empty() { "90" } else { ctx.settings.texture_room_percent.trim() },
        if ctx.settings.card_buffers_min_mb.trim().is_empty() { "auto" } else { ctx.settings.card_buffers_min_mb.trim() }
    );
    draw_text(hdc, &preset_meta, RECT::new(preset_r.left + 16, preset_r.top + 96, preset_r.right - 16, preset_r.bottom - 10), ctx.fonts.small, COL_MUTED, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

    // ── Primary actions: Prepare Cache (~35%) | Launch Title (~65%) ────────
    let gap = 16;
    let actions_y = ctl_y + ctl_h + 14;
    let actions_h = 74;
    let prep_w = ((r.width() - gap) * 35) / 100;

    let prep_r = RECT::new(r.left, actions_y, r.left + prep_w, actions_y + actions_h);
    let prep_prog = ctx
        .session
        .as_ref()
        .filter(|s| s.kind == crate::launcher::ProcessKind::Prepare)
        .map(|s| s.parsed_progress.unwrap_or(15));
    let preparing = prep_prog.is_some();
    let cache_ready = ctx.audit.shader_cache_ready && !needs_refresh;
    let prep_bg = if cache_ready && !preparing {
        COL_EMERALD_BG
    } else if ctx.hover == Target::Prepare {
        COL_PANEL_HOVER
    } else {
        COL_PANEL
    };
    let prep_bd = if preparing { COL_AMBER } else if cache_ready { COL_EMERALD } else { COL_CYAN };
    rounded_rect(hdc, prep_r, prep_bg, prep_bd, 14);
    if let Some(pct) = prep_prog {
        let bar_r = RECT::new(prep_r.left + 20, prep_r.bottom - 18, prep_r.right - 20, prep_r.bottom - 8);
        rounded_rect(hdc, bar_r, COL_SIDEBAR, COL_BORDER, 5);
        let fill_w = ((bar_r.width() * pct.clamp(5, 100) as i32) / 100).max(24);
        rounded_rect(hdc, RECT::new(bar_r.left, bar_r.top, bar_r.left + fill_w, bar_r.bottom), COL_CYAN, COL_CYAN, 5);
        let lbl = format!("PREPARING CACHE... {}% ({})", pct, ctx.audit.shader_cache_label);
        draw_text(hdc, &lbl, RECT::new(prep_r.left + 12, prep_r.top + 8, prep_r.right - 12, prep_r.bottom - 22), ctx.fonts.body_bold, COL_AMBER, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    } else {
        let prep_lbl = if cache_ready {
            let short_cache = ctx
                .audit
                .shader_cache_label
                .split(" (")
                .next()
                .unwrap_or(&ctx.audit.shader_cache_label);
            format!("CACHE READY · {short_cache}")
        } else {
            "PREPARE CACHE".into()
        };
        draw_text(
            hdc,
            &prep_lbl,
            RECT::new(prep_r.left + 10, prep_r.top, prep_r.right - 10, prep_r.bottom),
            ctx.fonts.heading,
            if cache_ready { COL_EMERALD } else { COL_CYAN },
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
    }
    hits.push((prep_r, Target::Prepare));

    let play_r = RECT::new(prep_r.right + gap, actions_y, r.right, actions_y + actions_h);
    let play_active = ctx.session.is_some();
    let play_bd = if ctx.hover == Target::LaunchOrStop { COL_WHITE } else if play_active { COL_CORAL } else { COL_ORANGE };
    rounded_rect(hdc, play_r, if play_active { COL_CORAL } else { COL_PINK }, play_bd, 14);
    let play_in = RECT::new(play_r.left + 2, play_r.top + 2, play_r.right - 2, play_r.bottom - 2);
    if play_active {
        fill_gradient(hdc, play_in, COL_CORAL, rgb(176, 28, 62), false);
    } else {
        fill_gradient(hdc, play_in, COL_PINK, COL_ORANGE, false);
    }
    let play_lbl = if let Some(sess) = ctx.session {
        let secs = sess.started_at.elapsed().as_secs();
        format!("■  RUNNING ({:02}:{:02}) — STOP", secs / 60, secs % 60)
    } else {
        "▶  LAUNCH TITLE".into()
    };
    let play_sub = format!(
        "{}  ·  {}  ·  {}  ·  {}",
        ctx.settings.active_runner_kind().badge(),
        ctx.settings.console_model.to_ascii_uppercase(),
        ctx.settings.display_mode_label(),
        if ctx.settings.borderless { "BORDERLESS" } else { "WINDOWED" }
    );
    draw_text(hdc, &play_lbl, RECT::new(play_r.left + 14, play_r.top + 10, play_r.right - 14, play_r.top + 52), ctx.fonts.title, COL_WHITE, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, &play_sub, RECT::new(play_r.left + 14, play_r.top + 50, play_r.right - 14, play_r.bottom - 10), ctx.fonts.small_bold, rgb(255, 232, 240), DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    hits.push((play_r, Target::LaunchOrStop));

    // ── Smart Crash Summary Card OR First-Run Onboarding Card ───────────────
    let mut card_bottom = actions_y + actions_h;
    if let Some((code_tag, diag, _log_path)) = ctx.last_crash {
        let cy = actions_y + actions_h + 14;
        let ch = (r.bottom - cy - 34).clamp(84, 112);
        if ch >= 84 {
            let cr = RECT::new(r.left, cy, r.right, cy + ch);
            rounded_rect(hdc, cr, COL_CORAL_BG, COL_CORAL, 14);
            let title = format!("CRASH SUMMARY · EXIT {code_tag}");
            draw_text(hdc, &title, RECT::new(cr.left + 16, cr.top + 10, cr.right - 340, cr.top + 30), ctx.fonts.small_bold, COL_CORAL, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            draw_text(hdc, diag, RECT::new(cr.left + 16, cr.top + 34, cr.right - 340, cr.bottom - 10), ctx.fonts.small, COL_TEXT, DT_LEFT | DT_WORDBREAK);

            let dis_r = RECT::new(cr.right - 88, cr.top + 16, cr.right - 14, cr.top + 52);
            let zip_r = RECT::new(dis_r.left - 108, cr.top + 16, dis_r.left - 6, cr.top + 52);
            let log_r = RECT::new(zip_r.left - 96, cr.top + 16, zip_r.left - 6, cr.top + 52);
            if crate::guardian::has_last_known_good() {
                let sb_r = RECT::new(log_r.left - 116, cr.top + 16, log_r.left - 6, cr.top + 52);
                draw_button(hdc, sb_r, "SAFE BOOT", ctx.fonts.small_bold, ctx.hover == Target::SafeBootRollback, COL_EMERALD, Target::SafeBootRollback, hits);
            }
            draw_button(hdc, log_r, "OPEN LOG", ctx.fonts.small_bold, ctx.hover == Target::OpenCurrentLog, COL_CYAN, Target::OpenCurrentLog, hits);
            draw_button(hdc, zip_r, "EXPORT ZIP", ctx.fonts.small_bold, ctx.hover == Target::ExportAiBundle, COL_AMBER, Target::ExportAiBundle, hits);
            draw_button(hdc, dis_r, "DISMISS", ctx.fonts.small_bold, ctx.hover == Target::DismissCrash, COL_MUTED, Target::DismissCrash, hits);
            card_bottom = cr.bottom;
        }
    } else if ctx.settings.game_dir.trim().is_empty()
        || !std::path::Path::new(&ctx.settings.game_dir).is_dir()
        || ctx.settings.exe.trim().is_empty()
        || !std::path::Path::new(&ctx.settings.exe).is_file()
    {
        let cy = actions_y + actions_h + 14;
        let ch = (r.bottom - cy - 34).clamp(72, 92);
        if ch >= 72 {
            let cr = RECT::new(r.left, cy, r.right, cy + ch);
            rounded_rect(hdc, cr, COL_AMBER_BG, COL_AMBER, 14);
            draw_text(hdc, "FIRST-RUN SETUP · SELECT YOUR GAME FOLDER", RECT::new(cr.left + 16, cr.top + 12, cr.right - 240, cr.top + 34), ctx.fonts.small_bold, COL_AMBER, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            draw_text(hdc, "Game Master inspects your Xbox Series folder read-only and never modifies game files.", RECT::new(cr.left + 16, cr.top + 38, cr.right - 240, cr.bottom - 10), ctx.fonts.small, COL_TEXT, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            let br_r = RECT::new(cr.right - 224, cr.top + 18, cr.right - 16, cr.bottom - 18);
            draw_button(hdc, br_r, "BROWSE GAME FOLDER...", ctx.fonts.small_bold, ctx.hover == Target::BrowseGameDir, COL_AMBER, Target::BrowseGameDir, hits);
            card_bottom = cr.bottom;
        }
    } else if Some(ctx.system_tip.1.as_str()) != ctx.dismissed_warning && !ctx.system_tip.1.is_empty() {
        let (is_warn, ref tip_msg) = *ctx.system_tip;
        let wy = actions_y + actions_h + 14;
        if wy + 44 <= r.bottom - 30 {
            let wr = RECT::new(r.left, wy, r.right, wy + 42);
            let bg = if is_warn { COL_AMBER_BG } else { COL_EMERALD_BG };
            let bd = if is_warn { COL_AMBER } else { COL_EMERALD };
            rounded_rect(hdc, wr, bg, bd, 12);
            let text_right = if is_warn {
                let dis_r = RECT::new(wr.right - 92, wr.top + 7, wr.right - 10, wr.bottom - 7);
                draw_button(hdc, dis_r, "DISMISS", ctx.fonts.small_bold, ctx.hover == Target::DismissWarning, COL_AMBER, Target::DismissWarning, hits);
                dis_r.left - 12
            } else {
                wr.right - 16
            };
            draw_text(
                hdc,
                &format!("SYSTEM TIP: {tip_msg}"),
                RECT::new(wr.left + 16, wr.top, text_right, wr.bottom),
                ctx.fonts.small_bold,
                bd,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            );
            card_bottom = wr.bottom;
        }
    }

    // ── Footer note + Minimize on Play quick toggle ─────────────────────────
    let note_y = card_bottom + 10;
    if note_y + 26 <= r.bottom {
        let min_r = RECT::new(r.right - 178, note_y - 2, r.right, note_y + 24);
        let min_lbl = if ctx.settings.zero_overhead { "Min on Play: ON" } else { "Min on Play: OFF" };
        draw_button(hdc, min_r, min_lbl, ctx.fonts.small_bold, ctx.hover == Target::ToggleZeroOverhead, if ctx.settings.zero_overhead { COL_EMERALD } else { COL_MUTED }, Target::ToggleZeroOverhead, hits);

        let hud_r = RECT::new(min_r.left - 148, note_y - 2, min_r.left - 8, note_y + 24);
        let hud_lbl = format!("HUD: {}", ctx.settings.hud_mode_label());
        draw_button(hdc, hud_r, &hud_lbl, ctx.fonts.small_bold, ctx.hover == Target::CycleHudMode, COL_CYAN, Target::CycleHudMode, hits);

        let km_r = RECT::new(hud_r.left - 176, note_y - 2, hud_r.left - 8, note_y + 24);
        let km_lbl = format!("KEYS: {}", crate::keymap::KeymapPreset::from_id(&ctx.settings.keymap_preset).label());
        draw_button(hdc, km_r, &km_lbl, ctx.fonts.small_bold, ctx.hover == Target::CycleKeymapPreset, COL_PINK, Target::CycleKeymapPreset, hits);

        let foot = format!(
            "{} · RAM {}% · COMMIT {}%",
            ctx.playtime_label,
            ctx.ram_history.last().copied().unwrap_or(0),
            ctx.commit_history.last().copied().unwrap_or(0)
        );
        draw_text(
            hdc,
            &foot,
            RECT::new(r.left + 4, note_y, km_r.left - 10, note_y + 22),
            ctx.fonts.small,
            COL_MUTED,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
    }

    // ── Live 2-Column Dashboard (fills remaining lower half of PLAY tab) ───
    let dash_y = note_y + 32;
    if r.bottom - dash_y >= 112 {
        let dash_gap = 16;
        let dash_w = (r.width() - dash_gap) / 2;
        let left_card = RECT::new(r.left, dash_y, r.left + dash_w, r.bottom - 2);
        let right_card = RECT::new(left_card.right + dash_gap, dash_y, r.right, r.bottom - 2);

        // Left Card: LIVE HARDWARE & PRE-FLIGHT BREAKDOWN
        rounded_rect(hdc, left_card, COL_PANEL, COL_BORDER, 14);
        let pf = crate::preflight::evaluate(ctx.settings, ctx.sys, ctx.audit);
        let pf_col = if pf.score >= 92 { COL_EMERALD } else if pf.score >= 75 { COL_CYAN } else { COL_AMBER };
        let pf_badge_r = RECT::new(left_card.right - 200, left_card.top + 8, left_card.right - 12, left_card.top + 30);
        draw_pill(hdc, pf_badge_r, &pf.summary, ctx.fonts.small_bold, COL_SIDEBAR, pf_col, pf_col);
        draw_text(
            hdc,
            "LIVE HARDWARE & PRE-FLIGHT",
            RECT::new(left_card.left + 14, left_card.top + 8, pf_badge_r.left - 8, left_card.top + 30),
            ctx.fonts.small_bold,
            COL_CYAN,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );

        let bars_x = left_card.left + 14;
        let bars_w = ((left_card.width() - 36) * 48) / 100;
        let mut by = left_card.top + 36;
        let tex_pct: u32 = ctx.settings.texture_room_percent.trim().parse().unwrap_or(85);
        let vram_gb = if ctx.sys.vram_gb > 0.5 { ctx.sys.vram_gb } else { 12.0 };
        let ceil_mb = ((vram_gb * 1024.0) * (tex_pct as f64 / 100.0)) as u32;
        let ram_used = (ctx.sys.total_ram_gb - ctx.sys.avail_ram_gb).max(0.0);
        let ram_pct = ctx.ram_history.last().copied().unwrap_or(ctx.sys.mem_load_pct.min(100));
        let com_used = (ctx.sys.total_commit_gb - ctx.sys.avail_commit_gb).max(0.0);
        let com_pct = ctx.commit_history.last().copied().unwrap_or(0);

        let bar_rows = [
            (
                format!("VRAM {:.1}G · {}% CEIL (~{} MB)", vram_gb, tex_pct, ceil_mb),
                tex_pct.clamp(5, 100),
                if tex_pct >= 92 { COL_AMBER } else { COL_EMERALD },
            ),
            (
                format!("RAM {:.1}/{:.1} GB ({}%)", ram_used, ctx.sys.total_ram_gb, ram_pct),
                ram_pct.clamp(2, 100),
                if ram_pct >= 88 { COL_CORAL } else { COL_CYAN },
            ),
            (
                format!("COMMIT {:.1}/{:.1} GB ({}%)", com_used, ctx.sys.total_commit_gb, com_pct),
                com_pct.clamp(2, 100),
                if com_pct >= 88 { COL_CORAL } else if com_pct >= 72 { COL_AMBER } else { COL_PINK },
            ),
        ];
        for (lbl, pct, col) in bar_rows {
            if by + 24 > left_card.bottom - 6 {
                break;
            }
            draw_text(hdc, &lbl, RECT::new(bars_x, by, bars_x + bars_w, by + 16), ctx.fonts.small, COL_TEXT, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            draw_progress_bar(hdc, RECT::new(bars_x, by + 17, bars_x + bars_w, by + 23), pct, col);
            by += 27;
        }

        let chk_x = bars_x + bars_w + 14;
        let chk_r_edge = left_card.right - 12;
        let mut cy = left_card.top + 36;
        for (ok, msg) in &pf.checks {
            if cy + 16 > left_card.bottom - 6 {
                break;
            }
            let prefix = if *ok { "✓ " } else { "! " };
            draw_text(
                hdc,
                &format!("{prefix}{msg}"),
                RECT::new(chk_x, cy, chk_r_edge, cy + 16),
                ctx.fonts.small,
                if *ok { COL_EMERALD } else { COL_AMBER },
                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            );
            cy += 17;
        }

        // Right Card: LAST SESSION & LIVE ACTIVITY
        rounded_rect(hdc, right_card, COL_PANEL, COL_BORDER, 14);
        let vlog_r = RECT::new(right_card.right - 104, right_card.top + 8, right_card.right - 12, right_card.top + 30);
        draw_button(hdc, vlog_r, "VIEW LOGS", ctx.fonts.small_bold, ctx.hover == Target::ViewLogsQuick, COL_CYAN, Target::ViewLogsQuick, hits);
        draw_text(
            hdc,
            "LAST SESSION & LIVE ACTIVITY",
            RECT::new(right_card.left + 14, right_card.top + 8, vlog_r.left - 8, right_card.top + 30),
            ctx.fonts.small_bold,
            COL_ORANGE,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );

        let mut ry = right_card.top + 34;
        let sess_summary = if let Some(rec) = ctx.recent_sessions.first() {
            format!(
                "{} · {} · {}s · {}",
                rec.runner, rec.console, rec.duration_secs, rec.peak_fps
            )
        } else {
            "No completed session recorded yet — launch title to benchmark.".into()
        };
        draw_text(
            hdc,
            &sess_summary,
            RECT::new(right_card.left + 14, ry, right_card.right - 14, ry + 18),
            ctx.fonts.small_bold,
            COL_WHITE,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        ry += 20;

        if ctx.fps_history.len() >= 2 && ry + 28 < right_card.bottom - 36 {
            let scaled: Vec<u32> = ctx.fps_history.iter().map(|&v| ((v * 100) / 350).min(100)).collect();
            draw_sparkline(hdc, RECT::new(right_card.left + 14, ry, right_card.right - 14, ry + 24), &scaled, COL_EMERALD);
            ry += 28;
        }

        let tail: Vec<&String> = ctx.log_lines.iter().rev().filter(|l| !l.trim().is_empty()).take(5).collect();
        if tail.is_empty() {
            draw_text(
                hdc,
                "Waiting for runner output...",
                RECT::new(right_card.left + 14, ry, right_card.right - 14, ry + 18),
                ctx.fonts.mono,
                COL_DIM,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            );
        } else {
            for line in tail.into_iter().rev() {
                if ry + 15 > right_card.bottom - 4 {
                    break;
                }
                let col = if crate::diagnostics::is_known_wave32_note(line) {
                    COL_CYAN
                } else if line.to_ascii_lowercase().contains("error") {
                    COL_CORAL
                } else if line.to_ascii_lowercase().contains("warn") {
                    COL_AMBER
                } else {
                    COL_MUTED
                };
                draw_text(
                    hdc,
                    line,
                    RECT::new(right_card.left + 14, ry, right_card.right - 14, ry + 15),
                    ctx.fonts.mono,
                    col,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
                ry += 15;
            }
        }
    }
}

unsafe fn draw_runners(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    rounded_rect(hdc, r, COL_PANEL, COL_BORDER, 16);
    draw_text(hdc, "MULTI-RUNNER HUB", RECT::new(r.left + 24, r.top + 16, (r.right - 590).max(r.left + 180), r.top + 46), ctx.fonts.heading, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let (sub, sub_col) = if let Some(ref err) = ctx.portwatch.error {
        (
            format!("Adapter: {}  ·  Sync failed: {}", ctx.settings.active_runner_kind().badge(), err),
            COL_AMBER,
        )
    } else {
        (
            format!("Adapter: {}  ·  Source: {} ({})", ctx.settings.active_runner_kind().badge(), ctx.portwatch.source_url, ctx.portwatch.synced_at),
            COL_CYAN,
        )
    };
    draw_text(hdc, &sub, RECT::new(r.left + 24, r.top + 46, r.left + 560, r.top + 66), ctx.fonts.small, sub_col, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Clean 3-button Header (Cache tools sit neatly in the Installed Runners footer)
    let b_sync = RECT::new(r.right - 130, r.top + 20, r.right - 20, r.top + 54);
    let sync_lbl = if ctx.portwatch_syncing { "SYNCING..." } else { "SYNC LIVE" };
    draw_button(hdc, b_sync, sync_lbl, ctx.fonts.small_bold, ctx.hover == Target::SyncPortWatch, COL_PINK, Target::SyncPortWatch, hits);
    let b_pw = RECT::new(b_sync.left - 124, r.top + 20, b_sync.left - 10, r.top + 54);
    draw_button(hdc, b_pw, "TRACKERS.TXT", ctx.fonts.small_bold, ctx.hover == Target::OpenTrackersFile, COL_CYAN, Target::OpenTrackersFile, hits);
    let b_dir = RECT::new(b_pw.left - 124, r.top + 20, b_pw.left - 10, r.top + 54);
    draw_button(hdc, b_dir, "RUNNERS DIR", ctx.fonts.small_bold, ctx.hover == Target::OpenRunnersDir, COL_TEXT, Target::OpenRunnersDir, hits);

    draw_line(hdc, r.left + 24, r.top + 74, r.right - 24, r.top + 74, COL_BORDER, 1);

    let gap = 16;
    let left_w = ((r.width() - 48 - gap) * 46) / 100;
    let left_r = RECT::new(r.left + 20, r.top + 84, r.left + 20 + left_w, r.bottom - 16);
    let right_r = RECT::new(left_r.right + gap, r.top + 84, r.right - 20, r.bottom - 16);

    // LEFT PANE: Installed Local Runners
    rounded_rect(hdc, left_r, COL_SIDEBAR, COL_BORDER, 12);
    let add_r = RECT::new(left_r.right - 64, left_r.top + 8, left_r.right - 10, left_r.top + 32);
    draw_button(hdc, add_r, "ADD...", ctx.fonts.small_bold, ctx.hover == Target::BrowseRunner, COL_WHITE, Target::BrowseRunner, hits);
    let ref_r = RECT::new(add_r.left - 68, left_r.top + 8, add_r.left - 6, left_r.top + 32);
    draw_button(hdc, ref_r, "RESCAN", ctx.fonts.small_bold, ctx.hover == Target::RescanRunners, COL_MUTED, Target::RescanRunners, hits);
    let junk_r = RECT::new(ref_r.left - 74, left_r.top + 8, ref_r.left - 6, left_r.top + 32);
    draw_button(hdc, junk_r, "CLEANUP", ctx.fonts.small_bold, ctx.hover == Target::CleanJunkRunners, COL_AMBER, Target::CleanJunkRunners, hits);
    draw_text(hdc, "LOCAL PATCHED BUILDS", RECT::new(left_r.left + 14, left_r.top + 10, (junk_r.left - 6).max(left_r.left + 140), left_r.top + 30), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    let active_runner_lc = ctx.settings.resolved_runner().to_string_lossy().to_ascii_lowercase();
    let runners_root = runners_dir();
    let mut y = left_r.top + 42;
    let r_start = ctx.runner_scroll.min(ctx.runners.len().saturating_sub(1));
    for (idx, run) in ctx.runners.iter().enumerate().skip(r_start) {
        if y + 60 > left_r.bottom - 8 {
            break;
        }
        let cr = RECT::new(left_r.left + 10, y, left_r.right - 10, y + 58);
        let is_active = run.path.to_string_lossy().to_ascii_lowercase() == active_runner_lc;
        let hov = ctx.hover == Target::SelectRunner(idx);
        rounded_rect(
            hdc,
            cr,
            if is_active { COL_CYAN_BG } else if hov { COL_PANEL_HOVER } else { COL_PANEL },
            if is_active { COL_CYAN } else if hov { COL_BORDER_HI } else { COL_BORDER },
            10,
        );
        // Push SelectRunner FIRST so DEL / ROLLBACK / CycleRunnerKind sit on top in hit_test!
        hits.push((cr, Target::SelectRunner(idx)));

        let badge_r = RECT::new(cr.right - 154, cr.top + 14, cr.right - 10, cr.bottom - 14);
        let del_r = RECT::new(badge_r.left - 50, cr.top + 14, badge_r.left - 6, cr.bottom - 14);
        let in_runners = run.path.starts_with(&runners_root);
        let show_del = in_runners && !is_active;
        let text_right = if run.has_backup {
            del_r.left - 90
        } else if show_del {
            del_r.left - 8
        } else {
            badge_r.left - 8
        };

        let slug_chk = crate::runners::runner_slug(&run.path, run.kind);
        let prof_tag = if Settings::has_runner_profile(&slug_chk) { " · PROF" } else { "" };
        let title_str = if !run.commit.is_empty() {
            format!("{}  ·  @{}{}", run.name, run.commit, prof_tag)
        } else {
            format!("{}{}", run.name, prof_tag)
        };
        draw_text(hdc, &title_str, RECT::new(cr.left + 12, cr.top + 6, text_right.max(cr.left + 120), cr.top + 26), ctx.fonts.body_bold, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let sub_r = if !run.built_at.is_empty() {
            format!("{}  ·  Updated {}  ·  Cache: {}", run.size_label, run.built_at, run.cache_label)
        } else {
            format!("{}  ·  Cache: {}", run.size_label, run.cache_label)
        };
        draw_text(hdc, &sub_r, RECT::new(cr.left + 12, cr.top + 30, text_right.max(cr.left + 120), cr.bottom - 6), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

        if is_active {
            draw_button(hdc, badge_r, ctx.settings.active_runner_kind().badge(), ctx.fonts.small_bold, ctx.hover == Target::CycleRunnerKind, COL_CYAN, Target::CycleRunnerKind, hits);
        } else {
            draw_pill(hdc, badge_r, run.kind.badge(), ctx.fonts.small_bold, COL_PANEL_ALT, COL_TEXT, COL_BORDER);
        }
        if show_del {
            draw_button(hdc, del_r, "DEL", ctx.fonts.small_bold, ctx.hover == Target::DeleteRunner(idx), COL_CORAL, Target::DeleteRunner(idx), hits);
        }
        if run.has_backup {
            let rb_right = if show_del { del_r.left - 6 } else { badge_r.left - 6 };
            let rb_r = RECT::new(rb_right - 78, cr.top + 14, rb_right, cr.bottom - 14);
            draw_button(hdc, rb_r, "ROLLBACK", ctx.fonts.small_bold, ctx.hover == Target::RollbackRunner(idx), COL_AMBER, Target::RollbackRunner(idx), hits);
        }
        y += 64;
    }

    let cf_top = left_r.bottom - 42;
    if cf_top > y + 4 {
        let cw = (left_r.width() - 36) / 3;
        draw_button(hdc, RECT::new(left_r.left + 10, cf_top, left_r.left + 10 + cw, left_r.bottom - 8), "CACHE DIR", ctx.fonts.small_bold, ctx.hover == Target::OpenCacheDir, COL_ORANGE, Target::OpenCacheDir, hits);
        draw_button(hdc, RECT::new(left_r.left + 18 + cw, cf_top, left_r.left + 18 + cw * 2, left_r.bottom - 8), "BACKUP CACHE", ctx.fonts.small_bold, ctx.hover == Target::ExportCacheZip, COL_EMERALD, Target::ExportCacheZip, hits);
        draw_button(hdc, RECT::new(left_r.left + 26 + cw * 2, cf_top, left_r.right - 10, left_r.bottom - 8), "CLEAR CACHE", ctx.fonts.small_bold, ctx.hover == Target::ClearActiveCache, COL_CORAL, Target::ClearActiveCache, hits);
    }

    // RIGHT PANE: Real-Time PortWatch VI Scanner Feed
    rounded_rect(hdc, right_r, COL_SIDEBAR, COL_BORDER, 12);
    draw_text(hdc, "LIVE TRACKER & GITHUB FAILOVER FEED (SCROLLABLE)", RECT::new(right_r.left + 14, right_r.top + 10, right_r.right - 14, right_r.top + 30), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    if let Some(ref err) = ctx.portwatch.error {
        if ctx.portwatch.projects.is_empty() {
            draw_text(
                hdc,
                &format!("Could not reach portwatch-vi.xyz:
{err}

Click SYNC LIVE to retry or PORTWATCH VI to open in browser."),
                RECT::new(right_r.left + 16, right_r.top + 44, right_r.right - 16, right_r.bottom - 16),
                ctx.fonts.body,
                COL_AMBER,
                DT_LEFT | DT_WORDBREAK,
            );
            return;
        }
    }

    if ctx.portwatch.projects.is_empty() {
        let msg = if ctx.portwatch_syncing {
            "Fetching live standings from https://portwatch-vi.xyz/tracker.json ..."
        } else {
            "Click SYNC LIVE to scan the latest PC compatibility runners, forks, and builds from portwatch-vi.xyz."
        };
        draw_text(hdc, msg, RECT::new(right_r.left + 16, right_r.top + 48, right_r.right - 16, right_r.top + 120), ctx.fonts.body, COL_MUTED, DT_LEFT | DT_WORDBREAK);
        return;
    }

    let mut py = right_r.top + 40;
    let start_idx = ctx.portwatch_scroll.min(ctx.portwatch.projects.len().saturating_sub(1));
    for (idx, proj) in ctx.portwatch.projects.iter().enumerate().skip(start_idx) {
        if py + 52 > right_r.bottom - 8 {
            break;
        }
        let pr = RECT::new(right_r.left + 10, py, right_r.right - 10, py + 48);
        rounded_rect(hdc, pr, if idx % 2 == 0 { COL_PANEL } else { COL_PANEL_ALT }, COL_BORDER, 10);

        let pct_r = RECT::new(pr.left + 8, pr.top + 10, pr.left + 68, pr.bottom - 10);
        let (pbg, pfg) = if proj.score >= 50.0 {
            (COL_EMERALD_BG, COL_EMERALD)
        } else if proj.score >= 30.0 {
            (COL_CYAN_BG, COL_CYAN)
        } else {
            (COL_PANEL_ALT, COL_MUTED)
        };
        let pct_txt = if proj.score > 0.0 { format!("{:.1}%", proj.score) } else { "LIVE".into() };
        draw_pill(hdc, pct_r, &pct_txt, ctx.fonts.small_bold, pbg, pfg, pfg);

        let name_lc = proj.name.to_ascii_lowercase();
        let slug_lc = proj.repo_slug();
        let repo_lc = proj.repo.to_ascii_lowercase();
        let matched_runner = ctx.runners.iter().find(|r| {
            matches_portwatch_project(r, &runners_root, &name_lc, &slug_lc, &repo_lc)
        });
        let installed = matched_runner.is_some();
        let remote_sha = ctx.portwatch.remote_sha_for(&proj.repo);
        let local_sha = matched_runner.map(|r| r.commit.as_str()).unwrap_or("");
        let has_new_commit = installed
            && !local_sha.is_empty()
            && remote_sha.map(|rs| !rs.eq_ignore_ascii_case(local_sha)).unwrap_or(false);
        let up_to_date = installed
            && !has_new_commit
            && matched_runner.map(|r| !r.built_at.is_empty()).unwrap_or(false);
        let has_repo = !proj.is_private && proj.repo.contains('/');
        let right_pad = if has_repo { 196 } else { 96 };

        let title = if proj.is_private {
            format!("{} (PRIVATE)", proj.name)
        } else if !proj.repo.is_empty() {
            format!("{}  ·  {}", proj.name, proj.repo)
        } else {
            proj.name.clone()
        };
        draw_text(hdc, &title, RECT::new(pct_r.right + 10, pr.top + 4, pr.right - right_pad, pr.top + 24), ctx.fonts.body_bold, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let detail = if !proj.summary.is_empty() {
            format!("{} — {}", proj.stage, proj.summary)
        } else {
            proj.stage.clone()
        };
        draw_text(hdc, &detail, RECT::new(pct_r.right + 10, pr.top + 24, pr.right - right_pad, pr.bottom - 4), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

        let open_r = RECT::new(pr.right - 82, pr.top + 9, pr.right - 8, pr.bottom - 9);
        let lbl = if proj.is_private { "TRACK" } else { "GITHUB" };
        draw_button(hdc, open_r, lbl, ctx.fonts.small_bold, ctx.hover == Target::OpenPortWatchRepo(idx), COL_CYAN, Target::OpenPortWatchRepo(idx), hits);

        if has_repo {
            let build_r = RECT::new(open_r.left - 104, pr.top + 9, open_r.left - 6, pr.bottom - 9);
            if has_new_commit {
                let hov = ctx.hover == Target::BuildPortWatchRepo(idx);
                rounded_rect(hdc, build_r, if hov { rgb(86, 56, 16) } else { COL_AMBER_BG }, COL_AMBER, 8);
                draw_text(hdc, "⚡ NEW COMMIT", build_r, ctx.fonts.small_bold, if hov { COL_WHITE } else { COL_AMBER }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                hits.push((build_r, Target::BuildPortWatchRepo(idx)));
            } else if up_to_date {
                let hov = ctx.hover == Target::BuildPortWatchRepo(idx);
                rounded_rect(hdc, build_r, if hov { rgb(18, 72, 54) } else { COL_EMERALD_BG }, COL_EMERALD, 8);
                draw_text(hdc, "✓ UP TO DATE", build_r, ctx.fonts.small_bold, if hov { COL_WHITE } else { COL_EMERALD }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                hits.push((build_r, Target::BuildPortWatchRepo(idx)));
            } else {
                let blbl = if installed { "UPDATE" } else { "GET/BUILD" };
                let bcol = if installed { COL_CYAN } else { COL_PINK };
                draw_button(hdc, build_r, blbl, ctx.fonts.small_bold, ctx.hover == Target::BuildPortWatchRepo(idx), bcol, Target::BuildPortWatchRepo(idx), hits);
            }
        }
        py += 54;
    }
}

unsafe fn draw_library(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    rounded_rect(hdc, r, COL_PANEL, COL_BORDER, 16);

    // Header row (two-tier so buttons never collide with title at narrower widths)
    let b_bk = RECT::new(r.right - 134, r.top + 18, r.right - 20, r.top + 50);
    let bk_lbl = format!("BACKUP ({})", ctx.backup_count);
    draw_button(hdc, b_bk, &bk_lbl, ctx.fonts.small_bold, ctx.hover == Target::BackupSaves, COL_EMERALD, Target::BackupSaves, hits);
    let b_rst = RECT::new(b_bk.left - 114, r.top + 18, b_bk.left - 8, r.top + 50);
    draw_button(hdc, b_rst, "RESTORE", ctx.fonts.small_bold, ctx.hover == Target::RestoreLatestSave, COL_AMBER, Target::RestoreLatestSave, hits);
    let b_mod = RECT::new(b_rst.left - 102, r.top + 18, b_rst.left - 8, r.top + 50);
    let mod_lbl = format!("MODS ({})", ctx.mod_count);
    draw_button(hdc, b_mod, &mod_lbl, ctx.fonts.small_bold, ctx.hover == Target::OpenModsDir, COL_PINK, Target::OpenModsDir, hits);
    let b3 = RECT::new(b_mod.left - 98, r.top + 18, b_mod.left - 8, r.top + 50);
    draw_button(hdc, b3, "RESCAN", ctx.fonts.small_bold, ctx.hover == Target::Rescan, COL_CYAN, Target::Rescan, hits);

    draw_text(hdc, "TOOLS & ASSETS", RECT::new(r.left + 24, r.top + 16, (b3.left - 12).max(r.left + 160), r.top + 46), ctx.fonts.heading, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let sub = format!("Build {}  ·  {}  ·  {}", ctx.settings.version_label(), ctx.data_label, ctx.file_count_label);
    draw_text(hdc, &sub, RECT::new(r.left + 24, r.top + 46, (b3.left - 12).max(r.left + 220), r.top + 66), ctx.fonts.small, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    draw_line(hdc, r.left + 24, r.top + 80, r.right - 24, r.top + 80, COL_BORDER, 1);

    // Table header
    let mut y = r.top + 92;
    draw_text(hdc, "COMPONENT", RECT::new(r.left + 28, y, r.left + 240, y + 22), ctx.fonts.small_bold, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, "STATUS", RECT::new(r.left + 250, y, r.left + 360, y + 22), ctx.fonts.small_bold, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, "SIZE / DETAIL", RECT::new(r.left + 370, y, r.left + 510, y + 22), ctx.fonts.small_bold, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, "RESOLVED PATH", RECT::new(r.left + 520, y, r.right - 28, y + 22), ctx.fonts.small_bold, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    y += 30;

    for (idx, item) in ctx.audit.items.iter().enumerate() {
        let row_r = RECT::new(r.left + 20, y, r.right - 20, y + 46);
        if row_r.bottom > r.bottom - 62 {
            break;
        }
        rounded_rect(hdc, row_r, if idx % 2 == 0 { COL_PANEL_ALT } else { COL_SIDEBAR }, COL_BORDER, 10);
        draw_text(hdc, item.name, RECT::new(row_r.left + 12, row_r.top + 4, row_r.left + 224, row_r.top + 24), ctx.fonts.body_bold, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        draw_text(hdc, item.role, RECT::new(row_r.left + 12, row_r.top + 24, row_r.left + 224, row_r.bottom - 4), ctx.fonts.small, COL_DIM, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

        let pill_r = RECT::new(row_r.left + 230, row_r.top + 10, row_r.left + 324, row_r.bottom - 10);
        let is_optional = idx == 7;
        let (bg, fg, lbl) = if item.present {
            (COL_EMERALD_BG, COL_EMERALD, "FOUND")
        } else if is_optional {
            (COL_PANEL, COL_MUTED, "OPTIONAL")
        } else {
            (COL_CORAL_BG, COL_CORAL, "MISSING")
        };
        draw_pill(hdc, pill_r, lbl, ctx.fonts.small_bold, bg, fg, fg);

        draw_text(hdc, &item.detail, RECT::new(row_r.left + 350, row_r.top, row_r.left + 490, row_r.bottom), ctx.fonts.small_bold, COL_TEXT, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        draw_text(hdc, &item.path, RECT::new(row_r.left + 500, row_r.top, row_r.right - 14, row_r.bottom), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_PATH_ELLIPSIS);
        y += 52;
    }

    // Interactive Mod / Patch Toggle Strip (when items exist in mods\)
    if !ctx.mods.is_empty() && y + 40 <= r.bottom - 56 {
        let mr = RECT::new(r.left + 20, y + 2, r.right - 20, y + 36);
        rounded_rect(hdc, mr, COL_SIDEBAR, COL_BORDER, 8);
        draw_text(hdc, "MODS / PATCHES:", RECT::new(mr.left + 12, mr.top, mr.left + 130, mr.bottom), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let mut mx = mr.left + 134;
        for (mi, mitem) in ctx.mods.iter().enumerate() {
            let mw = 132;
            if mx + mw > mr.right - 10 { break; }
            let pr = RECT::new(mx, mr.top + 4, mx + mw, mr.bottom - 4);
            let hov = ctx.hover == Target::ToggleMod(mi);
            let lbl = format!("{} {}", if mitem.enabled { "●" } else { "○" }, mitem.name);
            rounded_rect(hdc, pr, if mitem.enabled { COL_EMERALD_BG } else if hov { COL_PANEL_HOVER } else { COL_PANEL }, if mitem.enabled { COL_EMERALD } else { COL_BORDER }, 6);
            draw_text(hdc, &lbl, pr, ctx.fonts.small_bold, if mitem.enabled { COL_EMERALD } else { COL_MUTED }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            hits.push((pr, Target::ToggleMod(mi)));
            mx += mw + 6;
        }
    }

    // Warnings footer inside Library (critical only; user can dismiss it)
    let warn = ctx.audit.warnings.first().filter(|w| Some(w.as_str()) != ctx.dismissed_warning);
    if let Some(warn) = warn {
        let wr = RECT::new(r.left + 20, r.bottom - 50, r.right - 20, r.bottom - 14);
        rounded_rect(hdc, wr, COL_AMBER_BG, COL_AMBER, 10);
        let close_r = RECT::new(wr.right - 36, wr.top + 6, wr.right - 8, wr.bottom - 6);
        rounded_rect(hdc, close_r, COL_AMBER_BG, COL_AMBER, 6);
        draw_text(hdc, "×", close_r, ctx.fonts.small_bold, COL_AMBER, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        hits.push((close_r, Target::DismissWarning));
        draw_text(hdc, &format!("NOTE: {warn}"), RECT::new(wr.left + 14, wr.top, wr.right - 44, wr.bottom), ctx.fonts.small_bold, COL_AMBER, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    } else {
        let fx_note = if ctx.settings.resolved_runner().to_string_lossy().to_ascii_lowercase().contains("ztrx") {
            "All core checks passed. Note: ztrx has no fidelityfx.rs — host FSR is inactive."
        } else {
            "All core build checks passed. Game files remain untouched on disk."
        };
        draw_text(hdc, fx_note, RECT::new(r.left + 24, r.bottom - 42, r.right - 24, r.bottom - 16), ctx.fonts.small, COL_EMERALD, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    }
}

unsafe fn draw_tuning(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    rounded_rect(hdc, r, COL_PANEL, COL_BORDER, 16);
    draw_text(hdc, "ADVANCED / ENGINEERING", RECT::new(r.left + 24, r.top + 16, r.left + 380, r.top + 46), ctx.fonts.heading, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    // Top-right header toolbar: RESET DEFAULTS | A/B TEST | HIGH | BALANCED | LOW
    let p3 = RECT::new(r.right - 104, r.top + 20, r.right - 20, r.top + 52);
    draw_button(hdc, p3, "LOW", ctx.fonts.small_bold, ctx.hover == Target::ApplyPreset(2), COL_AMBER, Target::ApplyPreset(2), hits);
    let p2 = RECT::new(p3.left - 118, r.top + 20, p3.left - 8, r.top + 52);
    draw_button(hdc, p2, "BALANCED", ctx.fonts.small_bold, ctx.hover == Target::ApplyPreset(1), COL_CYAN, Target::ApplyPreset(1), hits);
    let p1 = RECT::new(p2.left - 92, r.top + 20, p2.left - 8, r.top + 52);
    draw_button(hdc, p1, "HIGH", ctx.fonts.small_bold, ctx.hover == Target::ApplyPreset(0), COL_PINK, Target::ApplyPreset(0), hits);
    let ab_r = RECT::new(p1.left - 166, r.top + 20, p1.left - 8, r.top + 52);
    let ab_exp = crate::ab_bench::AbExperiment::from_id(&ctx.settings.ab_experiment);
    let ab_col = if ab_exp == crate::ab_bench::AbExperiment::Off { COL_MUTED } else { COL_EMERALD };
    draw_button(hdc, ab_r, ab_exp.label(), ctx.fonts.small_bold, ctx.hover == Target::CycleAbExperiment, ab_col, Target::CycleAbExperiment, hits);
    let rst_r = RECT::new(ab_r.left - 142, r.top + 20, ab_r.left - 8, r.top + 52);
    draw_button(hdc, rst_r, "RESET DEFAULTS", ctx.fonts.small_bold, ctx.hover == Target::ResetV11Perf, COL_CORAL, Target::ResetV11Perf, hits);

    let active_env_count = crate::launcher::build_runner_env(ctx.settings).len();
    let tuning_sub = format!(
        "{active_env_count} verified runner env flags active  ·  Click any section header to collapse/expand  ·  Scroll for more"
    );
    draw_text(hdc, &tuning_sub, RECT::new(r.left + 24, r.top + 46, rst_r.left - 12, r.top + 66), ctx.fonts.small, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_line(hdc, r.left + 24, r.top + 74, r.right - 24, r.top + 74, COL_BORDER, 1);

    let clip_top = r.top + 78;
    let clip_bot = r.bottom - 6;
    let saved_dc = SaveDC(hdc);
    IntersectClipRect(hdc, r.left + 2, clip_top, r.right - 2, clip_bot);
    let mut card_hits: Vec<(RECT, Target)> = Vec::with_capacity(64);

    let left_x = r.left + 24;
    let inner_w = r.width() - 56; // leaves 8px right margin for scrollbar
    let row_h = 54;
    let row_gap = 8;
    let start_y = r.top + 84 - ctx.tuning_scroll as i32;
    let mut y = start_y;

    // Helper: collapsible section header with active-count badge
    let draw_sec_hdr = |hdc: HDC,
                        sec_idx: usize,
                        title: &str,
                        on_cnt: usize,
                        tot_cnt: usize,
                        y_pos: i32,
                        hits: &mut Vec<(RECT, Target)>| {
        let collapsed = ctx.adv_collapsed[sec_idx];
        let hr = RECT::new(left_x, y_pos, left_x + inner_w, y_pos + 28);
        let hov = ctx.hover == Target::ToggleAdvSection(sec_idx);
        rounded_rect(hdc, hr, if hov { COL_PANEL_HOVER } else { COL_SIDEBAR }, if hov { COL_BORDER_HI } else { COL_BORDER }, 8);
        let arrow = if collapsed { "▸" } else { "▾" };
        draw_text(
            hdc,
            &format!("{arrow}  {title}"),
            RECT::new(hr.left + 12, hr.top, hr.right - 130, hr.bottom),
            ctx.fonts.small_bold,
            COL_WHITE,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        let badge_r = RECT::new(hr.right - 118, hr.top + 4, hr.right - 8, hr.bottom - 4);
        let badge_lbl = format!("{on_cnt}/{tot_cnt} ON");
        draw_pill(
            hdc,
            badge_r,
            &badge_lbl,
            ctx.fonts.small_bold,
            if on_cnt > 0 { COL_CYAN_BG } else { COL_PANEL },
            if on_cnt > 0 { COL_CYAN } else { COL_MUTED },
            if on_cnt > 0 { COL_CYAN } else { COL_BORDER },
        );
        hits.push((hr, Target::ToggleAdvSection(sec_idx)));
    };

    // Helper: uniform card with standardized border colors (accent=on, muted=off, amber=exp) and 84px pill
    let draw_adv_card = |hdc: HDC,
                         cr: RECT,
                         title: &str,
                         desc: &str,
                         on: bool,
                         is_exp: bool,
                         pill_lbl: &str,
                         target: Target,
                         hits: &mut Vec<(RECT, Target)>| {
        let hov = ctx.hover == target;
        let bg = if on {
            if is_exp { COL_AMBER_BG } else { COL_CYAN_BG }
        } else if hov {
            COL_PANEL_HOVER
        } else {
            COL_PANEL_ALT
        };
        let bd = if on {
            if is_exp { COL_AMBER } else { COL_CYAN }
        } else if hov {
            COL_BORDER_HI
        } else {
            COL_BORDER
        };
        rounded_rect(hdc, cr, bg, bd, 11);

        let pill_w = 92;
        let pill = RECT::new(cr.right - pill_w - 8, cr.top + 8, cr.right - 8, cr.top + 28);
        let exp_w = 40;
        let title_right = if is_exp { pill.left - exp_w - 8 } else { pill.left - 6 };
        draw_text(
            hdc,
            title,
            RECT::new(cr.left + 12, cr.top + 7, title_right, cr.top + 27),
            ctx.fonts.small_bold,
            COL_WHITE,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        if is_exp {
            let exp_r = RECT::new(pill.left - exp_w - 4, cr.top + 9, pill.left - 4, cr.top + 27);
            rounded_rect(hdc, exp_r, COL_AMBER_BG, COL_AMBER, 8);
            draw_text(hdc, "EXP", exp_r, ctx.fonts.small_bold, COL_AMBER, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
        draw_text(
            hdc,
            desc,
            RECT::new(cr.left + 12, cr.top + 28, cr.right - 10, cr.bottom - 6),
            ctx.fonts.small,
            COL_MUTED,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        let pcol = if is_exp { COL_AMBER } else { COL_CYAN };
        draw_pill(
            hdc,
            pill,
            pill_lbl,
            ctx.fonts.small_bold,
            if on { pcol } else { COL_SIDEBAR },
            if on { COL_SIDEBAR } else { COL_MUTED },
            if on { pcol } else { COL_BORDER },
        );
        hits.push((cr, target));
    };

    let runner_disp = ctx
        .settings
        .resolved_runner()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("victory.exe")
        .to_string();
    let dgpu_desc = format!("Sets Windows GpuPreference=2 for {runner_disp} on hybrid GPUs.");

    // ── Section 1: SYSTEM & HOST SCHEDULING (2-col grid) ────────────────────
    let s1_items: [(&str, &str, bool, bool, &str, Target); 6] = [
        ("FORCE DEDICATED GPU (HIGH PERF)", &dgpu_desc, ctx.settings.prefer_dgpu, false, if ctx.settings.prefer_dgpu { "ON" } else { "OFF" }, Target::TogglePreferDgpu),
        ("HIGH PRIORITY & NO THROTTLING", "Boosts active runner to High priority and disables EcoQoS.", ctx.settings.high_priority, false, if ctx.settings.high_priority { "ON" } else { "OFF" }, Target::ToggleHighPriority),
        ("ZERO-OVERHEAD PLAY MODE", "Auto-minimizes Game Master & pauses UI polling while playing.", ctx.settings.zero_overhead, false, if ctx.settings.zero_overhead { "ON" } else { "OFF" }, Target::ToggleZeroOverhead),
        ("ISOLATED PER-RUNNER CACHE", "Per-fork VICTORY_CACHE_DIR (keep OFF to share global cache).", ctx.settings.isolated_cache, false, if ctx.settings.isolated_cache { "ON" } else { "OFF" }, Target::ToggleIsolatedCache),
        ("FAST CORES", "Pin submit & relay threads to performance P-cores.", ctx.settings.fast_cores, false, if ctx.settings.fast_cores { "ON" } else { "OFF" }, Target::ToggleFastCores),
        ("1MS TIMER", "High-resolution Windows 1ms multimedia frame pacing.", ctx.settings.timer_1ms, false, if ctx.settings.timer_1ms { "ON" } else { "OFF" }, Target::ToggleTimer1ms),
    ];
    let s1_on = s1_items.iter().filter(|x| x.2).count();
    draw_sec_hdr(hdc, 0, "SECTION 1 · SYSTEM & HOST SCHEDULING", s1_on, s1_items.len(), y, &mut card_hits);
    y += 34;
    if !ctx.adv_collapsed[0] {
        let c2_gap = 12;
        let c2_w = (inner_w - c2_gap) / 2;
        for (i, (t, d, on, exp, plbl, tgt)) in s1_items.iter().enumerate() {
            let col = (i % 2) as i32;
            let row = (i / 2) as i32;
            let cx = left_x + col * (c2_w + c2_gap);
            let cy = y + row * (row_h + row_gap);
            let cr = RECT::new(cx, cy, cx + c2_w, cy + row_h);
            draw_adv_card(hdc, cr, t, d, *on, *exp, plbl, *tgt, &mut card_hits);
            if *tgt == Target::ToggleIsolatedCache && ctx.settings.isolated_cache {
                let seed_r = RECT::new(cr.right - 198, cr.top + 8, cr.right - 106, cr.top + 28);
                draw_button(hdc, seed_r, "SEED CACHE", ctx.fonts.small_bold, ctx.hover == Target::SeedIsolatedCache, COL_EMERALD, Target::SeedIsolatedCache, &mut card_hits);
            }
        }
        y += 3 * (row_h + row_gap) + 6;
    }

    let c3_gap = 10;
    let c3_w = (inner_w - c3_gap * 2) / 3;

    // ── Section 2: VRAM, TEXTURES & BUFFERS (3-col grid) ────────────────────
    let buf_lbl = ctx.settings.card_buffers_label();
    let buf_on = !ctx.settings.card_buffers_min_mb.trim().is_empty();
    let tex_lbl = ctx.settings.texture_room_label();
    let q2l_lbl = if ctx.settings.queue2_landing_mb.is_empty() { "96 MB" } else { ctx.settings.queue2_landing_mb.as_str() };
    let s2_items: [(&str, &str, bool, bool, &str, Target); 8] = [
        ("VRAM BUF", "Min GPU buffer scan size", buf_on, false, buf_lbl.as_str(), Target::CycleCardBuffers),
        ("TEXTURE ROOM %", "VICTORY_TEXTURE_ROOM_PERCENT cap", true, false, tex_lbl.as_str(), Target::CycleTextureRoom),
        ("TEX NO WAIT", "Skip waits on title texture memory", ctx.settings.texture_no_wait, false, if ctx.settings.texture_no_wait { "ON" } else { "OFF" }, Target::ToggleTextureNoWait),
        ("NO RESIDENCY WAIT", "VICTORY_WAIT_RESIDENT=0 (no stalls)", ctx.settings.no_wait_resident, false, if ctx.settings.no_wait_resident { "ON" } else { "OFF" }, Target::ToggleNoWaitResident),
        ("JOIN COPIES", "Merges small GPU copy ranges", ctx.settings.join_copies, false, if ctx.settings.join_copies { "ON" } else { "OFF" }, Target::ToggleJoinCopies),
        ("QUEUE2 (3)", "Second copy queue for textures", ctx.settings.queue2, false, if ctx.settings.queue2 { "ON" } else { "OFF" }, Target::ToggleQueue2),
        ("QUEUE2 LAND", "Copy-queue landing buffer MB", !ctx.settings.queue2_landing_mb.is_empty(), false, q2l_lbl, Target::CycleQueue2Landing),
        ("INPLACE BUF", "Experimental: in-place title buffer mapping", ctx.settings.inplace_buf, true, if ctx.settings.inplace_buf { "ON" } else { "OFF" }, Target::ToggleInplaceBuf),
    ];
    let s2_on = s2_items.iter().filter(|x| x.2).count();
    draw_sec_hdr(hdc, 1, "SECTION 2 · VRAM, TEXTURES & BUFFERS", s2_on, s2_items.len(), y, &mut card_hits);
    y += 34;
    if !ctx.adv_collapsed[1] {
        for (i, (t, d, on, exp, plbl, tgt)) in s2_items.iter().enumerate() {
            let col = (i % 3) as i32;
            let row = (i / 3) as i32;
            let cr = RECT::new(left_x + col * (c3_w + c3_gap), y + row * (row_h + row_gap), left_x + col * (c3_w + c3_gap) + c3_w, y + row * (row_h + row_gap) + row_h);
            draw_adv_card(hdc, cr, t, d, *on, *exp, plbl, *tgt, &mut card_hits);
        }
        y += ((s2_items.len() + 2) / 3) as i32 * (row_h + row_gap) + 6;
    }

    // ── Section 3: GPU PIPELINE & PARALLELISM (3-col grid) ──────────────────
    let relay_lbl = ctx.settings.relay_label();
    let wait_lbl = if ctx.settings.pipeline_wait_ms > 0 { format!("{}ms", ctx.settings.pipeline_wait_ms) } else { "OFF".into() };
    let s3_items: [(&str, &str, bool, bool, &str, Target); 11] = [
        ("ASYNC RELAY", "Sends work to GPU before waiting", ctx.settings.relay_mode > 0, false, relay_lbl, Target::CycleRelayMode),
        ("ASYNC SUBMIT", "Submit shader work without blocking", ctx.settings.async_submit, false, if ctx.settings.async_submit { "ON" } else { "OFF" }, Target::ToggleAsyncSubmit),
        ("PIPE WAIT", "Per-frame pipeline build budget", ctx.settings.pipeline_wait_ms > 0, false, wait_lbl.as_str(), Target::CyclePipelineWait),
        ("WIDE BACK", "Reads GPU results in parallel", ctx.settings.wide_back, false, if ctx.settings.wide_back { "ON" } else { "OFF" }, Target::ToggleWideBack),
        ("WIDE REFRESH", "8-thread parallel copy refreshes", ctx.settings.wide_refresh, false, if ctx.settings.wide_refresh { "ON" } else { "OFF" }, Target::ToggleWideRefresh),
        ("WIDE MERGE", "8-thread parallel GPU writeback", ctx.settings.wide_merge, false, if ctx.settings.wide_merge { "ON" } else { "OFF" }, Target::ToggleWideMerge),
        ("GPU REHASH", "Batches repeated GPU hash passes", ctx.settings.card_rehash_runs, false, if ctx.settings.card_rehash_runs { "ON" } else { "OFF" }, Target::ToggleCardRehash),
        ("INDIRECT WINDOW", "Batch indirect draws in one window", ctx.settings.indirect_window, false, if ctx.settings.indirect_window { "ON" } else { "OFF" }, Target::ToggleIndirectWindow),
        ("SHADER MERGE", "Shadow merge gap 64 / runs 4096", ctx.settings.merge_passes, false, if ctx.settings.merge_passes { "ON" } else { "OFF" }, Target::ToggleMergePasses),
        ("THIN LISTS", "Parallel draw-call translation", ctx.settings.thin_lists, false, if ctx.settings.thin_lists { "ON" } else { "OFF" }, Target::ToggleThinLists),
        ("HOIST CLAMPS", "Hoist indirect clamps (GPU cut)", ctx.settings.hoist_clamps, false, if ctx.settings.hoist_clamps { "ON" } else { "OFF" }, Target::ToggleHoistClamps),
    ];
    let s3_on = s3_items.iter().filter(|x| x.2).count();
    draw_sec_hdr(hdc, 2, "SECTION 3 · GPU PIPELINE & PARALLELISM", s3_on, s3_items.len(), y, &mut card_hits);
    y += 34;
    if !ctx.adv_collapsed[2] {
        for (i, (t, d, on, exp, plbl, tgt)) in s3_items.iter().enumerate() {
            let col = (i % 3) as i32;
            let row = (i / 3) as i32;
            let cr = RECT::new(left_x + col * (c3_w + c3_gap), y + row * (row_h + row_gap), left_x + col * (c3_w + c3_gap) + c3_w, y + row * (row_h + row_gap) + row_h);
            draw_adv_card(hdc, cr, t, d, *on, *exp, plbl, *tgt, &mut card_hits);
        }
        y += ((s3_items.len() + 2) / 3) as i32 * (row_h + row_gap) + 6;
    }

    // ── Section 4: DISPLAY, VISUALS & HUD (3-col grid + Upscaler/Loop row) ──
    let disp_lbl = ctx.settings.display_mode_label();
    let hud_sub = if ctx.settings.hud_overlay { ctx.settings.hud_corner_label() } else { "Titlebar stats active" };
    let s4_items: [(&str, &str, bool, bool, &str, Target); 5] = [
        ("DISPLAY CAP", "4K / 1080P / 900P / 720P output", true, false, disp_lbl, Target::CycleDisplayMode),
        ("AMBIENT SHADE", "GI shade fill + crease AO", ctx.settings.ambient_fill, false, if ctx.settings.ambient_fill { "ON" } else { "OFF" }, Target::ToggleAmbientFill),
        ("BVH BUILDER", "Ray-trace BVH acceleration", ctx.settings.bvh_builder, true, if ctx.settings.bvh_builder { "ON" } else { "OFF" }, Target::ToggleBvhBuilder),
        ("FLOAT HUD", hud_sub, ctx.settings.hud_overlay, false, if ctx.settings.hud_overlay { "ON" } else { "OFF" }, Target::ToggleHudOverlay),
        ("CENTER RETICLE", "Center dot + auto menu cursor", ctx.settings.kbm_crosshair, false, if ctx.settings.kbm_crosshair { "ON" } else { "OFF" }, Target::ToggleKbmCrosshair),
    ];
    let s4_on = s4_items.iter().filter(|x| x.2).count();
    draw_sec_hdr(hdc, 3, "SECTION 4 · DISPLAY, VISUALS & HUD", s4_on, s4_items.len(), y, &mut card_hits);
    y += 34;
    if !ctx.adv_collapsed[3] {
        for (i, (t, d, on, exp, plbl, tgt)) in s4_items.iter().enumerate() {
            let col = (i % 3) as i32;
            let row = (i / 3) as i32;
            let cr = RECT::new(left_x + col * (c3_w + c3_gap), y + row * (row_h + row_gap), left_x + col * (c3_w + c3_gap) + c3_w, y + row * (row_h + row_gap) + row_h);
            draw_adv_card(hdc, cr, t, d, *on, *exp, plbl, *tgt, &mut card_hits);
        }
        y += ((s4_items.len() + 2) / 3) as i32 * (row_h + row_gap);

        let c2_gap = 12;
        let c2_w = (inner_w - c2_gap) / 2;
        let last_h = 68;
        let up_card = RECT::new(left_x, y, left_x + c2_w, y + last_h);
        rounded_rect(hdc, up_card, COL_PANEL_ALT, COL_BORDER, 11);
        draw_text(hdc, "HOST UPSCALER DLL FOLDER", RECT::new(up_card.left + 12, up_card.top + 6, up_card.right - 12, up_card.top + 24), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let up_path = if ctx.settings.upscaler_dir.is_empty() { "(None — click Browse)" } else { &ctx.settings.upscaler_dir };
        draw_text(hdc, up_path, RECT::new(up_card.left + 12, up_card.top + 26, up_card.right - 190, up_card.bottom - 8), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_PATH_ELLIPSIS);
        let cl_up = RECT::new(up_card.right - 80, up_card.top + (last_h - 28) / 2, up_card.right - 10, up_card.top + (last_h + 28) / 2);
        draw_button(hdc, cl_up, "CLEAR", ctx.fonts.small_bold, ctx.hover == Target::ClearUpscalerDir, COL_MUTED, Target::ClearUpscalerDir, &mut card_hits);
        let br_up = RECT::new(cl_up.left - 98, cl_up.top, cl_up.left - 8, cl_up.bottom);
        draw_button(hdc, br_up, "BROWSE...", ctx.fonts.small_bold, ctx.hover == Target::BrowseUpscalerDir, COL_CYAN, Target::BrowseUpscalerDir, &mut card_hits);

        let sl_card = RECT::new(left_x + c2_w + c2_gap, y, left_x + inner_w, y + last_h);
        rounded_rect(hdc, sl_card, COL_PANEL_ALT, COL_BORDER, 11);
        draw_text(hdc, "SHADER LOOP LIMIT (VICTORY_SHADER_LOOP_LIMIT)", RECT::new(sl_card.left + 12, sl_card.top + 6, sl_card.right - 12, sl_card.top + 24), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let seg_gap = 6;
        let seg_w = ((sl_card.width() - 24 - seg_gap * 5) / 6).max(44);
        for (idx, preset) in SHADER_PRESETS.iter().enumerate() {
            let sx = sl_card.left + 12 + idx as i32 * (seg_w + seg_gap);
            let sr = RECT::new(sx, sl_card.top + 28, sx + seg_w, sl_card.bottom - 10);
            let selected = (idx == 0 && ctx.settings.shader_loop_limit.trim().is_empty())
                || ctx.settings.shader_loop_limit.trim() == *preset;
            let hovered = ctx.hover == Target::SetShaderLimit(idx);
            rounded_rect(
                hdc,
                sr,
                if selected { COL_CYAN_BG } else if hovered { COL_PANEL_HOVER } else { COL_SIDEBAR },
                if selected { COL_CYAN } else { COL_BORDER },
                8,
            );
            draw_text(hdc, preset, sr, ctx.fonts.small_bold, if selected { COL_CYAN } else { COL_TEXT }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            card_hits.push((sr, Target::SetShaderLimit(idx)));
        }
        y += last_h + 10;
    }

    // ── Section 5: DIAGNOSTICS & DEBUG (collapsed by default) ───────────────
    let s5_items: [(&str, &str, bool, bool, &str, Target); 3] = [
        ("D3D12 DRED", "GPU fault breadcrumbs", ctx.settings.dred, false, if ctx.settings.dred { "ON" } else { "OFF" }, Target::ToggleDred),
        ("GPU TIMING", "Per-pass GPU ms telemetry", ctx.settings.gpu_timing, false, if ctx.settings.gpu_timing { "ON" } else { "OFF" }, Target::ToggleGpuTiming),
        ("CPU PROFILER", "Dump VICTORY_PROFILE=1 hotspots", ctx.settings.profiler_on, false, if ctx.settings.profiler_on { "ON" } else { "OFF" }, Target::ToggleProfiler),
    ];
    let s5_on = s5_items.iter().filter(|x| x.2).count();
    draw_sec_hdr(hdc, 4, "SECTION 5 · DIAGNOSTICS & DEBUG", s5_on, s5_items.len(), y, &mut card_hits);
    y += 34;
    if !ctx.adv_collapsed[4] {
        for (i, (t, d, on, exp, plbl, tgt)) in s5_items.iter().enumerate() {
            let col = (i % 3) as i32;
            let cr = RECT::new(left_x + col * (c3_w + c3_gap), y, left_x + col * (c3_w + c3_gap) + c3_w, y + row_h);
            draw_adv_card(hdc, cr, t, d, *on, *exp, plbl, *tgt, &mut card_hits);
        }
        y += row_h + 10;
    }

    // Clean vertical scrollbar on right edge
    let total_h = (y - start_y).max(1);
    let view_h = (clip_bot - clip_top).max(1);
    if total_h > view_h {
        let track_r = RECT::new(r.right - 12, clip_top + 4, r.right - 6, clip_bot - 4);
        rounded_rect(hdc, track_r, COL_SIDEBAR, COL_BORDER, 3);
        let track_h = track_r.height();
        let thumb_h = ((track_h * view_h) / total_h).clamp(28, track_h);
        let max_sc = (total_h - view_h).max(1);
        let thumb_y = track_r.top + ((track_h - thumb_h) * (ctx.tuning_scroll as i32).min(max_sc)) / max_sc;
        let thumb_r = RECT::new(track_r.left, thumb_y, track_r.right, thumb_y + thumb_h);
        rounded_rect(hdc, thumb_r, COL_CYAN, COL_CYAN, 3);
    }

    hits.extend(card_hits.into_iter().filter(|(rc, _)| rc.bottom > clip_top && rc.top < clip_bot));
    RestoreDC(hdc, saved_dc);
}

unsafe fn draw_logs(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    rounded_rect(hdc, r, COL_PANEL, COL_BORDER, 16);
    draw_text(hdc, "LIVE LOGS", RECT::new(r.left + 24, r.top + 18, (r.right - 760).max(r.left + 140), r.top + 50), ctx.fonts.heading, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, "Scroll with mouse wheel. Select a session log on the left or export a local system report.", RECT::new(r.left + 24, r.top + 50, r.left + 580, r.top + 70), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    let b3 = RECT::new(r.right - 178, r.top + 22, r.right - 20, r.top + 56);
    draw_button(hdc, b3, "EXPORT REPORT", ctx.fonts.small_bold, ctx.hover == Target::ExportReport, COL_ORANGE, Target::ExportReport, hits);
    let b2 = RECT::new(b3.left - 146, r.top + 22, b3.left - 10, r.top + 56);
    draw_button(hdc, b2, "OPEN LOGS DIR", ctx.fonts.small_bold, ctx.hover == Target::OpenLogsDir, COL_CYAN, Target::OpenLogsDir, hits);
    let b1 = RECT::new(b2.left - 122, r.top + 22, b2.left - 8, r.top + 56);
    draw_button(hdc, b1, "OPEN LOG", ctx.fonts.small_bold, ctx.hover == Target::OpenCurrentLog, COL_TEXT, Target::OpenCurrentLog, hits);
    let b_ai = RECT::new(b1.left - 140, r.top + 22, b1.left - 8, r.top + 56);
    draw_button(hdc, b_ai, "AI DEBUG BUNDLE", ctx.fonts.small_bold, ctx.hover == Target::ExportAiBundle, COL_PINK, Target::ExportAiBundle, hits);
    let b_csv = RECT::new(b_ai.left - 116, r.top + 22, b_ai.left - 8, r.top + 56);
    draw_button(hdc, b_csv, "SESSIONS CSV", ctx.fonts.small_bold, ctx.hover == Target::OpenSessionsCsv, COL_EMERALD, Target::OpenSessionsCsv, hits);
    let b_cln = RECT::new(b_csv.left - 108, r.top + 22, b_csv.left - 8, r.top + 56);
    draw_button(hdc, b_cln, "CLEAN LOGS", ctx.fonts.small_bold, ctx.hover == Target::CleanOldLogs, COL_CORAL, Target::CleanOldLogs, hits);

    draw_line(hdc, r.left + 24, r.top + 80, r.right - 24, r.top + 80, COL_BORDER, 1);

    // Split: Left recent logs list (240px) | Right terminal viewer
    let list_r = RECT::new(r.left + 20, r.top + 92, r.left + 264, r.bottom - 18);
    let term_r = RECT::new(list_r.right + 14, r.top + 92, r.right - 20, r.bottom - 18);

    rounded_rect(hdc, list_r, COL_SIDEBAR, COL_BORDER, 12);
    draw_text(hdc, "RECENT LOGS", RECT::new(list_r.left + 12, list_r.top + 10, list_r.right - 12, list_r.top + 28), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    if ctx.recent_logs.is_empty() {
        draw_text(hdc, "No logs yet.\nPress LAUNCH or\nPREPARE to start.", RECT::new(list_r.left + 14, list_r.top + 42, list_r.right - 14, list_r.top + 120), ctx.fonts.small, COL_DIM, DT_LEFT | DT_WORDBREAK);
    } else {
        for (i, entry) in ctx.recent_logs.iter().enumerate() {
            let iy = list_r.top + 36 + i as i32 * 44;
            if iy + 40 > list_r.bottom - 110 {
                break;
            }
            let ir = RECT::new(list_r.left + 8, iy, list_r.right - 8, iy + 38);
            let sel = i == ctx.selected_log;
            let hov = ctx.hover == Target::SelectLog(i);
            rounded_rect(
                hdc,
                ir,
                if sel { COL_CYAN_BG } else if hov { COL_PANEL_HOVER } else { COL_PANEL },
                if sel { COL_CYAN } else { COL_BORDER },
                8,
            );
            draw_text(hdc, &entry.name, RECT::new(ir.left + 8, ir.top + 2, ir.right - 8, ir.top + 20), ctx.fonts.small_bold, if sel { COL_WHITE } else { COL_TEXT }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            draw_text(hdc, &entry.size_label, RECT::new(ir.left + 8, ir.top + 20, ir.right - 8, ir.bottom - 2), ctx.fonts.small, COL_DIM, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            hits.push((ir, Target::SelectLog(i)));
        }
    }

    if let Some(best) = best_session(ctx.recent_sessions) {
        let br2 = RECT::new(list_r.left + 8, list_r.bottom - 102, list_r.right - 8, list_r.bottom - 58);
        if br2.top > list_r.top + 120 {
            rounded_rect(hdc, br2, COL_PANEL_ALT, COL_CYAN, 8);
            draw_text(hdc, &format!("BEST: {} ({}s)", best.runner, best.duration_secs), RECT::new(br2.left + 8, br2.top + 4, br2.right - 8, br2.top + 22), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            draw_text(hdc, &format!("{} · {}", best.peak_fps, best.console), RECT::new(br2.left + 8, br2.top + 22, br2.right - 8, br2.bottom - 4), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
    }
    if let Some(last) = ctx.recent_sessions.first() {
        let br = RECT::new(list_r.left + 8, list_r.bottom - 52, list_r.right - 8, list_r.bottom - 8);
        rounded_rect(hdc, br, COL_PANEL_ALT, COL_EMERALD, 8);
        draw_text(hdc, &format!("LAST: {} ({}s)", last.runner, last.duration_secs), RECT::new(br.left + 8, br.top + 4, br.right - 8, br.top + 22), ctx.fonts.small_bold, COL_EMERALD, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        draw_text(hdc, &format!("{} · {}", last.exit_code, last.peak_fps), RECT::new(br.left + 8, br.top + 22, br.right - 8, br.bottom - 4), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    }

    // Terminal area (with optional GPU timing summary strip)
    let mut term_r = term_r;
    if let Some(gpu_sum) = summarize_gpu_timings(ctx.log_lines) {
        let strip = RECT::new(term_r.left, term_r.top, term_r.right, term_r.top + 30);
        rounded_rect(hdc, strip, COL_CYAN_BG, COL_CYAN, 8);
        draw_text(hdc, &gpu_sum, RECT::new(strip.left + 12, strip.top, strip.right - 12, strip.bottom), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        term_r.top += 38;
    }
    if !ctx.slow_shaders.is_empty() || !ctx.settings.skip_pipelines.trim().is_empty() {
        let bar = RECT::new(term_r.left, term_r.top, term_r.right, term_r.top + 34);
        rounded_rect(hdc, bar, COL_PANEL_ALT, COL_BORDER, 8);
        draw_text(hdc, "SKIP SHADERS:", RECT::new(bar.left + 10, bar.top, bar.left + 112, bar.bottom), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let clr_r = RECT::new(bar.right - 88, bar.top + 4, bar.right - 6, bar.bottom - 4);
        draw_button(hdc, clr_r, "CLEAR", ctx.fonts.small_bold, ctx.hover == Target::ClearSkipShaders, COL_MUTED, Target::ClearSkipShaders, hits);
        let mut sx = bar.left + 114;
        for (si, sh) in ctx.slow_shaders.iter().enumerate() {
            let pw = 116;
            if sx + pw > clr_r.left - 6 {
                break;
            }
            let pr = RECT::new(sx, bar.top + 4, sx + pw, bar.bottom - 4);
            let skipped = ctx.settings.is_hash_skipped(&sh.hash);
            let short_h: String = sh.hash.chars().take(8).collect();
            let plbl = if let Some(ms) = sh.ms {
                format!("{short_h} {:.0}ms", ms)
            } else {
                short_h
            };
            let hov = ctx.hover == Target::ToggleSkipShader(si);
            rounded_rect(hdc, pr, if skipped { COL_CORAL_BG } else if hov { COL_PANEL_HOVER } else { COL_SIDEBAR }, if skipped { COL_CORAL } else { COL_BORDER }, 6);
            draw_text(hdc, &plbl, pr, ctx.fonts.small_bold, if skipped { COL_CORAL } else { COL_TEXT }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            hits.push((pr, Target::ToggleSkipShader(si)));
            sx += pw + 6;
        }
        term_r.top += 42;
    }
    // Log filter strip (ALL / ERRORS / WARNS / FPS) + COPY TAIL
    let fbar = RECT::new(term_r.left, term_r.top, term_r.right, term_r.top + 30);
    let flabels = ["ALL LINES", "ERRORS", "WARNINGS", "FPS / GPU"];
    let mut fx = fbar.left;
    for (fi, flbl) in flabels.iter().enumerate() {
        let fw = 92;
        let fr = RECT::new(fx, fbar.top, fx + fw, fbar.bottom - 2);
        let sel = ctx.log_filter == fi;
        let hov = ctx.hover == Target::SetLogFilter(fi);
        rounded_rect(hdc, fr, if sel { COL_CYAN_BG } else if hov { COL_PANEL_HOVER } else { COL_SIDEBAR }, if sel { COL_CYAN } else { COL_BORDER }, 6);
        draw_text(hdc, flbl, fr, ctx.fonts.small_bold, if sel { COL_CYAN } else { COL_MUTED }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        hits.push((fr, Target::SetLogFilter(fi)));
        fx += fw + 6;
    }
    let cp_r = RECT::new(fbar.right - 108, fbar.top, fbar.right, fbar.bottom - 2);
    draw_button(hdc, cp_r, "COPY TAIL", ctx.fonts.small_bold, ctx.hover == Target::CopyLogSlice, COL_EMERALD, Target::CopyLogSlice, hits);
    term_r.top += 34;

    rounded_rect(hdc, term_r, rgb(10, 8, 18), COL_BORDER, 12);
    let saved = SaveDC(hdc);
    IntersectClipRect(hdc, term_r.left + 10, term_r.top + 10, term_r.right - 10, term_r.bottom - 10);

    let filtered: Vec<&String> = ctx.log_lines.iter().filter(|l| {
        let lo = l.to_ascii_lowercase();
        match ctx.log_filter {
            1 => !crate::diagnostics::is_known_wave32_note(l) && (lo.contains("error") || lo.contains("panic") || lo.contains("failed") || lo.contains("exception") || lo.contains("removed")),
            2 => lo.contains("warn"),
            3 => lo.contains("fps") || lo.contains("ms/frame") || lo.contains("pipeline") || lo.contains("gpu"),
            _ => true,
        }
    }).collect();

    if filtered.is_empty() {
        let empty_msg = if ctx.log_lines.is_empty() { "Waiting for runner output..." } else { "No lines match the active filter." };
        draw_text(hdc, empty_msg, RECT::new(term_r.left + 16, term_r.top + 16, term_r.right - 16, term_r.top + 40), ctx.fonts.mono, COL_DIM, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    } else {
        let line_h = 19;
        let visible = ((term_r.height() - 24) / line_h).max(1) as usize;
        let total = filtered.len();
        let max_scroll = total.saturating_sub(visible);
        let scroll = ctx.log_scroll.min(max_scroll);
        // scroll = 0 means pinned to bottom (latest lines)
        let start = total.saturating_sub(visible + scroll);
        let end = (start + visible).min(total);
        for (row, line) in filtered[start..end].iter().enumerate() {
            let ly = term_r.top + 12 + row as i32 * line_h;
            let lo = line.to_ascii_lowercase();
            let col = if crate::diagnostics::is_known_wave32_note(line) {
                COL_CYAN
            } else if lo.contains("error") || lo.contains("panic") || lo.contains("1455") || lo.contains("failed") {
                COL_CORAL
            } else if lo.contains("warn") {
                COL_AMBER
            } else if line.starts_with("===") || lo.contains("ready") || lo.contains("present") {
                COL_CYAN
            } else {
                COL_TEXT
            };
            draw_text(hdc, line, RECT::new(term_r.left + 14, ly, term_r.right - 14, ly + line_h), ctx.fonts.mono, col, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
    }
    RestoreDC(hdc, saved);
}


unsafe fn draw_bench_input(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    rounded_rect(hdc, r, COL_PANEL, COL_BORDER, 16);
    let b_ai = RECT::new(r.right - 154, r.top + 18, r.right - 20, r.top + 52);
    draw_button(hdc, b_ai, "AI DEBUG BUNDLE", ctx.fonts.small_bold, ctx.hover == Target::ExportAiBundle, COL_PINK, Target::ExportAiBundle, hits);
    let b_csv = RECT::new(b_ai.left - 128, r.top + 18, b_ai.left - 8, r.top + 52);
    draw_button(hdc, b_csv, "SESSIONS CSV", ctx.fonts.small_bold, ctx.hover == Target::OpenSessionsCsv, COL_EMERALD, Target::OpenSessionsCsv, hits);
    let b_rst = RECT::new(b_csv.left - 134, r.top + 18, b_csv.left - 8, r.top + 52);
    draw_button(hdc, b_rst, "RESET KEYBINDS", ctx.fonts.small_bold, ctx.hover == Target::ResetKeybinds, COL_AMBER, Target::ResetKeybinds, hits);

    draw_text(hdc, "CONTROLS, KEYBINDS & BENCHMARKS", RECT::new(r.left + 24, r.top + 16, (b_rst.left - 10).max(r.left + 220), r.top + 46), ctx.fonts.heading, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, "Switch between Primary Controller and Keyboard/Mouse, click any keybind to remap, and compare runner FPS.", RECT::new(r.left + 24, r.top + 46, (b_rst.left - 10).max(r.left + 260), r.top + 66), ctx.fonts.small, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    draw_line(hdc, r.left + 24, r.top + 74, r.right - 24, r.top + 74, COL_BORDER, 1);

    let gap = 16;
    let left_w = ((r.width() - 48 - gap) * 50) / 100;
    let left_r = RECT::new(r.left + 20, r.top + 84, r.left + 20 + left_w, r.bottom - 16);
    let right_r = RECT::new(left_r.right + gap, r.top + 84, r.right - 20, r.bottom - 16);

    // LEFT PANE: Primary Controller / KBM Switcher + Interactive Click-to-Remap Keybind Matrix
    rounded_rect(hdc, left_r, COL_SIDEBAR, COL_BORDER, 12);
    let tb_w = (left_r.width() - 20 - 18) / 4;
    let mode_btn = RECT::new(left_r.left + 10, left_r.top + 10, left_r.left + 10 + tb_w, left_r.top + 36);
    draw_button(hdc, mode_btn, ctx.settings.input_mode_label(), ctx.fonts.small_bold, ctx.hover == Target::CycleInputMode, COL_PINK, Target::CycleInputMode, hits);
    let ma_btn = RECT::new(mode_btn.right + 6, left_r.top + 10, mode_btn.right + 6 + tb_w, left_r.top + 36);
    let ma_lbl = if ctx.settings.mouse_aim { "MOUSE: ON" } else { "MOUSE: OFF" };
    draw_button(hdc, ma_btn, ma_lbl, ctx.fonts.small_bold, ctx.hover == Target::ToggleMouseAim, if ctx.settings.mouse_aim { COL_EMERALD } else { COL_MUTED }, Target::ToggleMouseAim, hits);
    let sens_lbl = format!("SENS {}x", ctx.settings.mouse_sens);
    let sens_btn = RECT::new(ma_btn.right + 6, left_r.top + 10, ma_btn.right + 6 + tb_w, left_r.top + 36);
    draw_button(hdc, sens_btn, &sens_lbl, ctx.fonts.small_bold, ctx.hover == Target::CycleMouseSens, COL_CYAN, Target::CycleMouseSens, hits);
    let lock_btn = RECT::new(sens_btn.right + 6, left_r.top + 10, left_r.right - 10, left_r.top + 36);
    let lock_lbl = if ctx.settings.mouse_lock { "LOCK (F8)" } else { "FREE MOUSE" };
    draw_button(hdc, lock_btn, lock_lbl, ctx.fonts.small_bold, ctx.hover == Target::ToggleMouseLock, if ctx.settings.mouse_lock { COL_ORANGE } else { COL_MUTED }, Target::ToggleMouseLock, hits);

    let primary_pad = ctx.sys.xinput_pad_details.iter().find(|p| p.connected).copied().unwrap_or(ctx.sys.xinput_pad_details[0]);
    let kbm_forced = ctx.settings.input_mode == "kbm";
    let pad_card = RECT::new(left_r.left + 10, left_r.top + 44, left_r.right - 10, left_r.top + 96);
    let pad_active = primary_pad.connected && !kbm_forced;
    rounded_rect(
        hdc,
        pad_card,
        if pad_active { COL_EMERALD_BG } else { COL_PANEL },
        if pad_active { COL_EMERALD } else { COL_CYAN },
        10,
    );
    let dev_title = if kbm_forced {
        "KEYBOARD & MOUSE STAND-IN (FORCED ACTIVE)".to_string()
    } else if primary_pad.connected {
        format!("PRIMARY CONTROLLER (XINPUT #{})  ·  {}", primary_pad.slot + 1, primary_pad.battery_label)
    } else {
        "KEYBOARD & MOUSE STAND-IN (AUTO — NO PAD DETECTED)".to_string()
    };
    draw_text(hdc, &dev_title, RECT::new(pad_card.left + 12, pad_card.top + 5, pad_card.right - 124, pad_card.top + 25), ctx.fonts.body_bold, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let dev_sub = if pad_active {
        format!("Buttons bitmask: 0x{:04X} · Driving GameInput Pad #1 (click MODE to force KBM)", primary_pad.buttons)
    } else {
        "WASD Move · IJKL Camera · Click any action below to customize keybind".into()
    };
    draw_text(hdc, &dev_sub, RECT::new(pad_card.left + 12, pad_card.top + 26, pad_card.right - 124, pad_card.bottom - 5), ctx.fonts.small, if pad_active { COL_EMERALD } else { COL_MUTED }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    if primary_pad.connected {
        let rb = RECT::new(pad_card.right - 114, pad_card.top + 10, pad_card.right - 10, pad_card.bottom - 10);
        draw_button(hdc, rb, "TEST RUMBLE", ctx.fonts.small_bold, ctx.hover == Target::RumblePad(primary_pad.slot), COL_CYAN, Target::RumblePad(primary_pad.slot), hits);
    }

    // Interactive Click-to-Remap Keybind Grid (2 columns x 7 rows = 14 actions)
    draw_text(hdc, "CUSTOMIZABLE KEYBOARD STAND-IN BINDS (CLICK ANY SLOT TO REMAP)", RECT::new(left_r.left + 14, pad_card.bottom + 8, left_r.right - 14, pad_card.bottom + 26), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let grid_top = pad_card.bottom + 30;
    let grid_avail_h = (left_r.bottom - grid_top - 8).max(160);
    let rows_n = 7i32;
    let cell_gap = 6i32;
    let cell_h = ((grid_avail_h - cell_gap * (rows_n - 1)) / rows_n).clamp(24, 38);
    let cell_w = (left_r.width() - 20 - cell_gap) / 2;

    for (idx, (code, label, _def)) in crate::config::KEYBIND_SLOTS.iter().enumerate() {
        let col_i = (idx % 2) as i32;
        let row_i = (idx / 2) as i32;
        let cx = left_r.left + 10 + col_i * (cell_w + cell_gap);
        let cy = grid_top + row_i * (cell_h + cell_gap);
        if cy + cell_h > left_r.bottom - 4 {
            break;
        }
        let cr = RECT::new(cx, cy, cx + cell_w, cy + cell_h);
        let tgt = Target::RemapKeybind(idx);
        let hov = ctx.hover == tgt;
        let listening = ctx.listening_keybind == Some(idx);
        rounded_rect(
            hdc,
            cr,
            if listening { COL_AMBER_BG } else if hov { COL_PANEL_HOVER } else { COL_PANEL },
            if listening { COL_AMBER } else if hov { COL_CYAN } else { COL_BORDER },
            8,
        );
        draw_text(hdc, label, RECT::new(cr.left + 10, cr.top, cr.right - 84, cr.bottom), ctx.fonts.small, if listening || hov { COL_WHITE } else { COL_TEXT }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let bound_key = if listening { "PRESS KEY...".to_string() } else { ctx.settings.get_keybind(code).to_ascii_uppercase() };
        let key_pill = RECT::new(cr.right - 80, cr.top + 3, cr.right - 6, cr.bottom - 3);
        draw_pill(
            hdc,
            key_pill,
            &bound_key,
            ctx.fonts.small_bold,
            if listening { COL_AMBER } else { COL_CYAN_BG },
            if listening { COL_SIDEBAR } else if hov { COL_WHITE } else { COL_CYAN },
            if listening { COL_AMBER } else { COL_CYAN },
        );
        hits.push((cr, tgt));
    }

    // RIGHT PANE: Shader Translator Census Card + Live FPS Curve + Runner Benchmark Leaderboard
    rounded_rect(hdc, right_r, COL_SIDEBAR, COL_BORDER, 12);
    let mut right_top = right_r.top + 10;

    // v10.0 Live Shader Translator Census Card + RUN CENSUS button
    let sc_r = RECT::new(right_r.left + 10, right_top, right_r.right - 10, right_top + 64);
    let sc_ok = ctx.shader_census.total > 0 && ctx.shader_census.refused == 0;
    rounded_rect(hdc, sc_r, if sc_ok { COL_EMERALD_BG } else { COL_PANEL }, if sc_ok { COL_EMERALD } else { COL_CYAN }, 10);
    let sc_btn = RECT::new(sc_r.right - 122, sc_r.top + 6, sc_r.right - 8, sc_r.top + 30);
    let sc_blbl = if ctx.census_running { "SCANNING..." } else { "RUN CENSUS" };
    draw_button(hdc, sc_btn, sc_blbl, ctx.fonts.small_bold, ctx.hover == Target::RunShaderCensus, COL_CYAN, Target::RunShaderCensus, hits);
    let sc_hdr = if ctx.shader_census.total > 0 {
        format!("SHADER TRANSLATOR CENSUS  ·  {} / {} TRANSLATED", ctx.shader_census.translated, ctx.shader_census.total)
    } else {
        format!("{} SHADER TRANSLATOR · RUN CENSUS TO VERIFY", runner_badge(ctx.settings))
    };
    draw_text(hdc, &sc_hdr, RECT::new(sc_r.left + 12, sc_r.top + 6, sc_btn.left - 8, sc_r.top + 26), ctx.fonts.small_bold, if sc_ok { COL_EMERALD } else { COL_CYAN }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, &ctx.shader_census.summary, RECT::new(sc_r.left + 12, sc_r.top + 28, sc_r.right - 12, sc_r.bottom - 4), ctx.fonts.small, COL_TEXT, DT_LEFT | DT_WORDBREAK);
    right_top = sc_r.bottom + 8;

    if ctx.fps_history.len() >= 2 {
        let gr = RECT::new(right_r.left + 10, right_top, right_r.right - 10, right_top + 64);
        rounded_rect(hdc, gr, COL_PANEL, COL_BORDER, 8);
        let cur_fps = *ctx.fps_history.last().unwrap_or(&0) as f64 / 10.0;
        let peak_f = ctx.fps_history.iter().copied().max().unwrap_or(0) as f64 / 10.0;
        let gtitle = format!("LIVE / LAST SESSION FPS CURVE  ·  NOW {:.1} FPS  ·  PEAK {:.1} FPS", cur_fps, peak_f);
        draw_text(hdc, &gtitle, RECT::new(gr.left + 10, gr.top + 4, gr.right - 10, gr.top + 22), ctx.fonts.small_bold, COL_EMERALD, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let scaled: Vec<u32> = ctx.fps_history.iter().map(|&v| ((v * 100) / 350).min(100)).collect();
        draw_sparkline(hdc, RECT::new(gr.left + 10, gr.top + 24, gr.right - 10, gr.bottom - 6), &scaled, COL_EMERALD);
        right_top = gr.bottom + 8;
    }
    if !ctx.runner_benches.is_empty() {
        draw_text(hdc, "RUNNER A/B HEAD-TO-HEAD SCORECARD", RECT::new(right_r.left + 14, right_top, right_r.right - 14, right_top + 20), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        right_top += 22;
        let card_n = ctx.runner_benches.len().min(3) as i32;
        let ab_w = ((right_r.width() - 20 - (card_n - 1) * 8) / card_n).max(110);
        for (bi, rb) in ctx.runner_benches.iter().take(3).enumerate() {
            let bx = right_r.left + 10 + bi as i32 * (ab_w + 8);
            let br = RECT::new(bx, right_top, bx + ab_w, right_top + 50);
            let lead = bi == 0;
            rounded_rect(hdc, br, if lead { COL_EMERALD_BG } else { COL_PANEL }, if lead { COL_EMERALD } else { COL_BORDER }, 8);
            let rhead = format!("{}{}", if lead { "★ " } else { "" }, rb.runner);
            draw_text(hdc, &rhead, RECT::new(br.left + 8, br.top + 4, br.right - 8, br.top + 22), ctx.fonts.small_bold, if lead { COL_EMERALD } else { COL_WHITE }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            let rsub = format!("Best {:.1} · Avg {:.1} FPS ({}x)", rb.best_fps, rb.avg_fps, rb.runs);
            draw_text(hdc, &rsub, RECT::new(br.left + 8, br.top + 24, br.right - 8, br.bottom - 4), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
        right_top += 56;
    }
    draw_text(hdc, "SESSION BENCHMARK LEADERBOARD", RECT::new(right_r.left + 14, right_top, right_r.right - 14, right_top + 20), ctx.fonts.small_bold, COL_ORANGE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    if ctx.recent_sessions.is_empty() {
        draw_text(hdc, "No recorded sessions yet.\n\nLaunch your title and play for a few seconds — Game Master 6 automatically logs FPS, duration, exit code, and slowest shader per runner.", RECT::new(right_r.left + 16, right_top + 26, right_r.right - 16, right_r.bottom - 16), ctx.fonts.body, COL_MUTED, DT_LEFT | DT_WORDBREAK);
        return;
    }

    let best_ts = best_session(ctx.recent_sessions).map(|b| b.timestamp).unwrap_or_default();
    let mut sy = right_top + 24;
    for (idx, rec) in ctx.recent_sessions.iter().enumerate() {
        if sy + 50 > right_r.bottom - 8 {
            break;
        }
        let sr = RECT::new(right_r.left + 10, sy, right_r.right - 10, sy + 46);
        let is_best = !best_ts.is_empty() && rec.timestamp == best_ts;
        rounded_rect(
            hdc,
            sr,
            if is_best { COL_EMERALD_BG } else if idx % 2 == 0 { COL_PANEL } else { COL_PANEL_ALT },
            if is_best { COL_EMERALD } else { COL_BORDER },
            10,
        );
        let head = format!(
            "{}{}  ·  {}  ·  {}s",
            if is_best { "★ BEST · " } else { "" },
            rec.runner,
            rec.console,
            rec.duration_secs
        );
        draw_text(hdc, &head, RECT::new(sr.left + 12, sr.top + 4, sr.right - 214, sr.top + 23), ctx.fonts.body_bold, if is_best { COL_EMERALD } else { COL_WHITE }, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let sub = format!("{}  ·  Exit: {}  ·  Top Shader: {}", rec.timestamp, rec.exit_code, rec.top_shader);
        draw_text(hdc, &sub, RECT::new(sr.left + 12, sr.top + 24, sr.right - 214, sr.bottom - 4), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

        let fps_r = RECT::new(sr.right - 206, sr.top + 8, sr.right - 10, sr.bottom - 8);
        draw_pill(hdc, fps_r, &rec.peak_fps, ctx.fonts.small_bold, COL_SIDEBAR, if is_best { COL_EMERALD } else { COL_CYAN }, if is_best { COL_EMERALD } else { COL_CYAN });
        sy += 50;
    }
}

unsafe fn draw_settings(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    rounded_rect(hdc, r, COL_PANEL, COL_BORDER, 16);

    let reset_r = RECT::new(r.right - 144, r.top + 20, r.right - 20, r.top + 54);
    draw_button(hdc, reset_r, "RESET DEFAULTS", ctx.fonts.small_bold, ctx.hover == Target::ResetDefaults, COL_CORAL, Target::ResetDefaults, hits);
    let open_r = RECT::new(reset_r.left - 146, r.top + 20, reset_r.left - 10, r.top + 54);
    draw_button(hdc, open_r, "OPEN .SETTINGS", ctx.fonts.small_bold, ctx.hover == Target::OpenSettingsFile, COL_CYAN, Target::OpenSettingsFile, hits);
    let th_r = RECT::new(open_r.left - 138, r.top + 20, open_r.left - 10, r.top + 54);
    draw_button(hdc, th_r, ctx.settings.theme_label(), ctx.fonts.small_bold, ctx.hover == Target::CycleTheme, COL_PINK, Target::CycleTheme, hits);

    draw_text(hdc, "SETTINGS", RECT::new(r.left + 24, r.top + 16, (th_r.left - 16).max(r.left + 160), r.top + 48), ctx.fonts.heading, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    draw_text(hdc, "All edits persist automatically to GameMaster.settings beside the application.", RECT::new(r.left + 24, r.top + 48, (th_r.left - 16).max(r.left + 320), r.top + 68), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

    draw_line(hdc, r.left + 24, r.top + 76, r.right - 24, r.top + 76, COL_BORDER, 1);

    let runner_disp = ctx.settings.resolved_runner().to_string_lossy().into_owned();
    let paths: [(&str, &str, Target); 3] = [
        ("Game Executable (exe)", &ctx.settings.exe, Target::BrowseExe),
        ("Game Data Folder (game_dir)", &ctx.settings.game_dir, Target::BrowseGameDir),
        ("Active Runner (runner)", &runner_disp, Target::BrowseRunner),
    ];
    let compact = r.height() < 500;
    let p_h = if compact { 40 } else { 46 };
    let p_step = if compact { 46 } else { 52 };
    let mut y = r.top + 84;
    for (lbl, val, target) in paths {
        let row = RECT::new(r.left + 24, y, r.right - 24, y + p_h);
        rounded_rect(hdc, row, COL_PANEL_ALT, COL_BORDER, 10);
        draw_text(hdc, lbl, RECT::new(row.left + 14, row.top + 3, row.left + 250, row.top + 20), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        draw_text(hdc, val, RECT::new(row.left + 14, row.top + 20, row.right - 130, row.bottom - 3), ctx.fonts.small, COL_TEXT, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_PATH_ELLIPSIS);
        let btn = RECT::new(row.right - 116, row.top + 6, row.right - 10, row.bottom - 6);
        draw_button(hdc, btn, "BROWSE...", ctx.fonts.small_bold, ctx.hover == target, COL_WHITE, target, hits);
        y += p_step;
    }

    let box_h = if compact { 64 } else { 74 };
    let half_w = (r.width() - 48 - 16) / 2;
    let c_box = RECT::new(r.left + 24, y + 4, r.left + 24 + half_w, y + 4 + box_h);
    rounded_rect(hdc, c_box, COL_PANEL_ALT, COL_BORDER, 12);
    draw_text(hdc, "CONSOLE TARGET PROFILE", RECT::new(c_box.left + 14, c_box.top + 6, c_box.right - 14, c_box.top + 24), ctx.fonts.small_bold, COL_PINK, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let sw = (c_box.width() - 38) / 2;
    for (idx, (name, is_s)) in [("XBOX SERIES X", false), ("XBOX SERIES S", true)].iter().enumerate() {
        let sr = RECT::new(c_box.left + 14 + idx as i32 * (sw + 10), c_box.top + 26, c_box.left + 14 + idx as i32 * (sw + 10) + sw, c_box.bottom - 8);
        let active = ctx.settings.is_series_s() == *is_s;
        let tgt = Target::SetConsole(*is_s);
        let hov = ctx.hover == tgt;
        rounded_rect(hdc, sr, if active { COL_CYAN_BG } else if hov { COL_PANEL_HOVER } else { COL_SIDEBAR }, if active { COL_CYAN } else { COL_BORDER }, 8);
        draw_text(hdc, name, sr, ctx.fonts.small_bold, if active { COL_CYAN } else { COL_MUTED }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        hits.push((sr, tgt));
    }

    let w_box = RECT::new(c_box.right + 16, y + 4, r.right - 24, y + 4 + box_h);
    rounded_rect(hdc, w_box, COL_PANEL_ALT, COL_BORDER, 12);
    draw_text(hdc, "STARTUP WINDOW MODE", RECT::new(w_box.left + 14, w_box.top + 6, w_box.right - 14, w_box.top + 24), ctx.fonts.small_bold, COL_ORANGE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let wr = RECT::new(w_box.left + 14, w_box.top + 26, w_box.right - 14, w_box.bottom - 8);
    let hov_w = ctx.hover == Target::ToggleStartMaximized;
    rounded_rect(hdc, wr, if ctx.settings.start_maximized { COL_CYAN_BG } else if hov_w { COL_PANEL_HOVER } else { COL_SIDEBAR }, if ctx.settings.start_maximized { COL_CYAN } else { COL_BORDER }, 8);
    let w_lbl = if ctx.settings.start_maximized { "OPEN MAXIMIZED (CLICK FOR WINDOWED)" } else { "OPEN WINDOWED (CLICK FOR MAXIMIZED)" };
    draw_text(hdc, w_lbl, wr, ctx.fonts.small_bold, if ctx.settings.start_maximized { COL_CYAN } else { COL_MUTED }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    hits.push((wr, Target::ToggleStartMaximized));

    let m_top = c_box.bottom + 8;
    let m_h = if compact { 44 } else { 50 };
    let m_gap = 10;
    let m_w = (r.width() - 48 - m_gap * 4) / 5;
    let tdr_lbl = if ctx.sys.tdr_safe() { "TDR: SAFE (15s)" } else { "FIX TDR 15s" };
    let off_lbl = if ctx.settings.offline_mode { "OFFLINE: ON" } else { "OFFLINE: OFF" };
    let sys_tools: [(&str, DWORD, Target); 5] = [
        (tdr_lbl, if ctx.sys.tdr_safe() { COL_EMERALD } else { COL_CORAL }, Target::FixTdrDelay),
        ("VIRTUAL MEM", COL_ORANGE, Target::OpenVirtualMem),
        (off_lbl, if ctx.settings.offline_mode { COL_EMERALD } else { COL_MUTED }, Target::ToggleOfflineMode),
        ("KILL STRAYS", COL_AMBER, Target::KillStrayRunners),
        ("DESKTOP SHORTCUT", COL_CYAN, Target::CreateDesktopShortcut),
    ];
    for (ti, (tlbl, tcol, ttgt)) in sys_tools.iter().enumerate() {
        let tx = r.left + 24 + ti as i32 * (m_w + m_gap);
        let tr = RECT::new(tx, m_top, tx + m_w, m_top + m_h);
        draw_button(hdc, tr, tlbl, ctx.fonts.small_bold, ctx.hover == *ttgt, *tcol, *ttgt, hits);
    }

    draw_text(hdc, "TDR 15s prevents Windows from resetting your GPU driver during slow 3D frames.", RECT::new(r.left + 24, m_top + m_h + 2, r.right - 24, m_top + m_h + 18), ctx.fonts.small, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    let l_top = m_top + m_h + 22;
    let l_box = RECT::new(r.left + 24, l_top, r.right - 24, (l_top + box_h).min(r.bottom - 10));
    if l_box.height() >= 48 {
        rounded_rect(hdc, l_box, COL_PANEL_ALT, COL_BORDER, 12);
        draw_text(hdc, "RUNNER LOG VERBOSITY (VICTORY_LOG)", RECT::new(l_box.left + 14, l_box.top + 6, l_box.right - 340, l_box.top + 24), ctx.fonts.small_bold, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let env_r = RECT::new(l_box.right - 156, l_box.top + 4, l_box.right - 12, l_box.top + 24);
        draw_button(hdc, env_r, "EDIT CUSTOM ENV", ctx.fonts.small_bold, ctx.hover == Target::EditCustomEnv, COL_PINK, Target::EditCustomEnv, hits);
        let arg_r = RECT::new(env_r.left - 148, l_box.top + 4, env_r.left - 8, l_box.top + 24);
        draw_button(hdc, arg_r, "EDIT TITLE ARGS", ctx.fonts.small_bold, ctx.hover == Target::EditTitleArgs, COL_ORANGE, Target::EditTitleArgs, hits);
        let lw = (l_box.width() - 28 - 30) / 4;
        for (idx, lvl) in LOG_LEVELS.iter().enumerate() {
            let lr = RECT::new(l_box.left + 14 + idx as i32 * (lw + 10), l_box.top + 28, l_box.left + 14 + idx as i32 * (lw + 10) + lw, l_box.bottom - 8);
            let active = ctx.settings.log_level == *lvl;
            let tgt = Target::SetLogLevel(idx);
            let hov = ctx.hover == tgt;
            rounded_rect(hdc, lr, if active { COL_CYAN_BG } else if hov { COL_PANEL_HOVER } else { COL_SIDEBAR }, if active { COL_CYAN } else { COL_BORDER }, 8);
            draw_text(hdc, &lvl.to_ascii_uppercase(), lr, ctx.fonts.small_bold, if active { COL_CYAN } else { COL_MUTED }, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            hits.push((lr, tgt));
        }
    }
}

unsafe fn draw_dock(hdc: HDC, r: RECT, ctx: &RenderContext, hits: &mut Vec<(RECT, Target)>) {
    let launch_w = 320;
    let feed_r = RECT::new(r.left, r.top, r.right - launch_w - 14, r.bottom);
    let deck_r = RECT::new(feed_r.right + 14, r.top, r.right, r.bottom);

    // Left: Live Activity & Console Feed (replaces the old dead empty space!)
    rounded_rect(hdc, feed_r, COL_DOCK, COL_BORDER, 16);
    let feed_title = if let Some(sess) = ctx.session {
        let secs = sess.started_at.elapsed().as_secs();
        let fps_seg = if let Some(ft) = sess.live_telemetry {
            format!(
                "  ·  {:.1} FPS ({:.0}ms | GPU {:.0}ms | {} draws, {} drop)",
                ft.fps, ft.frame_ms, ft.gpu_ms, ft.draws, ft.dropped_draws
            )
        } else {
            String::new()
        };
        if let Some(ps) = ctx.proc_stats {
            format!(
                "LIVE ({:02}:{:02}){}  ·  RAM {:.1}G / CMT {:.1}G (PK {:.1}G)",
                secs / 60,
                secs % 60,
                fps_seg,
                ps.working_set_gb,
                ps.private_commit_gb,
                ps.peak_ws_gb,
            )
        } else {
            format!("LIVE  ·  {} ({:02}:{:02}){}", sess.kind.label(), secs / 60, secs % 60, fps_seg)
        }
    } else if let Some(toast) = ctx.toast {
        format!("ACTIVITY  ·  {}", toast.to_ascii_uppercase())
    } else {
        "ACTIVITY FEED  ·  LOCAL BUILD — ALL FILES STAY ON THIS MACHINE".into()
    };
    draw_text(
        hdc,
        &feed_title,
        RECT::new(feed_r.left + 18, feed_r.top + 12, feed_r.right - 120, feed_r.top + 30),
        ctx.fonts.small_bold,
        if ctx.session.is_some() { COL_CYAN } else if ctx.toast.is_some() { COL_EMERALD } else { COL_PINK },
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );

    let open_log_btn = RECT::new(feed_r.right - 102, feed_r.top + 10, feed_r.right - 14, feed_r.top + 34);
    draw_button(hdc, open_log_btn, "VIEW LOGS", ctx.fonts.small_bold, ctx.hover == Target::ViewLogsQuick, COL_MUTED, Target::ViewLogsQuick, hits);
    let key_btn = RECT::new(open_log_btn.left - 148, feed_r.top + 10, open_log_btn.left - 8, feed_r.top + 34);
    let pad_lbl = if let Some(p) = ctx.sys.xinput_pad_details.iter().find(|p| p.connected) {
        format!("PAD #{} ({})", p.slot + 1, p.battery_label)
    } else {
        "PAD / KEYMAP".into()
    };
    draw_button(hdc, key_btn, &pad_lbl, ctx.fonts.small_bold, ctx.hover == Target::ShowKeymap, if ctx.sys.xinput_pads > 0 { COL_EMERALD } else { COL_CYAN }, Target::ShowKeymap, hits);

    draw_line(hdc, feed_r.left + 16, feed_r.top + 40, feed_r.right - 16, feed_r.top + 40, COL_BORDER, 1);

    if let Some(sess) = ctx.session {
        draw_text(hdc, &sess.phase, RECT::new(feed_r.left + 18, feed_r.top + 46, feed_r.right - 18, feed_r.top + 66), ctx.fonts.body_bold, COL_WHITE, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        let tail_start = ctx.log_lines.len().saturating_sub(2);
        for (i, line) in ctx.log_lines[tail_start..].iter().enumerate() {
            let ly = feed_r.top + 70 + i as i32 * 22;
            draw_text(hdc, line, RECT::new(feed_r.left + 18, ly, feed_r.right - 18, ly + 20), ctx.fonts.mono, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
    } else if !ctx.log_lines.is_empty() {
        let tail_start = ctx.log_lines.len().saturating_sub(3);
        for (i, line) in ctx.log_lines[tail_start..].iter().enumerate() {
            let ly = feed_r.top + 48 + i as i32 * 23;
            draw_text(hdc, line, RECT::new(feed_r.left + 18, ly, feed_r.right - 18, ly + 21), ctx.fonts.mono, COL_MUTED, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
    } else if let Some(warn) = ctx.audit.warnings.first().filter(|w| Some(w.as_str()) != ctx.dismissed_warning) {
        draw_text(
            hdc,
            warn,
            RECT::new(feed_r.left + 18, feed_r.top + 48, feed_r.right - 106, feed_r.bottom - 10),
            ctx.fonts.body,
            COL_AMBER,
            DT_LEFT | DT_WORDBREAK,
        );
        draw_button(
            hdc,
            RECT::new(feed_r.right - 90, feed_r.bottom - 32, feed_r.right - 16, feed_r.bottom - 10),
            "DISMISS",
            ctx.fonts.small_bold,
            ctx.hover == Target::DismissWarning,
            COL_AMBER,
            Target::DismissWarning,
            hits,
        );
    } else {
        draw_text(hdc, "System ready. Press LAUNCH TITLE (or Ctrl+L) to start, or PREPARE CACHE before your first session.", RECT::new(feed_r.left + 18, feed_r.top + 52, feed_r.right - 18, feed_r.bottom - 14), ctx.fonts.body, COL_MUTED, DT_LEFT | DT_WORDBREAK);
    }

    // Right: Launch Command Deck
    rounded_rect(hdc, deck_r, COL_PANEL, COL_BORDER, 16);
    let btn_r = RECT::new(deck_r.left + 16, deck_r.top + 14, deck_r.right - 16, deck_r.top + 72);
    let active = ctx.session.is_some();
    let hovered = ctx.hover == Target::LaunchOrStop;

    if active {
        fill_gradient(hdc, btn_r, COL_CORAL, rgb(180, 32, 64), false);
    } else {
        fill_gradient(hdc, btn_r, COL_PINK, COL_ORANGE, false);
    }
    if hovered {
        let br = CreateSolidBrush(COL_WHITE);
        FrameRect(hdc, &btn_r, br);
        DeleteObject(br);
    }
    let btn_label = if active { "STOP RUNNER" } else { "LAUNCH TITLE" };
    draw_text(hdc, btn_label, btn_r, ctx.fonts.title, COL_WHITE, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    hits.push((btn_r, Target::LaunchOrStop));

    // Real progress bar or smooth animated activity bar
    let track = RECT::new(deck_r.left + 16, deck_r.top + 84, deck_r.right - 16, deck_r.top + 94);
    rounded_rect(hdc, track, rgb(34, 26, 54), COL_BORDER, 5);
    if let Some(sess) = ctx.session {
        if let Some(pct) = sess.parsed_progress {
            draw_progress_bar(hdc, track, pct, COL_CYAN);
            draw_text(hdc, &format!("{pct}%  ·  {}", sess.kind.label()), RECT::new(deck_r.left + 16, deck_r.top + 98, deck_r.right - 16, deck_r.bottom - 8), ctx.fonts.small, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        } else {
            // Smooth indeterminate neon pulse
            let span = (track.width() / 3).max(40);
            let travel = (track.width() - span).max(1);
            let pos = (ctx.anim_tick.wrapping_mul(14) % (travel as u32 * 2)) as i32;
            let offset = if pos <= travel { pos } else { travel * 2 - pos };
            let seg = RECT::new(track.left + offset, track.top + 1, track.left + offset + span, track.bottom - 1);
            fill_gradient(hdc, seg, COL_CYAN, COL_PINK, false);
            draw_text(hdc, "RUNNING  ·  STREAMING LOGS", RECT::new(deck_r.left + 16, deck_r.top + 98, deck_r.right - 16, deck_r.bottom - 8), ctx.fonts.small, COL_CYAN, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
    } else {
        let console = if ctx.settings.is_series_s() { "SERIES S" } else { "SERIES X" };
        let hud = if ctx.settings.hud_overlay { "ON" } else { "OFF" };
        let hint = format!("{console} · F9 HUD {hud} · CTRL+L");
        draw_text(hdc, &hint, RECT::new(deck_r.left + 16, deck_r.top + 98, deck_r.right - 16, deck_r.bottom - 8), ctx.fonts.small, COL_DIM, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
    }
}

/// Strictly matches a local discovered runner against a PortWatch Live Feed project.
/// Prevents `runners\baseline\victory.exe` from falsely matching upstream `Marvisak/VIctory`
/// via substring path matching on `victory`.
fn matches_portwatch_project(
    r: &crate::runners::DiscoveredRunner,
    runners_root: &std::path::Path,
    name_lc: &str,
    slug_lc: &str,
    repo_lc: &str,
) -> bool {
    let rr = r.repo.to_ascii_lowercase();
    if !repo_lc.is_empty() && rr == repo_lc {
        return true;
    }
    if r.kind == crate::runners::RunnerKind::Baseline && repo_lc == "marvisak/victory" {
        return false;
    }
    let parent_folder = r
        .path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let in_runners_dir = r
        .path
        .parent()
        .and_then(|p| p.parent())
        .map(|gp| gp == runners_root)
        .unwrap_or(false);
    in_runners_dir
        && ((!slug_lc.is_empty() && parent_folder == slug_lc)
            || (!name_lc.is_empty() && parent_folder == name_lc))
}

#[cfg(test)]
mod tests {
    use super::matches_portwatch_project;
    use crate::runners::{DiscoveredRunner, RunnerKind};
    use std::path::PathBuf;

    #[test]
    fn baseline_does_not_match_marvisak_victory() {
        let runners_root = PathBuf::from(r"C:\GM\runners");
        let base_runner = DiscoveredRunner {
            name: "baseline".into(),
            path: runners_root.join("baseline").join("victory.exe"),
            kind: RunnerKind::Baseline,
            size_label: "9.5 MB".into(),
            cache_label: "global".into(),
            has_backup: false,
            repo: String::new(),
            commit: String::new(),
            built_at: String::new(),
        };
        assert!(!matches_portwatch_project(
            &base_runner,
            &runners_root,
            "victory",
            "victory",
            "marvisak/victory"
        ));
    }
}
