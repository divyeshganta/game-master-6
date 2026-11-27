//! Universal-Standard Performance Overlay (v1.2 — RTSS / MangoHud / Steam Standard)
//! Supports 3 density modes:
//!   0 = COMPACT   (380x36 sleek horizontal telemetry strip)
//!   1 = STANDARD  (316x118 MangoHud/RTSS card: FPS, 1% Low, [WORLD]/[MENU], Frame/GPU ms, RAM/Commit, live 60-sample frametime sparkline)
//!   2 = MINIMAL   (148x34 Steam-style FPS-only pill)

use std::ffi::c_int;
use std::sync::Mutex;

use crate::win32::*;

#[derive(Clone, Debug)]
struct HudState {
    mode: u8, // 0=Compact, 1=Standard+Graph, 2=Minimal
    text: String,
    accent: DWORD,
    has_live: bool,
    fps: f64,
    low1_fps: f64,
    frame_ms: f64,
    gpu_ms: f64,
    draws: u32,
    ws_gb: f64,
    commit_gb: f64,
    frametimes: Vec<f64>,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            mode: 1,
            text: String::new(),
            accent: rgb(0, 240, 255),
            has_live: false,
            fps: 0.0,
            low1_fps: 0.0,
            frame_ms: 0.0,
            gpu_ms: 0.0,
            draws: 0,
            ws_gb: 0.0,
            commit_gb: 0.0,
            frametimes: Vec::with_capacity(64),
        }
    }
}

static HUD_STATE: Mutex<HudState> = Mutex::new(HudState {
    mode: 1,
    text: String::new(),
    accent: rgb(0, 240, 255),
    has_live: false,
    fps: 0.0,
    low1_fps: 0.0,
    frame_ms: 0.0,
    gpu_ms: 0.0,
    draws: 0,
    ws_gb: 0.0,
    commit_gb: 0.0,
    frametimes: Vec::new(),
});

fn dims_for_mode(mode: u8) -> (c_int, c_int) {
    match mode % 3 {
        0 => (396, 36),  // COMPACT
        2 => (152, 34),  // MINIMAL
        _ => (316, 118), // STANDARD + FRAMETIME GRAPH
    }
}

pub struct HudOverlay {
    hwnd: HWND,
    visible: bool,
    mode: u8,
    corner: u8,
    last_text: String,
    last_accent: DWORD,
}

