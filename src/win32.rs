#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::{c_int, c_uint, c_void};
use std::path::Path;

pub type HWND = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HMENU = *mut c_void;
pub type HBRUSH = *mut c_void;
pub type HCURSOR = *mut c_void;
pub type HICON = *mut c_void;
pub type HDC = *mut c_void;
pub type HGDIOBJ = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HANDLE = *mut c_void;
pub type HKEY = *mut c_void;
pub type LPARAM = isize;
pub type WPARAM = usize;
pub type LRESULT = isize;
pub type ATOM = u16;
pub type BOOL = c_int;
pub type DWORD = u32;
pub type WORD = u16;
pub type UINT = c_uint;
pub type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

// Window Messages
pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_SETCURSOR: UINT = 0x0020;
pub const WM_GETMINMAXINFO: UINT = 0x0024;
pub const WM_SETICON: UINT = 0x0080;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_MOUSELEAVE: UINT = 0x02A3;
pub const WM_HOTKEY: UINT = 0x0312;
pub const MOD_CONTROL: UINT = 0x0002;
pub const MOD_SHIFT: UINT = 0x0004;
pub const MOD_NOREPEAT: UINT = 0x4000;
pub const VK_F9: UINT = 0x78;
pub const VK_F10: UINT = 0x79;
pub const VK_F11: UINT = 0x7A;

// Hit test
pub const HTCLIENT: c_int = 1;

// Virtual keys
pub const VK_RETURN: WPARAM = 0x0D;
pub const VK_ESCAPE: WPARAM = 0x1B;
pub const VK_F5: WPARAM = 0x74;
pub const VK_CONTROL: c_int = 0x11;

// Window styles
pub const CS_HREDRAW: UINT = 0x0002;
pub const CS_VREDRAW: UINT = 0x0001;
pub const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
pub const WS_VISIBLE: DWORD = 0x10000000;
pub const WS_CLIPCHILDREN: DWORD = 0x02000000;
pub const WS_EX_COMPOSITED: DWORD = 0x02000000;
pub const WS_POPUP: DWORD = 0x80000000;
pub const WS_EX_TOPMOST: DWORD = 0x00000008;
pub const WS_EX_TRANSPARENT: DWORD = 0x00000020;
pub const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
pub const WS_EX_LAYERED: DWORD = 0x00080000;
pub const WS_EX_NOACTIVATE: DWORD = 0x08000000;
pub const LWA_COLORKEY: DWORD = 0x00000001;
pub const LWA_ALPHA: DWORD = 0x00000002;

// ShowWindow commands
pub const SW_HIDE: c_int = 0;
pub const SW_SHOWNORMAL: c_int = 1;
pub const SW_SHOWNOACTIVATE: c_int = 4;
pub const SW_SHOWMAXIMIZED: c_int = 3;
pub const SW_SHOW: c_int = 5;
pub const SW_MINIMIZE: c_int = 6;
pub const SW_RESTORE: c_int = 9;

// System metrics
pub const SM_CXSCREEN: c_int = 0;
pub const SM_CYSCREEN: c_int = 1;

// Cursors & Icons
pub const IDC_ARROW: usize = 32512;
pub const IDC_HAND: usize = 32649;
pub const IMAGE_ICON: UINT = 1;
pub const LR_LOADFROMFILE: UINT = 0x00000010;
pub const ICON_SMALL: WPARAM = 0;
pub const ICON_BIG: WPARAM = 1;

// Window long
pub const GWLP_USERDATA: c_int = -21;

// MessageBox flags
pub const MB_OK: UINT = 0x00000000;
pub const MB_YESNO: UINT = 0x00000004;
pub const MB_ICONERROR: UINT = 0x00000010;
pub const MB_ICONQUESTION: UINT = 0x00000020;
pub const MB_ICONWARNING: UINT = 0x00000030;
pub const MB_ICONINFORMATION: UINT = 0x00000040;
pub const IDYES: c_int = 6;

// File dialog flags
pub const OFN_HIDEREADONLY: DWORD = 0x00000004;
pub const OFN_NOCHANGEDIR: DWORD = 0x00000008;
pub const OFN_PATHMUSTEXIST: DWORD = 0x00000800;
pub const OFN_FILEMUSTEXIST: DWORD = 0x00001000;
pub const OFN_EXPLORER: DWORD = 0x00080000;

// Folder dialog flags
pub const BIF_RETURNONLYFSDIRS: UINT = 0x0001;
pub const BIF_EDITBOX: UINT = 0x0010;
pub const BIF_NEWDIALOGSTYLE: UINT = 0x0040;
pub const COINIT_APARTMENTTHREADED: DWORD = 0x2;

// GDI constants
pub const DIB_RGB_COLORS: UINT = 0;
pub const SRCCOPY: DWORD = 0x00CC0020;
pub const HALFTONE: c_int = 4;
pub const GRADIENT_FILL_RECT_H: DWORD = 0;
pub const GRADIENT_FILL_RECT_V: DWORD = 1;
pub const AC_SRC_OVER: u8 = 0x00;
pub const AC_SRC_ALPHA: u8 = 0x01;
pub const TRANSPARENT: c_int = 1;
pub const PS_SOLID: c_int = 0;
pub const PS_NULL: c_int = 5;
pub const NULL_BRUSH: c_int = 5;
pub const NULL_PEN: c_int = 8;
pub const ANTIALIASED_QUALITY: u8 = 4;
pub const CLEARTYPE_QUALITY: u8 = 5;
pub const DEFAULT_CHARSET: u8 = 1;
pub const FR_PRIVATE: DWORD = 0x00000010;

// DrawText flags
pub const DT_LEFT: DWORD = 0x00000000;
pub const DT_CENTER: DWORD = 0x00000001;
pub const DT_RIGHT: DWORD = 0x00000002;
pub const DT_VCENTER: DWORD = 0x00000004;
pub const DT_WORDBREAK: DWORD = 0x00000010;
pub const DT_SINGLELINE: DWORD = 0x00000020;
pub const DT_NOPREFIX: DWORD = 0x00000800;
pub const DT_END_ELLIPSIS: DWORD = 0x00008000;
pub const DT_PATH_ELLIPSIS: DWORD = 0x00004000;

// TrackMouseEvent
pub const TME_LEAVE: DWORD = 0x00000002;

// Processor features
pub const PF_AVX2_INSTRUCTIONS_AVAILABLE: DWORD = 40;

// Process priority & power throttling
pub const PROCESS_SET_INFORMATION: DWORD = 0x0200;
pub const HIGH_PRIORITY_CLASS: DWORD = 0x00000080;
pub const PROCESS_POWER_THROTTLING_CLASS: DWORD = 4;
pub const PROCESS_POWER_THROTTLING_CURRENT_VERSION: DWORD = 1;
pub const PROCESS_POWER_THROTTLING_EXECUTION_SPEED: DWORD = 0x1;
pub const PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION: DWORD = 0x4;

