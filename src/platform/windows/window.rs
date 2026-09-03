use std::{collections::VecDeque, ffi::c_void, io, mem::size_of, ptr};

use crate::{
    graphics::FrameBuffer,
    platform::{Event, Key, MouseButton},
};

use super::ffi::*;

#[derive(Default)]
struct WindowState {
    framebuffer: FrameBuffer,
    events: VecDeque<Event>,
    pending_high_surrogate: Option<u16>,
    destroyed: bool,
}

impl WindowState {
    fn push_text(&mut self, unit: u16) {
        // WM_CHAR delivers UTF-16 code units, so supplementary characters arrive
        // as two messages and must be combined before entering the common API.
        if (0xd800..=0xdbff).contains(&unit) {
            self.pending_high_surrogate = Some(unit);
            return;
        }

        let units = self.pending_high_surrogate.take().into_iter().chain([unit]);
        for character in char::decode_utf16(units) {
            self.events.push_back(Event::TextInput {
                character: character.unwrap_or(char::REPLACEMENT_CHARACTER),
            });
        }
    }
}

pub struct Window {
    handle: HWND,
    state: Box<WindowState>,
}

impl Window {
    pub fn new() -> io::Result<Self> {
        let class_name = wide("OpenDrawWindow");
        let title = wide("OpenDraw");
        let mut state = Box::new(WindowState::default());

        // SAFETY: The UTF-16 strings live through window creation. WindowState is
        // boxed, so the pointer stored in GWLP_USERDATA stays stable until Drop.
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

            let handle = CreateWindowExW(
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
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            ShowWindow(handle, SW_SHOW);
            UpdateWindow(handle);

            Ok(Self { handle, state })
        }
    }

    pub fn next_event(&mut self) -> io::Result<Option<Event>> {
        loop {
            if let Some(event) = self.state.events.pop_front() {
                return Ok(Some(event));
            }

            let mut message = MSG::default();
            let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
            match result {
                -1 => return Err(io::Error::last_os_error()),
                0 => return Ok(self.state.events.pop_front()),
                _ => unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                },
            };
        }
    }

    pub fn framebuffer(&mut self) -> &mut FrameBuffer {
        &mut self.state.framebuffer
    }

    pub fn poll_event(&mut self) -> Option<Event> {
        loop {
            if let Some(event) = self.state.events.pop_front() {
                return Some(event);
            }

            let mut message = MSG::default();
            if unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) } == 0 {
                return None;
            }
            if message.message == WM_QUIT {
                return Some(Event::CloseRequested);
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    pub fn present(&self) {
        unsafe {
            InvalidateRect(self.handle, ptr::null(), 0);
            UpdateWindow(self.handle);
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if !self.state.destroyed {
            // SAFETY: handle belongs to this Window and state remains alive while
            // DestroyWindow synchronously dispatches the final native messages.
            unsafe { DestroyWindow(self.handle) };
        }
    }
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
                let state = unsafe { &mut *state };
                state.framebuffer.resize(width, height);
                state.events.push_back(Event::Resized { width, height });
            }
            0
        }
        WM_MOUSEMOVE if !state.is_null() => {
            unsafe { &mut *state }.events.push_back(Event::MouseMove {
                x: signed_low_word(lparam),
                y: signed_high_word(lparam as usize),
            });
            0
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN if !state.is_null() => {
            unsafe { SetCapture(window) };
            unsafe { &mut *state }.events.push_back(Event::MouseDown {
                button: mouse_button(message),
            });
            0
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP if !state.is_null() => {
            unsafe { ReleaseCapture() };
            unsafe { &mut *state }.events.push_back(Event::MouseUp {
                button: mouse_button(message),
            });
            0
        }
        WM_MOUSEWHEEL if !state.is_null() => {
            unsafe { &mut *state }.events.push_back(Event::MouseWheel {
                delta: wheel_delta(wparam),
            });
            0
        }
        WM_KEYDOWN if !state.is_null() => {
            unsafe { &mut *state }.events.push_back(Event::KeyDown {
                key: key_from_virtual(wparam as u32),
            });
            0
        }
        WM_KEYUP if !state.is_null() => {
            unsafe { &mut *state }.events.push_back(Event::KeyUp {
                key: key_from_virtual(wparam as u32),
            });
            0
        }
        WM_SYSKEYDOWN | WM_SYSKEYUP if !state.is_null() => {
            let event = if message == WM_SYSKEYDOWN {
                Event::KeyDown {
                    key: key_from_virtual(wparam as u32),
                }
            } else {
                Event::KeyUp {
                    key: key_from_virtual(wparam as u32),
                }
            };
            unsafe { &mut *state }.events.push_back(event);
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
        WM_CHAR if !state.is_null() => {
            unsafe { &mut *state }.push_text(wparam as u16);
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
        WM_CLOSE if !state.is_null() => {
            unsafe { &mut *state }
                .events
                .push_back(Event::CloseRequested);
            0
        }
        WM_DESTROY => {
            if !state.is_null() {
                let state = unsafe { &mut *state };
                state.destroyed = true;
                state.events.push_back(Event::CloseRequested);
            }
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn signed_low_word(value: LPARAM) -> i32 {
    // Mouse coordinates are signed 16-bit values packed into LPARAM.
    value as u16 as i16 as i32
}

fn signed_high_word(value: usize) -> i32 {
    (value >> 16) as u16 as i16 as i32
}

fn wheel_delta(wparam: WPARAM) -> f32 {
    signed_high_word(wparam) as f32 / 120.0
}

fn mouse_button(message: UINT) -> MouseButton {
    match message {
        WM_LBUTTONDOWN | WM_LBUTTONUP => MouseButton::Left,
        WM_RBUTTONDOWN | WM_RBUTTONUP => MouseButton::Right,
        _ => MouseButton::Middle,
    }
}

fn key_from_virtual(key: u32) -> Key {
    match key {
        VK_BACK => Key::Backspace,
        VK_TAB => Key::Tab,
        VK_RETURN => Key::Enter,
        VK_SHIFT => Key::Shift,
        VK_CONTROL => Key::Control,
        VK_MENU => Key::Alt,
        VK_ESCAPE => Key::Escape,
        VK_SPACE => Key::Space,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,
        VK_END => Key::End,
        VK_HOME => Key::Home,
        VK_LEFT => Key::Left,
        VK_UP => Key::Up,
        VK_RIGHT => Key::Right,
        VK_DOWN => Key::Down,
        VK_DELETE => Key::Delete,
        0x30..=0x39 => Key::Digit((key - 0x30) as u8),
        0x41..=0x5a => Key::Letter(char::from_u32(key).unwrap()),
        0x70..=0x87 => Key::Function((key - 0x70 + 1) as u8),
        _ => Key::Unknown(key),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_native_input_without_leaking_win32_types() {
        let coordinates = (((-20_i16 as u16 as u32) << 16) | -10_i16 as u16 as u32) as isize;
        assert_eq!(signed_low_word(coordinates), -10);
        assert_eq!(signed_high_word(coordinates as usize), -20);
        assert_eq!(wheel_delta((120_u32 << 16) as usize), 1.0);
        assert_eq!(key_from_virtual(0x41), Key::Letter('A'));

        let mut state = WindowState::default();
        state.push_text(0xd83d);
        state.push_text(0xde00);
        assert_eq!(
            state.events.pop_front(),
            Some(Event::TextInput { character: '😀' })
        );
    }
}
