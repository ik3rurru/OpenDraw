mod event;

#[cfg(target_os = "windows")]
mod windows;

pub use event::{Event, Key, MouseButton};

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

#[cfg(target_os = "windows")]
pub use windows::{Window, decode_image};