// Registry (DirectX UserGpuPreferences)
pub const HKEY_CURRENT_USER: HKEY = 0x80000001usize as HKEY;
pub const HKEY_LOCAL_MACHINE: HKEY = 0x80000002usize as HKEY;
pub const KEY_QUERY_VALUE: DWORD = 0x0001;
pub const REG_DWORD: DWORD = 4;
pub const KEY_SET_VALUE: DWORD = 0x0002;
pub const KEY_CREATE_SUB_KEY: DWORD = 0x0004;
pub const REG_SZ: DWORD = 1;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl RECT {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub erase: BOOL,
    pub paint: RECT,
    pub restore: BOOL,
    pub inc_update: BOOL,
    pub reserved: [u8; 32],
}

#[repr(C)]
pub struct MINMAXINFO {
    pub pt_reserved: POINT,
    pub pt_max_size: POINT,
    pub pt_max_position: POINT,
    pub pt_min_track_size: POINT,
    pub pt_max_track_size: POINT,
}

#[repr(C)]
pub struct TRACKMOUSEEVENT {
    pub cb_size: DWORD,
    pub dw_flags: DWORD,
    pub hwnd_track: HWND,
    pub dw_hover_time: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TRIVERTEX {
    pub x: i32,
    pub y: i32,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GRADIENT_RECT {
    pub upper: u32,
    pub lower: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BLENDFUNCTION {
    pub blend_op: u8,
    pub blend_flags: u8,
    pub source_constant_alpha: u8,
    pub alpha_format: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFOHEADER {
    pub bi_size: DWORD,
    pub bi_width: i32,
    pub bi_height: i32,
    pub bi_planes: WORD,
    pub bi_bit_count: WORD,
    pub bi_compression: DWORD,
    pub bi_size_image: DWORD,
    pub bi_x_pels_per_meter: i32,
    pub bi_y_pels_per_meter: i32,
    pub bi_clr_used: DWORD,
    pub bi_clr_important: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFO {
    pub bmi_header: BITMAPINFOHEADER,
    pub bmi_colors: [DWORD; 1],
}

#[repr(C)]
#[derive(Default)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub w_param: WPARAM,
    pub l_param: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cb_size: UINT,
    pub style: UINT,
    pub lpfn_wnd_proc: WNDPROC,
    pub cb_cls_extra: c_int,
    pub cb_wnd_extra: c_int,
    pub h_instance: HINSTANCE,
    pub h_icon: HICON,
    pub h_cursor: HCURSOR,
    pub hbr_background: HBRUSH,
    pub lpsz_menu_name: *const u16,
    pub lpsz_class_name: *const u16,
    pub h_icon_sm: HICON,
}

#[repr(C)]
pub struct OPENFILENAMEW {
    pub l_struct_size: DWORD,
    pub hwnd_owner: HWND,
    pub h_instance: HINSTANCE,
    pub lpstr_filter: *const u16,
    pub lpstr_file: *mut u16,
    pub n_max_file: DWORD,
    pub lpstr_file_title: *mut u16,
    pub n_max_file_title: DWORD,
    pub lpstr_initial_dir: *const u16,
    pub lpstr_title: *const u16,
    pub flags: DWORD,
    pub n_file_offset: WORD,
    pub n_file_extension: WORD,
    pub lpstr_def_ext: *const u16,
    pub l_cust_data: LPARAM,
    pub lpfn_hook: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> isize>,
    pub lp_template_name: *const u16,
    pub pv_reserved: *mut c_void,
    pub dw_reserved: DWORD,
    pub flags_ex: DWORD,
}

#[repr(C)]
pub struct BROWSEINFOW {
    pub hwnd_owner: HWND,
    pub pidl_root: *const c_void,
    pub psz_display_name: *mut u16,
    pub lpsz_title: *const u16,
    pub ul_flags: UINT,
    pub lpfn: Option<unsafe extern "system" fn(HWND, UINT, LPARAM, LPARAM) -> isize>,
    pub l_param: LPARAM,
    pub i_image: c_int,
}

#[repr(C)]
#[derive(Default)]
pub struct MEMORYSTATUSEX {
    pub dw_length: DWORD,
    pub dw_memory_load: DWORD,
    pub ull_total_phys: u64,
    pub ull_avail_phys: u64,
    pub ull_total_page_file: u64,
    pub ull_avail_page_file: u64,
    pub ull_total_virtual: u64,
    pub ull_avail_virtual: u64,
    pub ull_avail_extended_virtual: u64,
}

#[repr(C)]
pub struct SYSTEM_INFO {
    pub dw_oem_id: DWORD,
    pub dw_page_size: DWORD,
    pub lp_min_app_addr: *mut c_void,
    pub lp_max_app_addr: *mut c_void,
    pub dw_active_processor_mask: usize,
    pub dw_number_of_processors: DWORD,
    pub dw_processor_type: DWORD,
    pub dw_allocation_granularity: DWORD,
    pub w_processor_level: WORD,
    pub w_processor_revision: WORD,
}

#[repr(C)]
pub struct PROCESS_POWER_THROTTLING_STATE {
    pub version: DWORD,
    pub control_mask: DWORD,
    pub state_mask: DWORD,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct SYSTEMTIME {
    pub w_year: WORD,
    pub w_month: WORD,
    pub w_day_of_week: WORD,
    pub w_day: WORD,
    pub w_hour: WORD,
    pub w_minute: WORD,
    pub w_second: WORD,
    pub w_milliseconds: WORD,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lp_module_name: *const u16) -> HINSTANCE;
    pub fn GlobalMemoryStatusEx(lp_buffer: *mut MEMORYSTATUSEX) -> BOOL;
    pub fn GetDiskFreeSpaceExW(
        lp_directory_name: *const u16,
        lp_free_bytes_available: *mut u64,
        lp_total_bytes: *mut u64,
        lp_total_free_bytes: *mut u64,
    ) -> BOOL;
    pub fn IsProcessorFeaturePresent(processor_feature: DWORD) -> BOOL;
    pub fn GetSystemInfo(lp_system_info: *mut SYSTEM_INFO);
    pub fn GetLocalTime(lp_system_time: *mut SYSTEMTIME);
    pub fn OpenProcess(dw_desired_access: DWORD, b_inherit_handle: BOOL, dw_process_id: DWORD) -> HANDLE;
    pub fn SetPriorityClass(h_process: HANDLE, dw_priority_class: DWORD) -> BOOL;
    pub fn SetProcessInformation(
        h_process: HANDLE,
        process_information_class: DWORD,
        process_information: *const c_void,
        process_information_size: DWORD,
    ) -> BOOL;
    pub fn CloseHandle(h_object: HANDLE) -> BOOL;
    pub fn LoadLibraryW(lp_lib_file_name: *const u16) -> HANDLE;
    pub fn GetProcAddress(h_module: HANDLE, lp_proc_name: *const u8) -> *mut c_void;
    pub fn GlobalAlloc(u_flags: UINT, dw_bytes: usize) -> HANDLE;
    pub fn GlobalLock(h_mem: HANDLE) -> *mut c_void;
    pub fn GlobalUnlock(h_mem: HANDLE) -> BOOL;
    pub fn CreateHardLinkW(
        lp_file_name: *const u16,
        lp_existing_file_name: *const u16,
        lp_security_attributes: *mut c_void,
    ) -> BOOL;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegCreateKeyExW(
        h_key: HKEY,
        lp_sub_key: *const u16,
        reserved: DWORD,
        lp_class: *const u16,
        dw_options: DWORD,
        sam_desired: DWORD,
        lp_security_attributes: *mut c_void,
        phk_result: *mut HKEY,
        lpdw_disposition: *mut DWORD,
    ) -> i32;
    pub fn RegSetValueExW(
        h_key: HKEY,
        lp_value_name: *const u16,
        reserved: DWORD,
        dw_type: DWORD,
        lp_data: *const u8,
        cb_data: DWORD,
    ) -> i32;
    pub fn RegDeleteValueW(h_key: HKEY, lp_value_name: *const u16) -> i32;
    pub fn RegOpenKeyExW(
        h_key: HKEY,
        lp_sub_key: *const u16,
        ul_options: DWORD,
        sam_desired: DWORD,
        phk_result: *mut HKEY,
    ) -> i32;
    pub fn RegQueryValueExW(
        h_key: HKEY,
        lp_value_name: *const u16,
        lp_reserved: *mut DWORD,
        lp_type: *mut DWORD,
        lp_data: *mut u8,
        lpcb_data: *mut DWORD,
    ) -> i32;
    pub fn RegCloseKey(h_key: HKEY) -> i32;
}

#[link(name = "user32")]
extern "system" {
    pub fn SetProcessDPIAware() -> BOOL;
    pub fn GetSystemMetrics(n_index: c_int) -> c_int;
    pub fn RegisterClassExW(lp_wc: *const WNDCLASSEXW) -> ATOM;
    pub fn CreateWindowExW(
        dw_ex_style: DWORD,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: DWORD,
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
        hwnd_parent: HWND,
        h_menu: HMENU,
        h_instance: HINSTANCE,
        lp_param: *mut c_void,
    ) -> HWND;
    pub fn ShowWindow(hwnd: HWND, n_cmd_show: c_int) -> BOOL;
    pub fn SetForegroundWindow(hwnd: HWND) -> BOOL;
    pub fn IsIconic(hwnd: HWND) -> BOOL;
    pub fn UpdateWindow(hwnd: HWND) -> BOOL;
    pub fn GetMessageW(lp_msg: *mut MSG, hwnd: HWND, min: UINT, max: UINT) -> BOOL;
    pub fn TranslateMessage(lp_msg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(n_exit_code: c_int);
    pub fn DefWindowProcW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    pub fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn AdjustWindowRectEx(rect: *mut RECT, style: DWORD, menu: BOOL, ex_style: DWORD) -> BOOL;
    pub fn SendMessageW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    pub fn SetTimer(
        hwnd: HWND,
        id_event: usize,
        u_elapse: UINT,
        lp_timer_proc: Option<unsafe extern "system" fn(HWND, UINT, usize, DWORD)>,
    ) -> usize;
    pub fn KillTimer(hwnd: HWND, id_event: usize) -> BOOL;
    pub fn MessageBoxW(hwnd: HWND, text: *const u16, caption: *const u16, kind: UINT) -> c_int;
    pub fn BeginPaint(hwnd: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hwnd: HWND, paint: *const PAINTSTRUCT) -> BOOL;
    pub fn DrawTextW(hdc: HDC, text: *const u16, count: c_int, rect: *mut RECT, flags: DWORD) -> c_int;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> c_int;
    pub fn FrameRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> c_int;
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: BOOL) -> BOOL;
    pub fn LoadCursorW(h_instance: HINSTANCE, lp_cursor_name: *const u16) -> HCURSOR;
    pub fn SetCursor(h_cursor: HCURSOR) -> HCURSOR;
    pub fn LoadImageW(
        h_inst: HINSTANCE,
        name: *const u16,
        img_type: UINT,
        cx: c_int,
        cy: c_int,
        fu_load: UINT,
    ) -> *mut c_void;
    pub fn SetWindowLongPtrW(hwnd: HWND, n_index: c_int, dw_new_long: isize) -> isize;
    pub fn GetWindowLongPtrW(hwnd: HWND, n_index: c_int) -> isize;
    pub fn DestroyWindow(hwnd: HWND) -> BOOL;
    pub fn TrackMouseEvent(lp_event_track: *mut TRACKMOUSEEVENT) -> BOOL;
    pub fn GetKeyState(n_virt_key: c_int) -> i16;
    pub fn OpenClipboard(h_wnd_new_owner: HWND) -> BOOL;
    pub fn EmptyClipboard() -> BOOL;
    pub fn GetClipboardData(u_format: UINT) -> HANDLE;
    pub fn SetClipboardData(u_format: UINT, h_mem: HANDLE) -> HANDLE;
    pub fn CloseClipboard() -> BOOL;
    pub fn SetLayeredWindowAttributes(hwnd: HWND, cr_key: DWORD, b_alpha: u8, dw_flags: DWORD) -> BOOL;
    pub fn SetWindowPos(hwnd: HWND, hwnd_after: HWND, x: c_int, y: c_int, cx: c_int, cy: c_int, flags: UINT) -> BOOL;
    pub fn RegisterHotKey(hwnd: HWND, id: c_int, fs_modifiers: UINT, vk: UINT) -> BOOL;
    pub fn UnregisterHotKey(hwnd: HWND, id: c_int) -> BOOL;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> c_int;
    pub fn EnumWindows(
        lp_enum_func: Option<unsafe extern "system" fn(HWND, LPARAM) -> BOOL>,
        l_param: LPARAM,
    ) -> BOOL;
    pub fn GetWindowThreadProcessId(hwnd: HWND, lpdw_process_id: *mut DWORD) -> DWORD;
    pub fn IsWindowVisible(hwnd: HWND) -> BOOL;
    pub fn GetForegroundWindow() -> HWND;
    pub fn GetWindowRect(hwnd: HWND, lp_rect: *mut RECT) -> BOOL;
    pub fn GetWindowTextW(hwnd: HWND, lp_string: *mut u16, n_max_count: c_int) -> c_int;
    pub fn SetWindowTextW(hwnd: HWND, lp_string: *const u16) -> BOOL;
    pub fn PostMessageW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn AddFontResourceExW(path: *const u16, flags: DWORD, reserved: *mut c_void) -> c_int;
    pub fn GetStockObject(fn_object: c_int) -> HGDIOBJ;
    pub fn CreateFontW(
        c_height: c_int,
        c_width: c_int,
        c_escapement: c_int,
        c_orientation: c_int,
        fn_weight: c_int,
        b_italic: u8,
        b_underline: u8,
        b_strike_out: u8,
        char_set: u8,
        out_precision: u8,
        clip_precision: u8,
        quality: u8,
        pitch_and_family: u8,
        face_name: *const u16,
    ) -> HGDIOBJ;
    pub fn DeleteObject(h_object: HGDIOBJ) -> BOOL;
    pub fn CreateSolidBrush(color: DWORD) -> HBRUSH;
    pub fn CreatePen(style: c_int, width: c_int, color: DWORD) -> HGDIOBJ;
    pub fn SelectObject(hdc: HDC, object: HGDIOBJ) -> HGDIOBJ;
    pub fn SetTextColor(hdc: HDC, color: DWORD) -> DWORD;
    pub fn SetBkMode(hdc: HDC, mode: c_int) -> c_int;
    pub fn RoundRect(hdc: HDC, x1: c_int, y1: c_int, x2: c_int, y2: c_int, w: c_int, h: c_int) -> BOOL;
    pub fn Ellipse(hdc: HDC, left: c_int, top: c_int, right: c_int, bottom: c_int) -> BOOL;
    pub fn Polygon(hdc: HDC, apt: *const POINT, cpt: c_int) -> BOOL;
    pub fn MoveToEx(hdc: HDC, x: c_int, y: c_int, pt: *mut POINT) -> BOOL;
    pub fn LineTo(hdc: HDC, x: c_int, y: c_int) -> BOOL;
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(hdc: HDC, w: c_int, h: c_int) -> HBITMAP;
    pub fn CreateDIBSection(
        hdc: HDC,
        pbmi: *const BITMAPINFO,
        usage: UINT,
        ppv_bits: *mut *mut c_void,
        h_section: *mut c_void,
        offset: DWORD,
    ) -> HBITMAP;
    pub fn BitBlt(
        hdc: HDC,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        src: HDC,
        sx: c_int,
        sy: c_int,
        rop: DWORD,
    ) -> BOOL;
    pub fn StretchBlt(
        hdc: HDC,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        src: HDC,
        sx: c_int,
        sy: c_int,
        sw: c_int,
        sh: c_int,
        rop: DWORD,
    ) -> BOOL;
    pub fn SetStretchBltMode(hdc: HDC, mode: c_int) -> c_int;
    pub fn SetBrushOrgEx(hdc: HDC, x: c_int, y: c_int, lppt: *mut POINT) -> BOOL;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn SaveDC(hdc: HDC) -> c_int;
    pub fn RestoreDC(hdc: HDC, saved: c_int) -> BOOL;
    pub fn IntersectClipRect(hdc: HDC, left: c_int, top: c_int, right: c_int, bottom: c_int) -> c_int;
    pub fn GetDIBits(hdc: HDC, hbm: HBITMAP, start: UINT, c_lines: UINT, lpv_bits: *mut c_void, lpbmi: *mut BITMAPINFO, usage: UINT) -> c_int;
}

#[link(name = "msimg32")]
extern "system" {
    pub fn GradientFill(
        hdc: HDC,
        vertices: *const TRIVERTEX,
        n_vertices: DWORD,
        rects: *const GRADIENT_RECT,
        n_rects: DWORD,
        mode: DWORD,
    ) -> BOOL;
    pub fn AlphaBlend(
        hdc_dest: HDC,
        x_origin_dest: c_int,
        y_origin_dest: c_int,
        w_dest: c_int,
        h_dest: c_int,
        hdc_src: HDC,
        x_origin_src: c_int,
        y_origin_src: c_int,
        w_src: c_int,
        h_src: c_int,
        ftn: BLENDFUNCTION,
    ) -> BOOL;
}

#[link(name = "comdlg32")]
extern "system" {
    pub fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn SHBrowseForFolderW(lpbi: *mut BROWSEINFOW) -> *mut c_void;
    pub fn SHGetPathFromIDListW(pidl: *const c_void, psz_path: *mut u16) -> BOOL;
    pub fn ShellExecuteW(
        hwnd: HWND,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: c_int,
    ) -> *mut c_void;
}

#[link(name = "ole32")]
extern "system" {
    pub fn CoInitializeEx(pv_reserved: *mut c_void, dw_co_init: DWORD) -> i32;
    pub fn CoUninitialize();
    pub fn CoTaskMemFree(pv: *mut c_void);
}

pub const fn rgb(r: u8, g: u8, b: u8) -> DWORD {
    ((b as DWORD) << 16) | ((g as DWORD) << 8) | (r as DWORD)
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub unsafe fn from_wide_slice(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub fn local_timestamp_filename() -> String {
    let mut st = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut st) };
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        st.w_year, st.w_month, st.w_day, st.w_hour, st.w_minute, st.w_second
    )
}

pub fn local_timestamp_human() -> String {
    let mut st = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut st) };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        st.w_year, st.w_month, st.w_day, st.w_hour, st.w_minute, st.w_second
    )
}

