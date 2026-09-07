mod event;
mod input;

#[cfg(target_os = "windows")]
mod windows;

pub use event::{Event, Key, MouseButton};
// Consumed from PEN-002 onward by the platform tablet backends.
#[allow(unused_imports)]
pub use input::{PenCapabilities, PenSample, PenTool, PointerType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveChanges {
    Save,
    Discard,
    Cancel,
}

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

#[cfg(target_os = "windows")]
pub use windows::{Window, decode_image, replace_file};

#[cfg(not(target_os = "windows"))]
pub fn replace_file(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}
