#![allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::upper_case_acronyms
)]

use std::ffi::c_void;

pub type BOOL = i32;
pub type UINT = u32;
pub type DWORD = u32;
pub type LONG = i32;
pub type WORD = u16;
pub type LPARAM = isize;
pub type WPARAM = usize;
pub type LRESULT = isize;
pub type ATOM = u16;
pub type HANDLE = *mut c_void;
pub type HWND = HANDLE;
pub type HINSTANCE = HANDLE;
pub type HICON = HANDLE;
pub type HCURSOR = HANDLE;
pub type HBRUSH = HANDLE;
pub type HMENU = HANDLE;
pub type HDC = HANDLE;

pub const CS_VREDRAW: UINT = 0x0001;
pub const CS_HREDRAW: UINT = 0x0002;
pub const WS_OVERLAPPEDWINDOW: DWORD = 0x00cf_0000;
pub const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
pub const SW_SHOW: i32 = 5;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_PAINT: UINT = 0x000f;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_NCCREATE: UINT = 0x0081;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_CHAR: UINT = 0x0102;
pub const WM_SYSKEYDOWN: UINT = 0x0104;
pub const WM_SYSKEYUP: UINT = 0x0105;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_RBUTTONDOWN: UINT = 0x0204;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_MBUTTONDOWN: UINT = 0x0207;
pub const WM_MBUTTONUP: UINT = 0x0208;
pub const WM_MOUSEWHEEL: UINT = 0x020a;
pub const GWLP_USERDATA: i32 = -21;
pub const IDC_ARROW: *const u16 = 32512_usize as *const u16;
pub const VK_BACK: u32 = 0x08;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0d;
pub const VK_SHIFT: u32 = 0x10;
pub const VK_CONTROL: u32 = 0x11;
pub const VK_MENU: u32 = 0x12;
pub const VK_ESCAPE: u32 = 0x1b;
pub const VK_SPACE: u32 = 0x20;
pub const VK_PRIOR: u32 = 0x21;
pub const VK_NEXT: u32 = 0x22;
pub const VK_END: u32 = 0x23;
pub const VK_HOME: u32 = 0x24;
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;
pub const VK_DELETE: u32 = 0x2e;
pub const BI_RGB: DWORD = 0;
pub const DIB_RGB_COLORS: UINT = 0;
pub const SRCCOPY: DWORD = 0x00cc_0020;

pub type WndProc = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
pub struct WNDCLASSW {
    pub style: UINT,
    pub lpfnWndProc: WndProc,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

#[repr(C)]
#[derive(Default)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)]
#[derive(Default)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
    pub lPrivate: DWORD,
}

#[repr(C)]
#[derive(Default)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

impl Default for PAINTSTRUCT {
    fn default() -> Self {
        // A zeroed PAINTSTRUCT is the input required by BeginPaint.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct CREATESTRUCTW {
    pub lpCreateParams: *mut c_void,
    pub hInstance: HINSTANCE,
    pub hMenu: HMENU,
    pub hwndParent: HWND,
    pub cy: i32,
    pub cx: i32,
    pub y: i32,
    pub x: i32,
    pub style: LONG,
    pub lpszName: *const u16,
    pub lpszClass: *const u16,
    pub dwExStyle: DWORD,
}

#[repr(C)]
pub struct BITMAPINFOHEADER {
    pub biSize: DWORD,
    pub biWidth: LONG,
    pub biHeight: LONG,
    pub biPlanes: WORD,
    pub biBitCount: WORD,
    pub biCompression: DWORD,
    pub biSizeImage: DWORD,
    pub biXPelsPerMeter: LONG,
    pub biYPelsPerMeter: LONG,
    pub biClrUsed: DWORD,
    pub biClrImportant: DWORD,
}

#[repr(C)]
pub struct RGBQUAD {
    pub rgbBlue: u8,
    pub rgbGreen: u8,
    pub rgbRed: u8,
    pub rgbReserved: u8,
}

#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [RGBQUAD; 1],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleW(module_name: *const u16) -> HINSTANCE;
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn RegisterClassW(window_class: *const WNDCLASSW) -> ATOM;
    pub fn CreateWindowExW(
        ex_style: DWORD,
        class_name: *const u16,
        window_name: *const u16,
        style: DWORD,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        parameter: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(window: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    pub fn DestroyWindow(window: HWND) -> BOOL;
    pub fn PostQuitMessage(exit_code: i32);
    pub fn GetMessageW(message: *mut MSG, window: HWND, min: UINT, max: UINT) -> BOOL;
    pub fn TranslateMessage(message: *const MSG) -> BOOL;
    pub fn DispatchMessageW(message: *const MSG) -> LRESULT;
    pub fn ShowWindow(window: HWND, command: i32) -> BOOL;
    pub fn UpdateWindow(window: HWND) -> BOOL;
    pub fn LoadCursorW(instance: HINSTANCE, cursor_name: *const u16) -> HCURSOR;
    pub fn GetClientRect(window: HWND, rect: *mut RECT) -> BOOL;
    pub fn SetWindowLongPtrW(window: HWND, index: i32, value: isize) -> isize;
    pub fn GetWindowLongPtrW(window: HWND, index: i32) -> isize;
    pub fn SetCapture(window: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn InvalidateRect(window: HWND, rect: *const RECT, erase: BOOL) -> BOOL;
    pub fn BeginPaint(window: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(window: HWND, paint: *const PAINTSTRUCT) -> BOOL;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub fn StretchDIBits(
        dc: HDC,
        x_dest: i32,
        y_dest: i32,
        dest_width: i32,
        dest_height: i32,
        x_src: i32,
        y_src: i32,
        src_width: i32,
        src_height: i32,
        bits: *const c_void,
        info: *const BITMAPINFO,
        usage: UINT,
        raster_operation: DWORD,
    ) -> i32;
}