/// Opens a native file picker in an isolated hidden STA PowerShell subprocess so Explorer
/// shell extensions and modal message loops never crash or re-enter the main Win32 process.
pub unsafe fn browse_file(_hwnd: HWND, title: &str, filter_pairs: &[(&str, &str)], initial: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let filter_str: String = filter_pairs
        .iter()
        .map(|(d, p)| format!("{d}|{p}"))
        .collect::<Vec<_>>()
        .join("|");
    let t = title.replace('\'', "''");
    let flt = filter_str.replace('\'', "''");
    let init_dir = Path::new(initial)
        .parent()
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().replace('\'', "''"))
        .unwrap_or_default();
    let ps = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $d = New-Object System.Windows.Forms.OpenFileDialog; \
         $d.Title = '{t}'; \
         $d.Filter = '{flt}'; \
         if ('{init_dir}' -ne '' -and (Test-Path '{init_dir}')) {{ $d.InitialDirectory = '{init_dir}'; }} \
         if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::OutputEncoding = [Text.Encoding]::UTF8; Write-Output ('OK:' + $d.FileName) }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-STA", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.starts_with("OK:"))?;
    let path = line.strip_prefix("OK:")?.trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

/// Opens a native folder picker in an isolated hidden STA PowerShell subprocess.
pub unsafe fn browse_folder(_hwnd: HWND, title: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let t = title.replace('\'', "''");
    let ps = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $d = New-Object System.Windows.Forms.FolderBrowserDialog; \
         $d.Description = '{t}'; \
         $d.ShowNewFolderButton = $true; \
         if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::OutputEncoding = [Text.Encoding]::UTF8; Write-Output ('OK:' + $d.SelectedPath) }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-STA", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.starts_with("OK:"))?;
    let path = line.strip_prefix("OK:")?.trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

