use std::{
    collections::VecDeque,
    ffi::{OsString, c_void},
    io,
    mem::size_of,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    ptr,
};

use crate::{
    document::MAX_PIXELS,
    graphics::FrameBuffer,
    platform::{DecodedImage, Event, Key, MouseButton, PenSample, PenTool, SaveChanges, normalize},
};

use super::ffi::*;

#[derive(Default)]
struct WindowState {
    framebuffer: FrameBuffer,
    events: VecDeque<Event>,
    pending_high_surrogate: Option<u16>,
    // Reused across WM_POINTERUPDATE batches so coalesced samples never
    // allocate per message.
    pen_history: Vec<POINTER_PEN_INFO>,
    // Retain ownership even if a later native query fails. A consumed pen
    // sequence must never switch back to emulated mouse messages halfway.
    pen_pointers: Vec<u32>,
    // Likewise, a pointer that began through DefWindowProc must finish there,
    // even if a later GetPointerType call could identify it as a pen.
    legacy_pointers: Vec<u32>,
    error: Option<io::Error>,
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

            if let Some(error) = state.error.take() {
                return Err(error);
            }

            Ok(Self { handle, state })
        }
    }

    pub fn next_event(&mut self) -> io::Result<Option<Event>> {
        loop {
            if let Some(error) = self.state.error.take() {
                return Err(error);
            }
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

    pub fn poll_event(&mut self) -> io::Result<Option<Event>> {
        loop {
            if let Some(error) = self.state.error.take() {
                return Err(error);
            }
            if let Some(event) = self.state.events.pop_front() {
                return Ok(Some(event));
            }

            let mut message = MSG::default();
            if unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) } == 0 {
                return Ok(None);
            }
            if message.message == WM_QUIT {
                return Ok(Some(Event::CloseRequested));
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    pub fn present(&mut self) -> io::Result<()> {
        unsafe {
            if InvalidateRect(self.handle, ptr::null(), 0) == 0 {
                return Err(io::Error::last_os_error());
            }
            UpdateWindow(self.handle);
        }
        self.state.error.take().map_or(Ok(()), Err)
    }

    pub fn confirm_save_changes(&self) -> io::Result<SaveChanges> {
        let message = wide("Save changes before continuing?");
        let title = wide("OpenDraw");
        let result = unsafe {
            MessageBoxW(
                self.handle,
                message.as_ptr(),
                title.as_ptr(),
                MB_YESNOCANCEL | MB_ICONWARNING,
            )
        };
        match result {
            IDYES => Ok(SaveChanges::Save),
            IDNO => Ok(SaveChanges::Discard),
            IDCANCEL => Ok(SaveChanges::Cancel),
            _ => Err(io::Error::last_os_error()),
        }
    }

    pub fn open_document_path(&self) -> io::Result<Option<PathBuf>> {
        self.document_path(false)
    }

    pub fn save_document_path(&self) -> io::Result<Option<PathBuf>> {
        self.document_path(true)
    }

    pub fn export_image_path(&self) -> io::Result<Option<(PathBuf, u32)>> {
        self.path_dialog(
            true,
            "PNG image (*.png)\0*.png\0BMP image (*.bmp)\0*.bmp\0",
            "Export image",
            None,
        )
    }

    pub fn import_image_path(&self) -> io::Result<Option<PathBuf>> {
        self.path_dialog(
            false,
            "PNG image (*.png)\0*.png\0",
            "Import PNG as layer",
            None,
        )
        .map(|selection| selection.map(|(path, _)| path))
    }

    fn document_path(&self, save: bool) -> io::Result<Option<PathBuf>> {
        self.path_dialog(
            save,
            "OpenDraw (*.odraw)\0*.odraw\0All files (*.*)\0*.*\0",
            if save {
                "Save OpenDraw document"
            } else {
                "Open OpenDraw document"
            },
            Some("odraw"),
        )
        .map(|selection| selection.map(|(path, _)| path))
    }

    fn path_dialog(
        &self,
        save: bool,
        filter: &str,
        title: &str,
        extension: Option<&str>,
    ) -> io::Result<Option<(PathBuf, u32)>> {
        // SAFETY: ReleaseCapture takes no pointers. A framebuffer button activates
        // on mouse-down, so the modal dialog must own subsequent pointer input.
        unsafe { ReleaseCapture() };
        let mut path = [0_u16; 32_768];
        let filter = wide(filter);
        let title = wide(title);
        let extension = extension.map(wide);
        let mut dialog = OPENFILENAMEW {
            lStructSize: size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: self.handle,
            lpstrFilter: filter.as_ptr(),
            nFilterIndex: 1,
            lpstrFile: path.as_mut_ptr(),
            nMaxFile: path.len() as u32,
            lpstrTitle: title.as_ptr(),
            lpstrDefExt: extension
                .as_ref()
                .map_or(ptr::null(), |extension| extension.as_ptr()),
            Flags: OFN_EXPLORER
                | OFN_NOCHANGEDIR
                | OFN_PATHMUSTEXIST
                | OFN_HIDEREADONLY
                | if save {
                    OFN_OVERWRITEPROMPT
                } else {
                    OFN_FILEMUSTEXIST
                },
            ..OPENFILENAMEW::default()
        };

        // SAFETY: OPENFILENAMEW and every UTF-16 buffer it references remain alive
        // for the synchronous modal call. nMaxFile matches the writable path buffer.
        let accepted = unsafe {
            if save {
                GetSaveFileNameW(&mut dialog)
            } else {
                GetOpenFileNameW(&mut dialog)
            }
        };
        if accepted == 0 {
            // SAFETY: this takes no pointers and reads the calling thread's error
            // state immediately after the failed common-dialog call.
            let code = unsafe { CommDlgExtendedError() };
            return if code == 0 {
                Ok(None)
            } else {
                Err(io::Error::other(format!(
                    "Windows file dialog failed with code 0x{code:08X}"
                )))
            };
        }

        let length = path
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(path.len());
        Ok(Some((
            PathBuf::from(OsString::from_wide(&path[..length])),
            dialog.nFilterIndex,
        )))
    }
}

pub fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn decode_image(path: &Path) -> io::Result<DecodedImage> {
    let mut token = 0;
    let input = GDIPLUS_STARTUP_INPUT {
        GdiplusVersion: 1,
        DebugEventCallback: ptr::null_mut(),
        SuppressBackgroundThread: 0,
        SuppressExternalCodecs: 0,
    };
    gdip_status(
        unsafe { GdiplusStartup(&mut token, &input, ptr::null_mut()) },
        "start image decoder",
    )?;
    let result = decode_started(path);
    unsafe { GdiplusShutdown(token) };
    result
}

fn decode_started(path: &Path) -> io::Result<DecodedImage> {
    let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut bitmap = ptr::null_mut();
    gdip_status(
        unsafe { GdipCreateBitmapFromFile(path.as_ptr(), &mut bitmap) },
        "open image",
    )?;
    if bitmap.is_null() {
        return Err(io::Error::other("image decoder returned no bitmap"));
    }
    let result = decode_bitmap(bitmap);
    unsafe { GdipDisposeImage(bitmap) };
    result
}

fn decode_bitmap(bitmap: *mut c_void) -> io::Result<DecodedImage> {
    let mut width = 0;
    let mut height = 0;
    gdip_status(
        unsafe { GdipGetImageWidth(bitmap, &mut width) },
        "read image width",
    )?;
    gdip_status(
        unsafe { GdipGetImageHeight(bitmap, &mut height) },
        "read image height",
    )?;
    let pixel_count = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 || pixel_count > MAX_PIXELS {
        return Err(io::Error::other("image dimensions are not supported"));
    }
    let pixel_count = usize::try_from(pixel_count)
        .map_err(|_| io::Error::other("image dimensions are not supported"))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(pixel_count)
        .map_err(|_| io::Error::other("not enough memory to import image"))?;
    pixels.resize(pixel_count, 0);

    let rect = GDIP_RECT {
        X: 0,
        Y: 0,
        Width: width as i32,
        Height: height as i32,
    };
    let mut data = BITMAP_DATA {
        Width: width,
        Height: height,
        Stride: (width * 4) as i32,
        PixelFormat: PIXEL_FORMAT_32BPP_ARGB,
        Scan0: pixels.as_mut_ptr().cast(),
        Reserved: 0,
    };
    gdip_status(
        unsafe {
            GdipBitmapLockBits(
                bitmap,
                &rect,
                IMAGE_LOCK_MODE_READ | IMAGE_LOCK_MODE_USER_INPUT_BUFFER,
                PIXEL_FORMAT_32BPP_ARGB,
                &mut data,
            )
        },
        "decode image pixels",
    )?;
    gdip_status(
        unsafe { GdipBitmapUnlockBits(bitmap, &mut data) },
        "unlock image pixels",
    )?;
    Ok(DecodedImage {
        width,
        height,
        pixels,
    })
}

fn gdip_status(status: i32, operation: &str) -> io::Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "could not {operation} (GDI+ status {status})"
        )))
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
                match state.framebuffer.resize(width, height) {
                    Ok(()) => state.events.push_back(Event::Resized { width, height }),
                    Err(error) => state.error = Some(io::Error::other(error)),
                }
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
        WM_POINTERENTER
        | WM_POINTERDOWN
        | WM_POINTERUPDATE
        | WM_POINTERUP
        | WM_POINTERLEAVE
        | WM_POINTERCAPTURECHANGED
            if !state.is_null() =>
        {
            if push_pen_events(unsafe { &mut *state }, window, message, wparam) {
                // Both the UI and tools consume PenSample directly. Returning
                // zero prevents DefWindowProc from generating duplicate mouse
                // clicks/strokes; mouse and touch keep their default handling.
                // https://learn.microsoft.com/en-us/windows/win32/inputmsg/wm-pointerdown
                0
            } else {
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_KILLFOCUS if !state.is_null() => {
            unsafe { &mut *state }.events.push_back(Event::FocusLost);
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
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
            if dc.is_null() {
                unsafe { &mut *state }.error = Some(io::Error::last_os_error());
            } else if let Err(error) = unsafe { present_framebuffer(dc, &(*state).framebuffer) } {
                unsafe { &mut *state }.error = Some(error);
            }
            if unsafe { EndPaint(window, &paint) } == 0 {
                unsafe { &mut *state }.error = Some(io::Error::last_os_error());
            }
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

/// Extracts the pointer id from a WM_POINTER* message's wParam
/// (Win32 GET_POINTERID_WPARAM: the low 16 bits).
fn pointer_id(wparam: WPARAM) -> u32 {
    (wparam & 0xffff) as u32
}

/// Translates WM_POINTER* messages into common pen events. Non-pen pointers
/// (mouse/touch) produce nothing and keep their default behavior.
fn push_pen_events(state: &mut WindowState, window: HWND, message: UINT, wparam: WPARAM) -> bool {
    let pointer_id = pointer_id(wparam);
    let terminal = matches!(message, WM_POINTERLEAVE | WM_POINTERCAPTURECHANGED)
        || (wparam >> 16) as u32 & POINTER_FLAG_CANCELED != 0;
    if state.legacy_pointers.contains(&pointer_id) {
        if terminal {
            state.legacy_pointers.retain(|&id| id != pointer_id);
        }
        return false;
    }
    // SAFETY: pointer_id comes from the message's wParam; every out-pointer is a
    // valid stack local that outlives each synchronous call, and the calls only
    // fill POD data, retaining no references afterwards.
    unsafe {
        let mut pointer_type = 0;
        if !state.pen_pointers.contains(&pointer_id) {
            if GetPointerType(pointer_id, &mut pointer_type) == 0 || pointer_type != PT_PEN {
                if !terminal {
                    state.legacy_pointers.push(pointer_id);
                }
                return false;
            }
            state.pen_pointers.push(pointer_id);
        }
        if message == WM_POINTERCAPTURECHANGED || (wparam >> 16) as u32 & POINTER_FLAG_CANCELED != 0
        {
            state.events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            state.pen_pointers.retain(|&id| id != pointer_id);
            return true;
        }
        if message == WM_POINTERLEAVE {
            state.events.push_back(Event::PenProximityOut {
                pointer_id: u64::from(pointer_id),
            });
            state.pen_pointers.retain(|&id| id != pointer_id);
            return true;
        }

        let Some(origin) = client_origin(window) else {
            state.events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            return true;
        };

        if message == WM_POINTERUPDATE {
            // Pen digitizers sample faster than the message queue delivers, so
            // Windows coalesces samples into one WM_POINTERUPDATE. The history
            // call returns every coalesced sample, newest first.
            let mut capacity = state.pen_history.capacity().max(16);
            loop {
                state.pen_history.clear();
                state
                    .pen_history
                    .resize_with(capacity, POINTER_PEN_INFO::default);
                let mut count = capacity as u32;
                // SAFETY: pen_history owns `capacity` contiguous zeroed
                // POINTER_PEN_INFO entries and count is a stack local; both
                // outlive the synchronous call, which only fills POD data.
                let retrieved = GetPointerPenInfoHistory(
                    pointer_id,
                    &mut count,
                    state.pen_history.as_mut_ptr(),
                ) != 0;
                if count as usize > capacity {
                    // Buffer too small: Windows reported the total available.
                    capacity = count as usize;
                    continue;
                }
                if retrieved && count > 0 {
                    push_coalesced_moves(
                        &mut state.events,
                        pointer_id,
                        &state.pen_history[..count as usize],
                        |point| pen_client_location(window, point, origin),
                    );
                    return true;
                }
                // Some drivers cannot supply history for every message. Keep
                // the current sample via GetPointerPenInfo below in that case.
                break;
            }
        }

        let mut pen_info = POINTER_PEN_INFO::default();
        if GetPointerPenInfo(pointer_id, &mut pen_info) == 0 {
            state.events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            return true;
        }
        if pen_info.pointer_info.pointer_flags & POINTER_FLAG_CANCELED != 0 {
            state.events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            return true;
        }
        let Some(location) =
            pen_client_location(window, pen_info.pointer_info.pt_pixel_location, origin)
        else {
            state.events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            return true;
        };
        let sample = pen_sample_from(pointer_id, &pen_info, location);
        state.events.push_back(match message {
            WM_POINTERENTER => Event::PenProximityIn(sample),
            WM_POINTERDOWN => Event::PenDown(sample),
            WM_POINTERUPDATE => Event::PenMove(sample),
            _ => Event::PenUp(sample),
        });
    }
    true
}

/// Logical screen origin of the client area, constant for one message batch.
fn client_origin(window: HWND) -> Option<POINT> {
    let mut origin = POINT { x: 0, y: 0 };
    // SAFETY: origin is a valid stack local alive for the synchronous call.
    (unsafe { ClientToScreen(window, &mut origin) } != 0).then_some(origin)
}

fn client_location(location: POINT, origin: POINT) -> POINT {
    POINT {
        x: location.x - origin.x,
        y: location.y - origin.y,
    }
}

fn pen_client_location(window: HWND, mut physical: POINT, origin: POINT) -> Option<POINT> {
    // POINTER_INFO positions use physical pixels. The framebuffer and mouse
    // use this window's logical coordinates, which differ with DPI scaling.
    // Convert before subtracting the client origin, including for history.
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-physicaltologicalpointforpermonitordpi
    // SAFETY: physical is a live POINT; the synchronous API retains no pointer.
    (unsafe { PhysicalToLogicalPointForPerMonitorDPI(window, &mut physical) } != 0)
        .then(|| client_location(physical, origin))
}

/// Emits one PenMove per coalesced sample. GetPointerPenInfoHistory returns the
/// newest sample first; strokes must process the oldest first or they run
/// backwards, so the slice is reversed into chronological order.
fn push_coalesced_moves(
    events: &mut VecDeque<Event>,
    pointer_id: u32,
    newest_first: &[POINTER_PEN_INFO],
    mut to_client: impl FnMut(POINT) -> Option<POINT>,
) {
    for pen_info in newest_first.iter().rev() {
        if pen_info.pointer_info.pointer_flags & POINTER_FLAG_CANCELED != 0 {
            events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            break;
        }
        let Some(location) = to_client(pen_info.pointer_info.pt_pixel_location) else {
            events.push_back(Event::PenCancelled {
                pointer_id: u64::from(pointer_id),
            });
            break;
        };
        events.push_back(Event::PenMove(pen_sample_from(
            pointer_id, pen_info, location,
        )));
    }
}

/// Builds the platform-independent `PenSample` from one POINTER_PEN_INFO.
/// Axes whose penMask bit is absent get neutral values, per the pen model.
fn pen_sample_from(pointer_id: u32, pen_info: &POINTER_PEN_INFO, client: POINT) -> PenSample {
    let flags = pen_info.pointer_info.pointer_flags;
    let in_contact = flags & POINTER_FLAG_INCONTACT != 0;
    PenSample {
        pointer_id: u64::from(pointer_id),
        x: client.x as f32,
        y: client.y as f32,
        pressure: if pen_info.pen_mask & PEN_MASK_PRESSURE != 0 {
            normalize(pen_info.pressure, 1024)
        } else if in_contact {
            // Device without pressure support: full pressure while drawing,
            // never 0.0 or the stroke would be invisible.
            1.0
        } else {
            0.0
        },
        tilt_x: if pen_info.pen_mask & PEN_MASK_TILT_X != 0 {
            pen_info.tilt_x as f32
        } else {
            0.0
        },
        tilt_y: if pen_info.pen_mask & PEN_MASK_TILT_Y != 0 {
            pen_info.tilt_y as f32
        } else {
            0.0
        },
        rotation: if pen_info.pen_mask & PEN_MASK_ROTATION != 0 {
            pen_info.rotation as f32
        } else {
            0.0
        },
        distance: 0.0,
        in_contact,
        in_proximity: flags & POINTER_FLAG_INRANGE != 0,
        barrel_button_1: pen_info.pen_flags & PEN_FLAG_BARREL != 0,
        // Pointer Input exposes one barrel button; do not invent a second one
        // from the primary/secondary action flags in POINTER_INFO.
        barrel_button_2: false,
        tool: if pen_info.pen_flags & (PEN_FLAG_INVERTED | PEN_FLAG_ERASER) != 0 {
            PenTool::Eraser
        } else {
            PenTool::Pen
        },
        timestamp: pen_info.pointer_info.performance_count,
    }
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

unsafe fn present_framebuffer(dc: HDC, framebuffer: &FrameBuffer) -> io::Result<()> {
    if framebuffer.width == 0 || framebuffer.height == 0 {
        return Ok(());
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
    let lines = unsafe {
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
        )
    };
    if lines == 0 || lines == GDI_ERROR {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
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
        assert_eq!(pointer_id(0x1234_0007), 7);

        let mut state = WindowState::default();
        state.push_text(0xd83d);
        state.push_text(0xde00);
        assert_eq!(
            state.events.pop_front(),
            Some(Event::TextInput { character: '😀' })
        );
    }

    #[test]
    fn translates_pen_pointer_input_into_pen_samples() {
        let mut info = POINTER_PEN_INFO::default();
        info.pointer_info.pointer_flags =
            POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT | POINTER_FLAG_FIRSTBUTTON;
        info.pointer_info.performance_count = 42;
        info.pen_mask = PEN_MASK_PRESSURE | PEN_MASK_ROTATION | PEN_MASK_TILT_X | PEN_MASK_TILT_Y;
        info.pressure = 512;
        info.rotation = 90;
        info.tilt_x = -23;
        info.tilt_y = 11;

        let sample = pen_sample_from(7, &info, POINT { x: 734, y: 421 });

        assert_eq!(sample.pointer_id, 7);
        assert_eq!((sample.x, sample.y), (734.0, 421.0));
        assert_eq!(sample.pressure, 0.5);
        assert_eq!(sample.rotation, 90.0);
        assert_eq!(sample.tilt_x, -23.0);
        assert_eq!(sample.tilt_y, 11.0);
        assert_eq!(sample.timestamp, 42);
        assert!(sample.in_contact);
        assert!(sample.in_proximity);
        assert!(!sample.barrel_button_1);
        assert!(!sample.barrel_button_2);
        assert_eq!(sample.tool, PenTool::Pen);
    }

    #[test]
    fn pen_samples_use_neutral_values_for_unsupported_axes() {
        let hovering = pen_sample_from(1, &POINTER_PEN_INFO::default(), POINT::default());
        assert_eq!(hovering.pressure, 0.0);
        assert_eq!(hovering.tilt_x, 0.0);
        assert_eq!(hovering.tilt_y, 0.0);
        assert_eq!(hovering.rotation, 0.0);
        assert_eq!(hovering.tool, PenTool::Pen);

        let mut info = POINTER_PEN_INFO::default();
        info.pointer_info.pointer_flags = POINTER_FLAG_INCONTACT;
        info.pen_flags = PEN_FLAG_INVERTED;
        let drawing = pen_sample_from(1, &info, POINT::default());
        assert_eq!(drawing.pressure, 1.0);
        assert_eq!(drawing.tool, PenTool::Eraser);
    }

    #[test]
    fn windows_sdk_pen_packets_produce_visible_pressure_strokes() {
        use crate::{
            document::Document,
            graphics::Color,
            tools::{BrushSample, BrushTool, Tool},
        };

        // These bytes come from the C Windows SDK types, never from our Rust
        // declarations. See tests/fixtures/windows_pen_fixture.c to regenerate.
        #[cfg(target_pointer_width = "64")]
        let native_bytes = include_bytes!("../../../tests/fixtures/windows-pen-x64.bin");
        #[cfg(target_pointer_width = "32")]
        let native_bytes = include_bytes!("../../../tests/fixtures/windows-pen-x86.bin");
        let native_stride = native_bytes.len() / 2;
        assert_eq!(
            size_of::<POINTER_PEN_INFO>(),
            native_stride,
            "the Win32 API would overwrite an undersized pen buffer"
        );

        let newest_first: Vec<POINTER_PEN_INFO> = native_bytes
            .chunks_exact(native_stride)
            .map(|bytes| {
                // SAFETY: The size assertion above verifies a complete native
                // record. All fields are integers or raw handles (never
                // dereferenced here); read_unaligned handles the byte buffer.
                unsafe { ptr::read_unaligned(bytes.as_ptr().cast::<POINTER_PEN_INFO>()) }
            })
            .collect();
        assert_eq!(newest_first[0].pointer_info.h_target as usize, 0x40506);
        assert_eq!(newest_first[0].pointer_info.history_count, 2);
        assert_eq!(newest_first[0].pointer_info.input_data, -9);
        let mut events = VecDeque::new();
        push_coalesced_moves(&mut events, 7, &newest_first, |point| {
            Some(client_location(point, POINT { x: 200, y: 100 }))
        });
        let mut samples = Vec::new();
        for event in events {
            let Event::PenMove(sample) = event else {
                panic!("expected a pen move")
            };
            assert!(sample.in_contact && sample.in_proximity);
            assert!(!sample.barrel_button_1 && !sample.barrel_button_2);
            assert_eq!(sample.tool, PenTool::Pen);
            assert_eq!(
                (sample.tilt_x, sample.tilt_y, sample.rotation),
                (-23.0, 11.0, 90.0)
            );
            samples.push(sample);
        }
        assert_eq!(
            (
                samples[0].x,
                samples[0].y,
                samples[0].pressure,
                samples[0].timestamp
            ),
            (10.0, 20.0, 0.25, 1000)
        );
        assert_eq!(
            (
                samples[1].x,
                samples[1].y,
                samples[1].pressure,
                samples[1].timestamp
            ),
            (50.0, 30.0, 1.0, 1001)
        );

        let to_brush = |sample: PenSample| BrushSample {
            x: sample.x,
            y: sample.y,
            pressure: sample.pressure,
            tilt_x: sample.tilt_x,
            tilt_y: sample.tilt_y,
            rotation: sample.rotation,
            timestamp: sample.timestamp,
        };
        let mut document = Document::new(80, 60, Color::rgba(0, 0, 0, 0)).unwrap();
        let pixels = &mut document.active_layer_mut().pixels;
        let mut brush = BrushTool::default();
        brush.pointer_down(pixels, to_brush(samples[0]));
        assert_eq!(pixels.get_pixel(10, 20).unwrap().alpha(), 64);
        brush.pointer_move(pixels, to_brush(samples[1]));
        brush.pointer_up(pixels);
        assert!(pixels.get_pixel(30, 25).unwrap().alpha() > 0);
        assert_eq!(pixels.get_pixel(50, 30).unwrap().alpha(), 255);
    }

    #[test]
    fn native_barrel_and_eraser_bits_are_independent_of_tip_contact() {
        let mut info = POINTER_PEN_INFO::default();
        info.pointer_info.pointer_flags = 0x16; // In range, contact, primary action.
        let tip = pen_sample_from(7, &info, POINT::default());
        assert!(!tip.barrel_button_1);
        assert_eq!(tip.tool, PenTool::Pen);
        info.pen_flags = 0x01; // PEN_FLAG_BARREL per the Windows SDK.
        let barrel = pen_sample_from(7, &info, POINT::default());
        assert!(barrel.barrel_button_1);
        assert!(!barrel.barrel_button_2);
        assert_eq!(barrel.tool, PenTool::Pen);
        for flags in [0x02, 0x04] {
            // Inverted pen or eraser button.
            info.pen_flags = flags;
            let eraser = pen_sample_from(7, &info, POINT::default());
            assert_eq!(eraser.tool, PenTool::Eraser);
            assert!(!eraser.barrel_button_1);
        }
    }

    #[test]
    fn capture_loss_of_a_known_pen_is_consumed_without_a_successful_native_query() {
        let mut state = WindowState::default();
        state.pen_pointers.push(7);
        // A null HWND cannot provide pen data. Capture loss must still cancel
        // our known contact and must not be promoted to a mouse sequence.
        assert!(push_pen_events(
            &mut state,
            ptr::null_mut(),
            WM_POINTERCAPTURECHANGED,
            7
        ));
        assert_eq!(
            state.events.pop_front(),
            Some(Event::PenCancelled { pointer_id: 7 })
        );
        assert!(state.pen_pointers.is_empty());
        assert!(!push_pen_events(
            &mut state,
            ptr::null_mut(),
            WM_POINTERUPDATE,
            0
        ));
        // Failed initial classification stays on the legacy route through
        // release, so the emulated MouseDown cannot lose its MouseUp.
        assert_eq!(state.legacy_pointers, [0]);
        assert!(!push_pen_events(
            &mut state,
            ptr::null_mut(),
            WM_POINTERUP,
            0
        ));
        assert!(!push_pen_events(
            &mut state,
            ptr::null_mut(),
            WM_POINTERLEAVE,
            0
        ));
        assert!(state.legacy_pointers.is_empty());
    }

    #[test]
    fn cancellation_in_coalesced_history_stops_the_remaining_samples() {
        let mut canceled = POINTER_PEN_INFO::default();
        canceled.pointer_info.pointer_flags = POINTER_FLAG_CANCELED;
        let mut events = VecDeque::new();
        push_coalesced_moves(
            &mut events,
            7,
            &[POINTER_PEN_INFO::default(), canceled],
            Some,
        );
        assert_eq!(
            events.into_iter().collect::<Vec<_>>(),
            [Event::PenCancelled { pointer_id: 7 }]
        );
    }

    #[test]
    fn every_history_sample_uses_the_coordinate_transform_and_failure_cancels() {
        let mut info = POINTER_PEN_INFO::default();
        info.pointer_info.pt_pixel_location = POINT { x: 480, y: 360 };
        let mut events = VecDeque::new();
        // A 200% scale and a logical client origin of (100, 50).
        push_coalesced_moves(&mut events, 7, std::slice::from_ref(&info), |physical| {
            Some(client_location(
                POINT {
                    x: physical.x / 2,
                    y: physical.y / 2,
                },
                POINT { x: 100, y: 50 },
            ))
        });
        let Some(Event::PenMove(sample)) = events.pop_front() else {
            panic!("missing pen move")
        };
        assert_eq!((sample.x, sample.y), (140.0, 130.0));
        push_coalesced_moves(&mut events, 7, std::slice::from_ref(&info), |_| None);
        assert_eq!(
            events.pop_front(),
            Some(Event::PenCancelled { pointer_id: 7 })
        );
        assert!(events.is_empty());
    }

    #[test]
    fn coalesced_pen_history_arrives_in_chronological_order() {
        fn entry(timestamp: u64, screen_x: i32) -> POINTER_PEN_INFO {
            let mut info = POINTER_PEN_INFO::default();
            info.pointer_info.pt_pixel_location = POINT {
                x: screen_x,
                y: 450,
            };
            info.pointer_info.pointer_flags = POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT;
            info.pointer_info.performance_count = timestamp;
            info.pen_mask = PEN_MASK_PRESSURE;
            info.pressure = 512;
            info
        }

        // GetPointerPenInfoHistory returns the newest sample first.
        let newest_first = [entry(10, 300), entry(9, 200), entry(8, 100)];

        let mut state = WindowState::default();
        push_coalesced_moves(&mut state.events, 7, &newest_first, |point| {
            Some(client_location(point, POINT { x: 100, y: 50 }))
        });

        let mut coordinates = Vec::new();
        while let Some(event) = state.events.pop_front() {
            let Event::PenMove(sample) = event else {
                panic!("expected only PenMove events, got {event:?}");
            };
            coordinates.push((sample.timestamp, sample.x, sample.y));
        }
        assert_eq!(
            coordinates,
            [(8, 0.0, 400.0), (9, 100.0, 400.0), (10, 200.0, 400.0)]
        );
    }

    #[test]
    fn decodes_exported_png_pixels_with_windows() {
        let path = std::env::temp_dir().join(format!(
            "opendraw-{}-native-png-decode.png",
            std::process::id()
        ));
        let color = crate::graphics::Color::rgba(10, 20, 30, 40);
        let mut document =
            crate::document::Document::new(2, 2, crate::graphics::Color::rgba(0, 0, 0, 0)).unwrap();
        document
            .active_layer_mut()
            .pixels
            .stamp_circle(0.5, 0.5, 0.5, color);
        crate::file::export(&document, &path, crate::file::ImageFormat::Png).unwrap();

        let image = decode_image(&path).unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.pixels[0], color.as_u32());
        assert_eq!(image.pixels[3], 0);
        std::fs::remove_file(path).unwrap();
    }
}
