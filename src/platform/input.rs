//! Common pointer and pen input types shared by every platform backend.
//!
//! Native tablet APIs (Windows Pointer Input, Wayland tablet-v2, XInput2) are
//! translated by `platform` code into these types. The rest of OpenDraw never
//! sees the operating system's representation of a stylus.

// ponytail: PEN-001 defines the pen model before its consumers exist; the
// platform backends from PEN-002 construct these. Remove once they do.
#![allow(dead_code)]

/// Physical kind of a pointer device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerType {
    Mouse,
    Pen,
    Eraser,
    Touch,
}

/// Physical tool reported by the stylus hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PenTool {
    Pen,
    Eraser,
    Brush,
    Pencil,
    Airbrush,
    Unknown,
}

/// Data axes a tablet can actually report. Never assume an axis exists: a
/// missing axis must be replaced with a neutral value by the backend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PenCapabilities {
    pub pressure: bool,
    pub tilt: bool,
    pub rotation: bool,
    pub distance: bool,
    pub eraser: bool,
    pub barrel_buttons: u8,
}

/// One stylus sample in window coordinates, already normalized:
///
/// - `pressure`: 0.0 ..= 1.0 (1.0 = maximum),
/// - `tilt_x`/`tilt_y`: degrees, -90.0 ..= 90.0,
/// - `rotation`: degrees, 0.0 ..= 360.0,
/// - `distance`: 0.0 ..= 1.0.
///
/// A device without pressure must report 1.0 while the pen is in contact,
/// not 0.0, or its strokes would be invisible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenSample {
    pub pointer_id: u64,
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
    pub distance: f32,
    pub in_contact: bool,
    pub in_proximity: bool,
    pub barrel_button_1: bool,
    pub barrel_button_2: bool,
    pub tool: PenTool,
    pub timestamp: u64,
}

/// Converts a raw axis value over `0..=max` into the normalized `0.0..=1.0`
/// range used by `PenSample`. Values outside the raw range are clamped.
pub fn normalize(value: u32, max: u32) -> f32 {
    if max == 0 {
        return 0.0;
    }
    (value as f32 / max as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_tablet_axis_ranges() {
        assert_eq!(normalize(0, 1024), 0.0);
        assert_eq!(normalize(512, 1024), 0.5);
        assert_eq!(normalize(1024, 1024), 1.0);
        assert_eq!(normalize(0, 65535), 0.0);
        assert_eq!(normalize(65535, 65535), 1.0);
    }

    #[test]
    fn normalizations_clamp_out_of_range_values() {
        assert_eq!(normalize(2000, 1024), 1.0);
        assert_eq!(normalize(0, 0), 0.0);
    }
}