pub unsafe fn open_path(hwnd: HWND, path: &Path) {
    let op = wide("open");
    let target = wide(&path.to_string_lossy());
    ShellExecuteW(hwnd, op.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
}

/// Boosts child process priority and disables Windows EcoQoS power throttling.
pub unsafe fn boost_process(pid: u32) {
    let h = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
    if h.is_null() {
        return;
    }
    SetPriorityClass(h, HIGH_PRIORITY_CLASS);
    let state = PROCESS_POWER_THROTTLING_STATE {
        version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        control_mask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
        state_mask: 0,
    };
    SetProcessInformation(
        h,
        PROCESS_POWER_THROTTLING_CLASS,
        &state as *const _ as *const c_void,
        std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as DWORD,
    );
    CloseHandle(h);
}

/// Sets or clears Windows per-app High Performance GPU preference (GpuPreference=2;) for hybrid laptops.
pub unsafe fn set_high_perf_gpu_preference(exe_path: &Path, enable: bool) -> Result<(), String> {
    let subkey = wide(r"Software\Microsoft\DirectX\UserGpuPreferences");
    let mut hkey: HKEY = std::ptr::null_mut();
    let status = RegCreateKeyExW(
        HKEY_CURRENT_USER,
        subkey.as_ptr(),
        0,
        std::ptr::null(),
        0,
        KEY_SET_VALUE | KEY_CREATE_SUB_KEY,
        std::ptr::null_mut(),
        &mut hkey,
        std::ptr::null_mut(),
    );
    if status != 0 || hkey.is_null() {
        return Err(format!("Could not open Windows GPU preferences key (err {status})"));
    }

    let val_name = wide(&exe_path.to_string_lossy());
    let res = if enable {
        let data = wide("GpuPreference=2;");
        let bytes = (data.len() * 2) as DWORD;
        let rc = RegSetValueExW(
            hkey,
            val_name.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr() as *const u8,
            bytes,
        );
        if rc == 0 { Ok(()) } else { Err(format!("RegSetValueExW failed (err {rc})")) }
    } else {
        let _ = RegDeleteValueW(hkey, val_name.as_ptr());
        Ok(())
    };
    RegCloseKey(hkey);
    res
}

pub unsafe fn open_virtual_memory_settings(hwnd: HWND) {
    let op = wide("open");
    let file = wide("SystemPropertiesPerformance.exe");
    ShellExecuteW(hwnd, op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
}

pub unsafe fn open_url(hwnd: HWND, url: &str) {
    let op = wide("open");
    let target = wide(url);
    ShellExecuteW(hwnd, op.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
}

pub const INTERNET_OPEN_TYPE_PRECONFIG: DWORD = 0;
pub const INTERNET_FLAG_RELOAD: DWORD = 0x80000000;
pub const INTERNET_FLAG_NO_CACHE_WRITE: DWORD = 0x04000000;
pub const INTERNET_FLAG_SECURE: DWORD = 0x00800000;
pub const INTERNET_OPTION_CONNECT_TIMEOUT: DWORD = 2;
pub const INTERNET_OPTION_RECEIVE_TIMEOUT: DWORD = 6;
pub const INTERNET_OPTION_HTTP_DECODING: DWORD = 65;
pub const HTTP_QUERY_STATUS_CODE: DWORD = 19;
pub const HTTP_QUERY_FLAG_NUMBER: DWORD = 0x20000000;

#[link(name = "wininet")]
extern "system" {
    pub fn InternetOpenW(
        lpsz_agent: *const u16,
        dw_access_type: DWORD,
        lpsz_proxy: *const u16,
        lpsz_proxy_bypass: *const u16,
        dw_flags: DWORD,
    ) -> HANDLE;
    pub fn InternetOpenUrlW(
        h_internet: HANDLE,
        lpsz_url: *const u16,
        lpsz_headers: *const u16,
        dw_headers_length: DWORD,
        dw_flags: DWORD,
        dw_context: usize,
    ) -> HANDLE;
    pub fn HttpQueryInfoW(
        h_request: HANDLE,
        dw_info_level: DWORD,
        lp_buffer: *mut c_void,
        lpdw_buffer_length: *mut DWORD,
        lpdw_index: *mut DWORD,
    ) -> BOOL;
    pub fn InternetReadFile(
        h_file: HANDLE,
        lp_buffer: *mut c_void,
        dw_number_of_bytes_to_read: DWORD,
        lpdw_number_of_bytes_read: *mut DWORD,
    ) -> BOOL;
    pub fn InternetSetOptionW(
        h_internet: HANDLE,
        dw_option: DWORD,
        lp_buffer: *const c_void,
        dw_buffer_length: DWORD,
    ) -> BOOL;
    pub fn InternetCloseHandle(h_internet: HANDLE) -> BOOL;
}

/// Fetches a UTF-8 text payload over HTTPS/HTTP using native Windows WinINet (zero external crates).
pub unsafe fn http_get_text(url: &str, max_bytes: usize) -> Result<String, String> {
    let agent = wide("GameMaster6/1.0");
    let h_inet = InternetOpenW(
        agent.as_ptr(),
        INTERNET_OPEN_TYPE_PRECONFIG,
        std::ptr::null(),
        std::ptr::null(),
        0,
    );
    if h_inet.is_null() {
        return Err("WinINet InternetOpenW failed".into());
    }
    let timeout_ms: DWORD = 8000;
    InternetSetOptionW(
        h_inet,
        INTERNET_OPTION_CONNECT_TIMEOUT,
        &timeout_ms as *const DWORD as *const c_void,
        std::mem::size_of::<DWORD>() as DWORD,
    );
    InternetSetOptionW(
        h_inet,
        INTERNET_OPTION_RECEIVE_TIMEOUT,
        &timeout_ms as *const DWORD as *const c_void,
        std::mem::size_of::<DWORD>() as DWORD,
    );
    let enable_decoding: BOOL = 1;
    InternetSetOptionW(
        h_inet,
        INTERNET_OPTION_HTTP_DECODING,
        &enable_decoding as *const BOOL as *const c_void,
        std::mem::size_of::<BOOL>() as DWORD,
    );

    let url_w = wide(url);
    let headers_w = wide("Accept-Encoding: gzip, deflate\r\n");
    let headers_len = headers_w.len().saturating_sub(1) as DWORD;
    let flags = INTERNET_FLAG_RELOAD
        | INTERNET_FLAG_NO_CACHE_WRITE
        | if url.starts_with("https://") { INTERNET_FLAG_SECURE } else { 0 };
    let h_url = InternetOpenUrlW(
        h_inet,
        url_w.as_ptr(),
        headers_w.as_ptr(),
        headers_len,
        flags,
        0,
    );
    if h_url.is_null() {
        InternetCloseHandle(h_inet);
        return Err("Could not connect to URL".into());
    }

    let mut status: DWORD = 0;
    let mut status_len: DWORD = std::mem::size_of::<DWORD>() as DWORD;
    let mut index: DWORD = 0;
    if HttpQueryInfoW(
        h_url,
        HTTP_QUERY_STATUS_CODE | HTTP_QUERY_FLAG_NUMBER,
        &mut status as *mut DWORD as *mut c_void,
        &mut status_len,
        &mut index,
    ) != 0
        && !(200..300).contains(&status)
    {
        InternetCloseHandle(h_url);
        InternetCloseHandle(h_inet);
        return Err(format!("HTTP {status}"));
    }

    let mut out = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let mut read: DWORD = 0;
        let ok = InternetReadFile(
            h_url,
            chunk.as_mut_ptr() as *mut c_void,
            chunk.len() as DWORD,
            &mut read,
        );
        if ok == 0 || read == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..read as usize]);
        if out.len() >= max_bytes {
            break;
        }
    }

    InternetCloseHandle(h_url);
    InternetCloseHandle(h_inet);
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Reads Windows WDDM GPU timeout `TdrDelay` (seconds). Returns None if default (2s).
pub unsafe fn read_tdr_delay_secs() -> Option<u32> {
    let subkey = wide(r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers");
    let mut hkey: HKEY = std::ptr::null_mut();
    let rc = RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey);
    if rc != 0 || hkey.is_null() {
        return None;
    }
    let val_name = wide("TdrDelay");
    let mut reg_type: DWORD = 0;
    let mut data: u32 = 0;
    let mut cb: DWORD = std::mem::size_of::<u32>() as DWORD;
    let qrc = RegQueryValueExW(
        hkey,
        val_name.as_ptr(),
        std::ptr::null_mut(),
        &mut reg_type,
        &mut data as *mut u32 as *mut u8,
        &mut cb,
    );
    RegCloseKey(hkey);
    if qrc == 0 && reg_type == REG_DWORD {
        Some(data)
    } else {
        None
    }
}

/// Creates a zero-byte NTFS hardlink (instant on same drive), falling back to file copy across drives.
pub fn hardlink_or_copy(src_file: &Path, dst_file: &Path) -> Result<&'static str, String> {
    if dst_file.exists() {
        return Ok("already present");
    }
    let dst_w = wide(&dst_file.to_string_lossy());
    let src_w = wide(&src_file.to_string_lossy());
    let ok = unsafe { CreateHardLinkW(dst_w.as_ptr(), src_w.as_ptr(), std::ptr::null_mut()) };
    if ok != 0 {
        return Ok("hardlinked (0 extra bytes)");
    }
    std::fs::copy(src_file, dst_file)
        .map(|_| "copied")
        .map_err(|e| format!("Could not link or copy executable: {e}"))
}

