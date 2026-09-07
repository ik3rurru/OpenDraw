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
pub const WM_QUIT: UINT = 0x0012;
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
pub const WM_POINTERUPDATE: UINT = 0x0245;
pub const WM_POINTERDOWN: UINT = 0x0246;
pub const WM_POINTERUP: UINT = 0x0247;
pub const WM_POINTERENTER: UINT = 0x0249;
pub const WM_POINTERLEAVE: UINT = 0x024a;
pub const PM_REMOVE: UINT = 0x0001;
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
pub const GDI_ERROR: i32 = -1;
pub const MOVEFILE_REPLACE_EXISTING: DWORD = 0x0000_0001;
pub const MOVEFILE_WRITE_THROUGH: DWORD = 0x0000_0008;
pub const MB_YESNOCANCEL: UINT = 0x0000_0003;
pub const MB_ICONWARNING: UINT = 0x0000_0030;
pub const IDCANCEL: i32 = 2;
pub const IDYES: i32 = 6;
pub const IDNO: i32 = 7;
pub const OFN_OVERWRITEPROMPT: DWORD = 0x0000_0002;
pub const OFN_HIDEREADONLY: DWORD = 0x0000_0004;
pub const OFN_NOCHANGEDIR: DWORD = 0x0000_0008;
pub const OFN_PATHMUSTEXIST: DWORD = 0x0000_0800;
pub const OFN_FILEMUSTEXIST: DWORD = 0x0000_1000;
pub const OFN_EXPLORER: DWORD = 0x0008_0000;
pub const IMAGE_LOCK_MODE_READ: UINT = 1;
pub const IMAGE_LOCK_MODE_USER_INPUT_BUFFER: UINT = 4;
pub const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200a;
// POINTER_INPUT_TYPE value for a stylus.
pub const PT_PEN: u32 = 3;
// POINTER_INFO.pointerFlags bits.
pub const POINTER_FLAG_INRANGE: DWORD = 0x0000_0002;
pub const POINTER_FLAG_INCONTACT: DWORD = 0x0000_0004;
pub const POINTER_FLAG_FIRSTBUTTON: DWORD = 0x0000_0010;
pub const POINTER_FLAG_SECONDBUTTON: DWORD = 0x0000_0020;
// POINTER_PEN_INFO.penFlags bit: the eraser end of the stylus is in use.
pub const PEN_FLAGS_INVERTED: DWORD = 0x0000_0001;
// POINTER_PEN_INFO.penMask bits: which pen axes are valid in this sample.
pub const PEN_MASK_PRESSURE: DWORD = 0x0000_0001;
pub const PEN_MASK_ROTATION: DWORD = 0x0000_0002;
pub const PEN_MASK_TILT_X: DWORD = 0x0000_0004;
pub const PEN_MASK_TILT_Y: DWORD = 0x0000_0008;

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
#[derive(Clone, Copy, Default)]
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

#[repr(C)]
pub struct OPENFILENAMEW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hInstance: HINSTANCE,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: DWORD,
    pub nFilterIndex: DWORD,
    pub lpstrFile: *mut u16,
    pub nMaxFile: DWORD,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: DWORD,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: DWORD,
    pub nFileOffset: WORD,
    pub nFileExtension: WORD,
    pub lpstrDefExt: *const u16,
    pub lCustData: LPARAM,
    pub lpfnHook: *mut c_void,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut c_void,
    pub dwReserved: DWORD,
    pub FlagsEx: DWORD,
}