impl HudOverlay {
    pub unsafe fn create() -> Option<Self> {
        let class = wide("GameMasterHudOverlayV15");
        let hinst = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: 0,
            lpfn_wnd_proc: Some(hud_wnd_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: hinst,
            h_icon: std::ptr::null_mut(),
            h_cursor: std::ptr::null_mut(),
            hbr_background: std::ptr::null_mut(),
            lpsz_menu_name: std::ptr::null(),
            lpsz_class_name: class.as_ptr(),
            h_icon_sm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        let ex_style = WS_EX_TOPMOST
            | WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE;
        let title = wide("GM6 Performance HUD");
        let (w, h) = dims_for_mode(1);
        let hwnd = CreateWindowExW(
            ex_style,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            18,
            18,
            w,
            h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return None;
        }
        SetLayeredWindowAttributes(hwnd, 0, 232, LWA_ALPHA);
        Some(Self {
            hwnd,
            visible: false,
            mode: 1,
            corner: 0,
            last_text: String::new(),
            last_accent: 0,
        })
    }

    pub unsafe fn set_mode(&mut self, mode: u8) {
        let m = mode % 3;
        self.mode = m;
        if let Ok(mut g) = HUD_STATE.lock() {
            g.mode = m;
        }
        self.reposition();
        if self.visible {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub unsafe fn set_corner(&mut self, corner: u8) {
        self.corner = corner % 4;
        self.reposition();
    }

    unsafe fn reposition(&self) {
        let sx = GetSystemMetrics(SM_CXSCREEN).max(800);
        let sy = GetSystemMetrics(SM_CYSCREEN).max(600);
        let (w, h) = dims_for_mode(self.mode);
        let (x, y) = match self.corner % 4 {
            1 => (sx - w - 18, 18),
            2 => (18, sy - h - 42),
            3 => (sx - w - 18, sy - h - 42),
            _ => (18, 18),
        };
        SetWindowPos(
            self.hwnd,
            std::ptr::null_mut(),
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    /// Universal-standard telemetry update with rolling 60-sample frametime graph and 1% Low FPS.
    pub unsafe fn update_telemetry(
        &mut self,
        mode: u8,
        fps: f64,
        frame_ms: f64,
        gpu_ms: f64,
        draws: u32,
        ws_gb: f64,
        commit_gb: f64,
        accent: DWORD,
    ) {
        if self.mode != (mode % 3) {
            self.set_mode(mode);
        }
        let world_tag = if draws >= 500 { "WORLD" } else { "MENU" };
        let summary = format!(
            "Title · {:.1} FPS [{world_tag}] · {:.1}ms (GPU {:.1}ms) · RAM {:.1}G",
            fps, frame_ms, gpu_ms, ws_gb
        );
        if let Ok(mut g) = HUD_STATE.lock() {
            g.mode = self.mode;
            g.text = summary.clone();
            g.accent = accent;
            g.has_live = true;
            g.fps = fps;
            g.frame_ms = frame_ms;
            g.gpu_ms = gpu_ms;
            g.draws = draws;
            g.ws_gb = ws_gb;
            g.commit_gb = commit_gb;
            if frame_ms > 0.1 && frame_ms < 2000.0 {
                g.frametimes.push(frame_ms);
                if g.frametimes.len() > 60 {
                    let excess = g.frametimes.len() - 60;
                    g.frametimes.drain(0..excess);
                }
                let mut sorted = g.frametimes.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = ((sorted.len() as f64) * 0.99).floor() as usize;
                let p99_ms = sorted[idx.min(sorted.len().saturating_sub(1))].max(0.1);
                g.low1_fps = (1000.0 / p99_ms).clamp(0.1, 999.0);
            }
        }
        self.last_text = summary;
        self.last_accent = accent;
        if !self.visible {
            self.reposition();
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            self.visible = true;
        }
        InvalidateRect(self.hwnd, std::ptr::null(), 0);
    }

    pub unsafe fn update(&mut self, text: &str, accent: DWORD) {
        let changed = self.last_text != text || self.last_accent != accent || !self.visible;
        if !changed {
            return;
        }
        self.last_text = text.to_string();
        self.last_accent = accent;
        if let Ok(mut g) = HUD_STATE.lock() {
            g.text = self.last_text.clone();
            g.accent = accent;
            g.has_live = false;
        }
        if !self.visible {
            self.reposition();
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            self.visible = true;
        }
        InvalidateRect(self.hwnd, std::ptr::null(), 0);
    }

    pub unsafe fn hide(&mut self) {
        if self.visible {
            ShowWindow(self.hwnd, SW_HIDE);
            self.visible = false;
            self.last_text.clear();
            if let Ok(mut g) = HUD_STATE.lock() {
                g.frametimes.clear();
                g.has_live = false;
            }
        }
    }

    pub unsafe fn destroy(&mut self) {
        if !self.hwnd.is_null() {
            DestroyWindow(self.hwnd);
            self.hwnd = std::ptr::null_mut();
            self.visible = false;
        }
    }
}

unsafe fn make_hud_font(px: c_int, bold: bool) -> HGDIOBJ {
    let face = wide("Consolas");
    CreateFontW(
        -px,
        0,
        0,
        0,
        if bold { 700 } else { 400 },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        0,
        0,
        CLEARTYPE_QUALITY,
        0,
        face.as_ptr(),
    )
}

unsafe extern "system" fn hud_wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut cr = RECT::default();
            GetClientRect(hwnd, &mut cr);
            let w = cr.right.max(1);
            let h = cr.bottom.max(1);

            let mem = CreateCompatibleDC(hdc);
            let bmp = CreateCompatibleBitmap(hdc, w, h);
            let old_bmp = SelectObject(mem, bmp);

            let st = HUD_STATE.lock().map(|g| g.clone()).unwrap_or_default();

            // Background card + accent rail
            let bg_br = CreateSolidBrush(rgb(10, 12, 20));
            let pen = CreatePen(PS_SOLID, 1, st.accent);
            let old_b = SelectObject(mem, bg_br);
            let old_p = SelectObject(mem, pen);
            RoundRect(mem, 0, 0, w, h, 10, 10);
            SelectObject(mem, old_b);
            SelectObject(mem, old_p);
            DeleteObject(pen);
            DeleteObject(bg_br);

            let bar_br = CreateSolidBrush(st.accent);
            let bar_r = RECT::new(2, 6, 5, h - 6);
            FillRect(mem, &bar_r, bar_br);
            DeleteObject(bar_br);
            SetBkMode(mem, TRANSPARENT);

            if st.mode == 1 && st.has_live && h >= 100 {
                // ── STANDARD RTSS / MangoHud Card + Frametime Sparkline ──
                let f_big = make_hud_font(20, true);
                let f_sm = make_hud_font(12, true);
                let f_dim = make_hud_font(11, false);
                let old_f = SelectObject(mem, f_big);

                // Row 1: Big FPS + [WORLD/MENU] + 1% LOW
                SetTextColor(mem, st.accent);
                let mut r_fps = RECT::new(12, 6, 126, 30);
                let fps_w = wide(&format!("{:.1} FPS", st.fps));
                DrawTextW(mem, fps_w.as_ptr(), -1, &mut r_fps, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

                SelectObject(mem, f_sm);
                let world_tag = if st.draws >= 500 { "[WORLD]" } else { "[MENU]" };
                SetTextColor(mem, if st.draws >= 500 { rgb(34, 211, 138) } else { rgb(255, 190, 60) });
                let mut r_tag = RECT::new(128, 8, 196, 28);
                let tag_w = wide(world_tag);
                DrawTextW(mem, tag_w.as_ptr(), -1, &mut r_tag, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

                SetTextColor(mem, rgb(210, 218, 236));
                let mut r_low = RECT::new(196, 8, w - 10, 28);
                let low_w = wide(&format!("1% {:.1}", st.low1_fps));
                DrawTextW(mem, low_w.as_ptr(), -1, &mut r_low, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

                // Row 2: Frame ms | GPU ms | Draws
                SelectObject(mem, f_dim);
                SetTextColor(mem, rgb(186, 196, 220));
                let mut r_row2 = RECT::new(12, 32, w - 10, 48);
                let row2_w = wide(&format!("FRAME {:5.1}ms  ·  GPU {:5.1}ms  ·  {} DRW", st.frame_ms, st.gpu_ms, st.draws));
                DrawTextW(mem, row2_w.as_ptr(), -1, &mut r_row2, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

                // Row 3: RAM WorkingSet | Private Commit
                let mut r_row3 = RECT::new(12, 50, w - 10, 66);
                let row3_w = wide(&format!("RAM {:4.1} GB  ·  COMMIT {:4.1} GB", st.ws_gb, st.commit_gb));
                DrawTextW(mem, row3_w.as_ptr(), -1, &mut r_row3, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

                // Row 4: Live 60-sample Frametime Sparkline Graph (0..66.7ms scale)
                let gx1 = 12;
                let gy1 = 72;
                let gx2 = w - 12;
                let gy2 = h - 10;
                let gbg = CreateSolidBrush(rgb(16, 19, 30));
                let gr = RECT::new(gx1, gy1, gx2, gy2);
                FillRect(mem, &gr, gbg);
                DeleteObject(gbg);

                // 33.3ms (30fps) midpoint reference line
                let ref_pen = CreatePen(PS_SOLID, 1, rgb(44, 52, 74));
                let old_gp = SelectObject(mem, ref_pen);
                let mut pt = POINT { x: 0, y: 0 };
                let mid_y = gy1 + (gy2 - gy1) / 2;
                MoveToEx(mem, gx1, mid_y, &mut pt);
                LineTo(mem, gx2, mid_y);
                SelectObject(mem, old_gp);
                DeleteObject(ref_pen);

                if st.frametimes.len() >= 2 {
                    let gpen = CreatePen(PS_SOLID, 2, st.accent);
                    let old_lp = SelectObject(mem, gpen);
                    let gw = (gx2 - gx1).max(1) as f64;
                    let gh = (gy2 - gy1).max(1) as f64;
                    let n = st.frametimes.len();
                    for (i, &ms) in st.frametimes.iter().enumerate() {
                        let px = gx1 + ((i as f64 / (n - 1).max(1) as f64) * gw) as c_int;
                        let norm = (ms / 66.7).clamp(0.0, 1.0);
                        let py = gy2 - (norm * gh) as c_int;
                        if i == 0 {
                            MoveToEx(mem, px, py, &mut pt);
                        } else {
                            LineTo(mem, px, py);
                        }
                    }
                    SelectObject(mem, old_lp);
                    DeleteObject(gpen);
                }

                SelectObject(mem, old_f);
                DeleteObject(f_big);
                DeleteObject(f_sm);
                DeleteObject(f_dim);
            } else {
                // ── COMPACT or MINIMAL single-line bar ──
                let font = make_hud_font(if st.mode == 2 { 15 } else { 13 }, true);
                let old_f = SelectObject(mem, font);
                SetTextColor(mem, if st.mode == 2 { st.accent } else { rgb(244, 248, 255) });
                let display_txt = if st.mode == 2 && st.has_live {
                    format!("{:.1} FPS", st.fps)
                } else {
                    st.text.clone()
                };
                let mut tr = RECT::new(12, 0, w - 8, h);
                let tw = wide(&display_txt);
                DrawTextW(mem, tw.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
                SelectObject(mem, old_f);
                DeleteObject(font);
            }

            BitBlt(hdc, 0, 0, w, h, mem, 0, 0, SRCCOPY);
            SelectObject(mem, old_bmp);
            DeleteObject(bmp);
            DeleteDC(mem);

            EndPaint(hwnd, &ps);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ─── v1.2 Standard-Style Click-Through Crosshair Overlay ─────────────────────
static CROSSHAIR_AIM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub struct CrosshairOverlay {
    hwnd: HWND,
    visible: bool,
    last_aim: bool,
    last_x: c_int,
    last_y: c_int,
}

impl CrosshairOverlay {
    pub unsafe fn create() -> Option<Self> {
        let class = wide("GameMasterCrosshairV15");
        let hinst = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cb_size: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: 0,
            lpfn_wnd_proc: Some(crosshair_wnd_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: hinst,
            h_icon: std::ptr::null_mut(),
            h_cursor: std::ptr::null_mut(),
            hbr_background: std::ptr::null_mut(),
            lpsz_menu_name: std::ptr::null(),
            lpsz_class_name: class.as_ptr(),
            h_icon_sm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        let ex_style = WS_EX_TOPMOST
            | WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE;
        let title = wide("GM6 Reticle");
        let hwnd = CreateWindowExW(
            ex_style,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            32,
            32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return None;
        }
        SetLayeredWindowAttributes(hwnd, rgb(255, 0, 255), 255, LWA_COLORKEY);
        Some(Self {
            hwnd,
            visible: false,
            last_aim: false,
            last_x: -1,
            last_y: -1,
        })
    }

    pub unsafe fn sync(&mut self, enabled: bool) {
        if !enabled {
            self.hide();
            return;
        }
        let state_path = std::env::temp_dir().join("victory-kbm-state.txt");
        let state_str = std::fs::read_to_string(state_path).unwrap_or_default();
        let trimmed = state_str.trim();
        let (show_now, is_aim) = match trimmed {
            "GAMEPLAY|AIM" => (true, true),
            "GAMEPLAY|HIP" => (true, false),
            _ => (false, false),
        };
        if !show_now {
            self.hide();
            return;
        }
        let Some(rc) = foreground_game_rect() else {
            self.hide();
            return;
        };
        let x = (rc.left + rc.right) / 2 - 16;
        let y = (rc.top + rc.bottom) / 2 - 16;
        let moved = x != self.last_x || y != self.last_y;
        let aim_changed = is_aim != self.last_aim;
        if moved || !self.visible {
            SetWindowPos(
                self.hwnd,
                (-1isize) as HWND,
                x,
                y,
                32,
                32,
                SWP_NOACTIVATE,
            );
            self.last_x = x;
            self.last_y = y;
        }
        if !self.visible {
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            self.visible = true;
        }
        if aim_changed || moved {
            self.last_aim = is_aim;
            CROSSHAIR_AIM.store(is_aim, std::sync::atomic::Ordering::Relaxed);
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub unsafe fn hide(&mut self) {
        if self.visible {
            ShowWindow(self.hwnd, SW_HIDE);
            self.visible = false;
        }
    }
}

unsafe extern "system" fn crosshair_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let bg = CreateSolidBrush(rgb(255, 0, 255));
            let full = RECT::new(0, 0, 32, 32);
            FillRect(hdc, &full, bg);
            DeleteObject(bg);

            let is_aim = CROSSHAIR_AIM.load(std::sync::atomic::Ordering::Relaxed);
            let cx = 16;
            let cy = 16;

            let dark_br = CreateSolidBrush(rgb(12, 12, 16));
            let dark_pen = CreatePen(PS_SOLID, 1, rgb(12, 12, 16));
            let old_b = SelectObject(hdc, dark_br);
            let old_p = SelectObject(hdc, dark_pen);
            Ellipse(hdc, cx - 3, cy - 3, cx + 4, cy + 4);

            let white_br = CreateSolidBrush(rgb(250, 252, 255));
            let white_pen = CreatePen(PS_SOLID, 1, rgb(250, 252, 255));
            SelectObject(hdc, white_br);
            SelectObject(hdc, white_pen);
            Ellipse(hdc, cx - 2, cy - 2, cx + 3, cy + 3);

            if is_aim {
                let mut pt = POINT { x: 0, y: 0 };
                SelectObject(hdc, dark_pen);
                for (x1, y1, x2, y2) in [
                    (cx - 10, cy, cx - 5, cy),
                    (cx + 6, cy, cx + 11, cy),
                    (cx, cy - 10, cx, cy - 5),
                    (cx, cy + 6, cx, cy + 11),
                ] {
                    MoveToEx(hdc, x1, y1 - 1, &mut pt);
                    LineTo(hdc, x2, y2 - 1);
                    MoveToEx(hdc, x1, y1 + 1, &mut pt);
                    LineTo(hdc, x2, y2 + 1);
                }
                SelectObject(hdc, white_pen);
                for (x1, y1, x2, y2) in [
                    (cx - 10, cy, cx - 5, cy),
                    (cx + 6, cy, cx + 11, cy),
                    (cx, cy - 10, cx, cy - 5),
                    (cx, cy + 6, cx, cy + 11),
                ] {
                    MoveToEx(hdc, x1, y1, &mut pt);
                    LineTo(hdc, x2, y2);
                }
            }

            SelectObject(hdc, old_b);
            SelectObject(hdc, old_p);
            DeleteObject(white_br);
            DeleteObject(white_pen);
            DeleteObject(dark_br);
            DeleteObject(dark_pen);

            EndPaint(hwnd, &ps);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