#[link(name = "winmm")]
extern "system" {
    pub fn timeBeginPeriod(u_period: UINT) -> UINT;
    pub fn timeEndPeriod(u_period: UINT) -> UINT;
}

/// Enables or releases Windows 1ms multimedia timer resolution for smoother frame pacing.
pub unsafe fn set_timer_1ms(enable: bool) {
    if enable {
        timeBeginPeriod(1);
    } else {
        timeEndPeriod(1);
    }
}

/// Terminates stray background runner processes so they do not hold VRAM or cache locks.

pub fn kill_stray_runners() -> usize {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut killed = 0usize;
    for img in ["GameRunner.exe", "victory.exe", "victory-diag.exe"] {
        if let Ok(out) = std::process::Command::new("taskkill")
            .args(["/F", "/IM", img])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            if out.status.success() {
                killed += 1;
            }
        }
    }
    killed
}

#[repr(C)]
#[derive(Default)]
struct XInputGamepad {
    w_buttons: WORD,
    b_left_trigger: u8,
    b_right_trigger: u8,
    s_thumb_lx: i16,
    s_thumb_ly: i16,
    s_thumb_rx: i16,
    s_thumb_ry: i16,
}

#[repr(C)]
#[derive(Default)]
struct XInputState {
    packet_number: DWORD,
    gamepad: XInputGamepad,
}

