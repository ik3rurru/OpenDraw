use std::{ffi::c_void, io, mem::size_of, ptr};

use crate::graphics::{Color, FrameBuffer};

use super::ffi::*;

#[derive(Default)]
struct WindowState {
    framebuffer: FrameBuffer,
}

pub fn run() -> io::Result<()> {
    let class_name = wide("OpenDrawWindow");
    let title = wide("OpenDraw");
    let mut state = Box::new(WindowState::default());

    // SAFETY: Every pointer passed to Win32 remains valid for the duration documented
    // below. WindowState is boxed so callbacks keep a stable address until WM_QUIT.
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        if instance.is_null() {
            return Err(io::Error::last_os_error());
        }

        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: ptr::null_mut(),
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        if RegisterClassW(&window_class) == 0 {
            return Err(io::Error::last_os_error());
        }

        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            960,
            640,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            (&mut *state as *mut WindowState).cast(),
        );
        if window.is_null() {
            return Err(io::Error::last_os_error());
        }

        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);

        let mut message = MSG::default();
        loop {
            match GetMessageW(&mut message, ptr::null_mut(), 0, 0) {
                -1 => return Err(io::Error::last_os_error()),
                0 => break,
                _ => {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }

    Ok(())
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        // SAFETY: During WM_NCCREATE, lparam points to a CREATESTRUCTW owned by Win32.
        // lpCreateParams is the live boxed WindowState supplied to CreateWindowExW.
        let create = unsafe { &*(lparam as *const CREATESTRUCTW) };
        unsafe { SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize) };
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }

    // SAFETY: GWLP_USERDATA is set above before later messages are dispatched and the
    // boxed state outlives the native window/message loop. No other code mutates it.
    let state = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut WindowState };

    match message {
        WM_SIZE if !state.is_null() => {
            let mut rect = RECT::default();
            if unsafe { GetClientRect(window, &mut rect) } != 0 {
                let width = (rect.right - rect.left).max(0) as u32;
                let height = (rect.bottom - rect.top).max(0) as u32;
                let framebuffer = unsafe { &mut (*state).framebuffer };
                framebuffer.resize(width, height);
                framebuffer.checkerboard(24, Color::rgb(224, 224, 224), Color::rgb(176, 176, 176));
                unsafe { InvalidateRect(window, ptr::null(), 0) };
            }
            0
        }
        WM_PAINT if !state.is_null() => {
            let mut paint = PAINTSTRUCT::default();
            let dc = unsafe { BeginPaint(window, &mut paint) };
            if !dc.is_null() {
                unsafe { present(dc, &(*state).framebuffer) };
            }
            unsafe { EndPaint(window, &paint) };
            0
        }
        WM_ERASEBKGND => 1,
        WM_CLOSE => {
            unsafe { DestroyWindow(window) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

unsafe fn present(dc: HDC, framebuffer: &FrameBuffer) {
    if framebuffer.width == 0 || framebuffer.height == 0 {
        return;
    }

    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative height tells GDI that our first row is the top row.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };

    // SAFETY: pixels contains width*height contiguous u32 values and remains borrowed
    // for the synchronous StretchDIBits call. BITMAPINFO describes the same dimensions.
    unsafe {
        StretchDIBits(
            dc,
            0,
            0,
            width,
            height,
            0,
            0,
            width,
            height,
            framebuffer.pixels.as_ptr().cast::<c_void>(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
