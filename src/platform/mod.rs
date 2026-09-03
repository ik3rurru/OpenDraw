mod event;

#[cfg(target_os = "windows")]
mod windows;

pub use event::{Event, Key, MouseButton};

#[cfg(target_os = "windows")]
pub use windows::Window;