#[repr(C)]
#[derive(Default)]
struct XInputVibration {
    w_left_motor_speed: WORD,
    w_right_motor_speed: WORD,
}

#[repr(C)]
#[derive(Default)]
struct XInputBatteryInfo {
    battery_type: u8,
    battery_level: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct XInputPadInfo {
    pub slot: u32,
    pub connected: bool,
    pub battery_label: &'static str,
    pub buttons: u16,
}

type XInputGetStateFn = unsafe extern "system" fn(DWORD, *mut XInputState) -> DWORD;
type XInputSetStateFn = unsafe extern "system" fn(DWORD, *mut XInputVibration) -> DWORD;
type XInputGetBatteryFn = unsafe extern "system" fn(DWORD, u8, *mut XInputBatteryInfo) -> DWORD;

#[derive(Clone, Copy)]
struct XInputApi {
    get_state: XInputGetStateFn,
    set_state: Option<XInputSetStateFn>,
    get_battery: Option<XInputGetBatteryFn>,
}

unsafe fn load_xinput_api() -> Option<XInputApi> {
    static API: std::sync::OnceLock<Option<XInputApi>> = std::sync::OnceLock::new();
    *API.get_or_init(|| {
        for dll in ["xinput1_4.dll", "xinput9_1_0.dll", "xinput1_3.dll"] {
            let w = wide(dll);
            let h = LoadLibraryW(w.as_ptr());
            if !h.is_null() {
                let gs = GetProcAddress(h, b"XInputGetState\0".as_ptr());
                if !gs.is_null() {
                    let ss = GetProcAddress(h, b"XInputSetState\0".as_ptr());
                    let gb = GetProcAddress(h, b"XInputGetBatteryInformation\0".as_ptr());
                    return Some(XInputApi {
                        get_state: std::mem::transmute(gs),
                        set_state: if ss.is_null() { None } else { Some(std::mem::transmute(ss)) },
                        get_battery: if gb.is_null() { None } else { Some(std::mem::transmute(gb)) },
                    });
                }
            }
        }
        None
    })
}

/// Polls all 4 XInput controller slots with battery & button state via xinput1_4.dll.
pub unsafe fn poll_xinput_pads() -> [XInputPadInfo; 4] {
    let mut out = [
        XInputPadInfo { slot: 0, connected: false, battery_label: "OFF", buttons: 0 },
        XInputPadInfo { slot: 1, connected: false, battery_label: "OFF", buttons: 0 },
        XInputPadInfo { slot: 2, connected: false, battery_label: "OFF", buttons: 0 },
        XInputPadInfo { slot: 3, connected: false, battery_label: "OFF", buttons: 0 },
    ];
    let Some(api) = load_xinput_api() else {
        return out;
    };
    for idx in 0..4usize {
        let mut st = XInputState::default();
        if (api.get_state)(idx as DWORD, &mut st) == 0 {
            let mut bat_lbl = "CONNECTED";
            if let Some(gb) = api.get_battery {
                let mut bi = XInputBatteryInfo::default();
                if gb(idx as DWORD, 0, &mut bi) == 0 {
                    bat_lbl = match bi.battery_type {
                        1 => "WIRED",
                        2 | 3 => match bi.battery_level {
                            0 => "EMPTY",
                            1 => "LOW",
                            2 => "MED",
                            _ => "FULL",
                        },
                        _ => "READY",
                    };
                }
            }
            out[idx] = XInputPadInfo {
                slot: idx as u32,
                connected: true,
                battery_label: bat_lbl,
                buttons: st.gamepad.w_buttons,
            };
        }
    }
    out
}

/// Fires a brief vibration pulse on the specified XInput controller slot (0..=3) in a background thread.
pub unsafe fn rumble_xinput_pad(slot: u32, duration_ms: u64) -> bool {
    let Some(api) = load_xinput_api() else {
        return false;
    };
    let Some(set_fn) = api.set_state else {
        return false;
    };
    let mut vib = XInputVibration {
        w_left_motor_speed: 36000,
        w_right_motor_speed: 24000,
    };
    if set_fn(slot, &mut vib) != 0 {
        return false;
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(duration_ms));
        let mut stop = XInputVibration::default();
        let _ = set_fn(slot, &mut stop);
    });
    true
}