impl Default for OPENFILENAMEW {
    fn default() -> Self {
        // SAFETY: this C struct contains only integers and nullable handles/pointers;
        // Win32 requires every unused field to be zero.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct GDIPLUS_STARTUP_INPUT {
    pub GdiplusVersion: UINT,
    pub DebugEventCallback: *mut c_void,
    pub SuppressBackgroundThread: BOOL,
    pub SuppressExternalCodecs: BOOL,
}

#[repr(C)]
pub struct GDIP_RECT {
    pub X: i32,
    pub Y: i32,
    pub Width: i32,
    pub Height: i32,
}

#[repr(C)]
pub struct BITMAP_DATA {
    pub Width: UINT,
    pub Height: UINT,
    pub Stride: i32,
    pub PixelFormat: i32,
    pub Scan0: *mut c_void,
    pub Reserved: usize,
}

// Reproduces Win32 POINTER_INFO (winuser.h). Field order, types and alignment
// must match the native layout exactly: HANDLE is pointer-sized, which #[repr(C)]
// resolves per target exactly like the C compiler does.
#[repr(C)]
#[derive(Default)]
pub struct POINTER_INFO {
    pub pointer_type: u32,
    pub pointer_id: u32,
    pub frame_id: u32,
    pub pointer_flags: DWORD,
    pub h_target: HANDLE,
    pub pt_pixel_location: POINT,
    pub pt_himetric_location: POINT,
    pub dw_time: DWORD,
    pub history_count: u32,
    pub input_data: i32,
    pub key_states: DWORD,
    pub performance_count: u64,
}

// Reproduces Win32 POINTER_PEN_INFO (winuser.h).
#[repr(C)]
#[derive(Default)]
pub struct POINTER_PEN_INFO {
    pub pointer_info: POINTER_INFO,
    pub pen_flags: DWORD,
    pub pen_mask: DWORD,
    pub pressure: u32,
    pub rotation: u32,
    pub tilt_x: i32,
    pub tilt_y: i32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleW(module_name: *const u16) -> HINSTANCE;
    pub fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: DWORD) -> BOOL;
}

#[link(name = "gdiplus")]
unsafe extern "system" {
    pub fn GdiplusStartup(
        token: *mut usize,
        input: *const GDIPLUS_STARTUP_INPUT,
        output: *mut c_void,
    ) -> i32;
    pub fn GdiplusShutdown(token: usize);
    pub fn GdipCreateBitmapFromFile(filename: *const u16, bitmap: *mut *mut c_void) -> i32;
    pub fn GdipGetImageWidth(image: *mut c_void, width: *mut UINT) -> i32;
    pub fn GdipGetImageHeight(image: *mut c_void, height: *mut UINT) -> i32;
    pub fn GdipBitmapLockBits(
        bitmap: *mut c_void,
        rect: *const GDIP_RECT,
        flags: UINT,
        format: i32,
        locked: *mut BITMAP_DATA,
    ) -> i32;
    pub fn GdipBitmapUnlockBits(bitmap: *mut c_void, locked: *mut BITMAP_DATA) -> i32;
    pub fn GdipDisposeImage(image: *mut c_void) -> i32;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    pub fn GetOpenFileNameW(dialog: *mut OPENFILENAMEW) -> BOOL;
    pub fn GetSaveFileNameW(dialog: *mut OPENFILENAMEW) -> BOOL;
    pub fn CommDlgExtendedError() -> DWORD;
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
    pub fn PeekMessageW(
        message: *mut MSG,
        window: HWND,
        min: UINT,
        max: UINT,
        remove_message: UINT,
    ) -> BOOL;
    pub fn TranslateMessage(message: *const MSG) -> BOOL;
    pub fn DispatchMessageW(message: *const MSG) -> LRESULT;
    pub fn ShowWindow(window: HWND, command: i32) -> BOOL;
    pub fn UpdateWindow(window: HWND) -> BOOL;
    pub fn MessageBoxW(window: HWND, text: *const u16, caption: *const u16, kind: UINT) -> i32;
    pub fn LoadCursorW(instance: HINSTANCE, cursor_name: *const u16) -> HCURSOR;
    pub fn GetClientRect(window: HWND, rect: *mut RECT) -> BOOL;
    pub fn SetWindowLongPtrW(window: HWND, index: i32, value: isize) -> isize;
    pub fn GetWindowLongPtrW(window: HWND, index: i32) -> isize;
    pub fn SetCapture(window: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn GetPointerType(pointer_id: u32, pointer_type: *mut u32) -> BOOL;
    pub fn GetPointerPenInfo(pointer_id: u32, pen_info: *mut POINTER_PEN_INFO) -> BOOL;
    pub fn GetPointerPenInfoHistory(
        pointer_id: u32,
        entries_count: *mut u32,
        pen_info: *mut POINTER_PEN_INFO,
    ) -> BOOL;
    pub fn ClientToScreen(window: HWND, point: *mut POINT) -> BOOL;
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