/// Dynamically probes xinput1_4.dll / xinput9_1_0.dll and returns how many XInput pads are connected (0..=4).
pub unsafe fn detect_xinput_pads() -> u32 {
    poll_xinput_pads().iter().filter(|p| p.connected).count() as u32
}

const CF_UNICODETEXT: UINT = 13;
const GMEM_MOVEABLE: UINT = 0x0002;

/// Reads UTF-16LE text (`CF_UNICODETEXT`) from the Windows clipboard safely.
pub unsafe fn paste_from_clipboard(hwnd: HWND) -> Option<String> {
    if OpenClipboard(hwnd) == 0 {
        return None;
    }
    let hmem = GetClipboardData(CF_UNICODETEXT);
    let res = if !hmem.is_null() {
        let ptr = GlobalLock(hmem) as *const u16;
        if !ptr.is_null() {
            let mut len = 0usize;
            while len < 8192 && *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            let s = String::from_utf16_lossy(slice);
            GlobalUnlock(hmem);
            Some(s)
        } else {
            None
        }
    } else {
        None
    };
    CloseClipboard();
    res
}

/// Copies UTF-8 text into the Windows clipboard as UTF-16LE (`CF_UNICODETEXT`).
pub unsafe fn copy_to_clipboard(hwnd: HWND, text: &str) -> bool {
    if OpenClipboard(hwnd) == 0 {
        return false;
    }
    EmptyClipboard();
    let w = wide(text);
    let bytes = w.len() * 2;
    let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if hmem.is_null() {
        CloseClipboard();
        return false;
    }
    let ptr = GlobalLock(hmem) as *mut u16;
    if !ptr.is_null() {
        std::ptr::copy_nonoverlapping(w.as_ptr(), ptr, w.len());
        GlobalUnlock(hmem);
        SetClipboardData(CF_UNICODETEXT, hmem);
        CloseClipboard();
        true
    } else {
        CloseClipboard();
        false
    }
}

/// Creates a Windows Desktop shortcut (.lnk) pointing to Game Master.exe with logo.ico.
pub fn create_desktop_shortcut(exe_path: &Path, icon_path: &Path) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let exe_s = exe_path.to_string_lossy().replace('\'', "''");
    let dir_s = exe_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_string_lossy()
        .replace('\'', "''");
    let ico_s = icon_path.to_string_lossy().replace('\'', "''");
    let ps = format!(
        "$ws = New-Object -ComObject WScript.Shell;          $lnk = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Game Master 6.lnk';          $sc = $ws.CreateShortcut($lnk);          $sc.TargetPath = '{exe_s}';          $sc.WorkingDirectory = '{dir_s}';          if (Test-Path '{ico_s}') {{ $sc.IconLocation = '{ico_s}'; }}          $sc.Save();          Write-Output $lnk"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to invoke PowerShell: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOZORDER: UINT = 0x0004;
pub const SWP_NOACTIVATE: UINT = 0x0010;

/// Probes the discrete GPU name and driver version via WMI/CIM without flashing a console.
pub fn probe_primary_gpu() -> (String, String) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let ps = "Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name + '|' + $_.DriverVersion }";
    let Ok(out) = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return ("Direct3D 12 GPU".into(), String::new());
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut best: Option<(String, String)> = None;
    for line in text.lines() {
        let l = line.trim();
        if let Some((name, drv)) = l.split_once('|') {
            let n = name.trim();
            let d = drv.trim();
            if n.is_empty() {
                continue;
            }
            let lo = n.to_ascii_lowercase();
            // Prefer discrete NVIDIA / RTX / RX GPU over integrated graphics
            if lo.contains("nvidia") || lo.contains("rtx") || lo.contains("radeon rx") {
                return (n.to_string(), d.to_string());
            }
            if best.is_none() {
                best = Some((n.to_string(), d.to_string()));
            }
        }
    }
    best.unwrap_or_else(|| ("Direct3D 12 GPU".into(), String::new()))
}

/// Enables Windows 10/11 native dark titlebar via dwmapi.dll (dynamic load, zero link issues).
pub unsafe fn enable_dark_titlebar(hwnd: HWND) {
    type DwmSetWindowAttributeFn = unsafe extern "system" fn(HWND, DWORD, *const c_void, DWORD) -> i32;
    let dll = wide("dwmapi.dll");
    let h = LoadLibraryW(dll.as_ptr());
    if h.is_null() {
        return;
    }
    let proc = GetProcAddress(h, b"DwmSetWindowAttribute\0".as_ptr());
    if proc.is_null() {
        return;
    }
    let f: DwmSetWindowAttributeFn = std::mem::transmute(proc);
    let val: DWORD = 1;
    // Try Win11/Win10 20H1+ attribute 20, fallback to 19
    let _ = f(hwnd, 20, &val as *const DWORD as *const c_void, 4);
    let _ = f(hwnd, 19, &val as *const DWORD as *const c_void, 4);
}

/// Enables Per-Monitor DPI Awareness v2 on Windows 10/11 so 125%/150% scaling stays sharp,
/// falling back to `SetProcessDPIAware()` on older systems.
pub unsafe fn enable_dpi_awareness() {
    type SetProcessDpiAwarenessContextFn = unsafe extern "system" fn(isize) -> BOOL;
    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
    let dll = wide("user32.dll");
    let h = GetModuleHandleW(dll.as_ptr());
    if !h.is_null() {
        let proc = GetProcAddress(h as HANDLE, b"SetProcessDpiAwarenessContext\0".as_ptr());
        if !proc.is_null() {
            let f: SetProcessDpiAwarenessContextFn = std::mem::transmute(proc);
            if f(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0 {
                return;
            }
        }
    }
    SetProcessDPIAware();
}

/// Captures the primary display to a 24-bit BMP file using pure Win32 GDI (zero external crates).
pub unsafe fn capture_screen_bmp(dst_path: &Path) -> Result<(), String> {
    let w = GetSystemMetrics(SM_CXSCREEN).max(1);
    let h = GetSystemMetrics(SM_CYSCREEN).max(1);
    let screen_dc = GetDC(std::ptr::null_mut());
    if screen_dc.is_null() {
        return Err("GetDC(NULL) failed".into());
    }
    let mem_dc = CreateCompatibleDC(screen_dc);
    let bmp = CreateCompatibleBitmap(screen_dc, w, h);
    let old = SelectObject(mem_dc, bmp);
    BitBlt(mem_dc, 0, 0, w, h, screen_dc, 0, 0, SRCCOPY);

    let row_stride = ((w * 3 + 3) & !3) as usize;
    let img_bytes = row_stride * (h as usize);
    let mut pixels = vec![0u8; img_bytes];
    let mut bmi = BITMAPINFO::default();
    bmi.bmi_header.bi_size = std::mem::size_of::<BITMAPINFOHEADER>() as DWORD;
    bmi.bmi_header.bi_width = w;
    bmi.bmi_header.bi_height = h;
    bmi.bmi_header.bi_planes = 1;
    bmi.bmi_header.bi_bit_count = 24;
    bmi.bmi_header.bi_compression = 0;
    GetDIBits(mem_dc, bmp, 0, h as UINT, pixels.as_mut_ptr() as *mut c_void, &mut bmi, DIB_RGB_COLORS);

    SelectObject(mem_dc, old);
    DeleteObject(bmp);
    DeleteDC(mem_dc);
    ReleaseDC(std::ptr::null_mut(), screen_dc);

    let file_size = (54 + img_bytes) as u32;
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(img_bytes as u32).to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&pixels);
    std::fs::write(dst_path, out).map_err(|e| format!("Failed to save screenshot: {e}"))
}

/// Opens a clean native Windows text-input dialog (dark WinForms) without flashing a console.
pub fn prompt_text_input(title: &str, prompt: &str, default_val: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let t = title.replace('\'', "''");
    let pr = prompt.replace('\'', "''");
    let dv = default_val.replace('\'', "''");
    let ps = format!(
        "Add-Type -AssemblyName System.Windows.Forms,System.Drawing;          $f = New-Object Windows.Forms.Form; $f.Text = '{t}'; $f.Width = 540; $f.Height = 175;          $f.StartPosition = 'CenterScreen'; $f.FormBorderStyle = 'FixedDialog'; $f.MaximizeBox = $false;          $f.BackColor = [Drawing.Color]::FromArgb(20,15,34); $f.ForeColor = [Drawing.Color]::White;          $l = New-Object Windows.Forms.Label; $l.Text = '{pr}'; $l.Left = 14; $l.Top = 14; $l.Width = 496; $l.Height = 32;          $tb = New-Object Windows.Forms.TextBox; $tb.Text = '{dv}'; $tb.Left = 14; $tb.Top = 52; $tb.Width = 496;          $tb.BackColor = [Drawing.Color]::FromArgb(30,24,48); $tb.ForeColor = [Drawing.Color]::Cyan;          $ok = New-Object Windows.Forms.Button; $ok.Text = 'Save'; $ok.Left = 340; $ok.Top = 92; $ok.Width = 80; $ok.DialogResult = 'OK';          $cn = New-Object Windows.Forms.Button; $cn.Text = 'Cancel'; $cn.Left = 430; $cn.Top = 92; $cn.Width = 80; $cn.DialogResult = 'Cancel';          $f.AcceptButton = $ok; $f.CancelButton = $cn; $f.Controls.AddRange(@($l,$tb,$ok,$cn));          if ($f.ShowDialog() -eq 'OK') {{ Write-Output ('OK:' + $tb.Text) }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.starts_with("OK:"))?;
    line.strip_prefix("OK:").map(|s| s.trim().to_string())
}

/// Probes true 64-bit dedicated VRAM (GB) for discrete GPUs from Windows Display Class registry.
pub fn probe_vram_gb() -> f64 {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let ps = r"(Get-ItemProperty -Path 'HKLM:\SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0*' -Name 'HardwareInformation.qwMemorySize' -ErrorAction SilentlyContinue | Measure-Object -Property 'HardwareInformation.qwMemorySize' -Maximum).Maximum";
    let Ok(out) = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return 0.0;
    };
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    s.parse::<u64>().map(|b| b as f64 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(0.0)
}


/// Maximizes the active runner's top-level window (by PID) once its game window appears.
pub unsafe fn maximize_runner_window(pid: u32) -> bool {
    struct Ctx {
        pid: u32,
        found: HWND,
    }
    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut Ctx);
        let mut wpid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut wpid);
        if wpid == ctx.pid && IsWindowVisible(hwnd) != 0 {
            ctx.found = hwnd;
            return 0;
        }
        1
    }
    let mut ctx = Ctx {
        pid,
        found: std::ptr::null_mut(),
    };
    EnumWindows(Some(enum_cb), &mut ctx as *mut Ctx as LPARAM);
    if ctx.found.is_null() {
        return false;
    }
    let _ = ShowWindow(ctx.found, SW_SHOWMAXIMIZED);
    let _ = SetForegroundWindow(ctx.found);
    true
}

/// Automatically finds the active runner's top-level window (by PID) and brands it with
/// "Game Master 6 - Title" and Game Master 6's `logo.ico` icon.
pub unsafe fn brand_runner_window(pid: u32, title_text: &str, icon_path: &Path, clean_caption: bool) -> bool {
    struct Ctx {
        pid: u32,
        found: HWND,
    }
    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut Ctx);
        let mut wpid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut wpid);
        if wpid == ctx.pid && IsWindowVisible(hwnd) != 0 {
            ctx.found = hwnd;
            return 0;
        }
        1
    }
    let mut ctx = Ctx {
        pid,
        found: std::ptr::null_mut(),
    };
    EnumWindows(Some(enum_cb), &mut ctx as *mut Ctx as LPARAM);
    if ctx.found.is_null() {
        return false;
    }
    let ico_w = wide(&icon_path.to_string_lossy());
    let big_ico = LoadImageW(std::ptr::null_mut(), ico_w.as_ptr(), IMAGE_ICON, 48, 48, LR_LOADFROMFILE);
    let sm_ico = LoadImageW(std::ptr::null_mut(), ico_w.as_ptr(), IMAGE_ICON, 16, 16, LR_LOADFROMFILE);
    if !big_ico.is_null() {
        let _ = PostMessageW(ctx.found, WM_SETICON, ICON_BIG, big_ico as LPARAM);
    }
    if !sm_ico.is_null() {
        let _ = PostMessageW(ctx.found, WM_SETICON, ICON_SMALL, sm_ico as LPARAM);
    }
    if clean_caption {
        let tw = wide(title_text);
        let _ = SetWindowTextW(ctx.found, tw.as_ptr());
    }
    true
}

/// Returns the window rect of the foreground runner window (if focused).
pub unsafe fn foreground_game_rect() -> Option<RECT> {
    let fg = GetForegroundWindow();
    if fg.is_null() {
        return None;
    }
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(fg, buf.as_mut_ptr(), buf.len() as c_int);
    if len <= 0 {
        return None;
    }
    let title = String::from_utf16_lossy(&buf[..len as usize]);
    let lower = title.to_ascii_lowercase();
    if lower.contains("game master") {
        return None;
    }
    if !lower.contains("victory") {
        return None;
    }
    let mut rc = RECT::new(0, 0, 0, 0);
    if GetWindowRect(fg, &mut rc) != 0 && rc.width() >= 320 && rc.height() >= 200 {
        Some(rc)
    } else {
        None
    }
}

#[repr(C)]
struct SYSTEM_POWER_STATUS {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: DWORD,
    battery_full_life_time: DWORD,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(lp_system_power_status: *mut SYSTEM_POWER_STATUS) -> BOOL;
}

/// Returns true only when Windows explicitly reports running on battery (ACLineStatus == 0).
pub fn is_on_battery_power() -> bool {
    let mut sps = SYSTEM_POWER_STATUS {
        ac_line_status: 255,
        battery_flag: 0,
        battery_life_percent: 255,
        system_status_flag: 0,
        battery_life_time: 0,
        battery_full_life_time: 0,
    };
    unsafe { GetSystemPowerStatus(&mut sps) != 0 && sps.ac_line_status == 0 }
}
