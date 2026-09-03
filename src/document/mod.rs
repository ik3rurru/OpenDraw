mod canvas_view;

use std::fmt;

use crate::graphics::Color;

pub use canvas_view::CanvasView;

pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;

pub struct Document {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
}

pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: u8,
    pub pixels: PixelBuffer,
}

pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentError {
    InvalidDimensions,
    TooLarge,
    AllocationFailed,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions => formatter.write_str("document dimensions must be positive"),
            Self::TooLarge => formatter.write_str("document exceeds the pixel limit"),
            Self::AllocationFailed => formatter.write_str("could not allocate document pixels"),
        }
    }
}

impl Document {
    pub fn new(width: u32, height: u32, background: Color) -> Result<Self, DocumentError> {
        Ok(Self {
            width,
            height,
            layers: vec![Layer {
                name: String::from("LAYER 1"),
                visible: true,
                opacity: 255,
                pixels: PixelBuffer::new(width, height, background)?,
            }],
            active_layer: 0,
        })
    }

    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active_layer]
    }
}

impl PixelBuffer {
    fn new(width: u32, height: u32, fill: Color) -> Result<Self, DocumentError> {
        if width == 0 || height == 0 {
            return Err(DocumentError::InvalidDimensions);
        }

        let pixel_count = u64::from(width) * u64::from(height);
        if pixel_count > MAX_PIXELS {
            return Err(DocumentError::TooLarge);
        }
        let pixel_count = usize::try_from(pixel_count).map_err(|_| DocumentError::TooLarge)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(pixel_count)
            .map_err(|_| DocumentError::AllocationFailed)?;
        pixels.resize(pixel_count, fill.as_u32());

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(Color::from_u32(self.pixels[(y * self.width + x) as usize]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_one_initialized_layer_with_safe_limits() {
        let white = Color::rgb(255, 255, 255);
        let document = Document::new(4, 3, white).unwrap();
        assert_eq!(document.layers.len(), 1);
        assert_eq!(document.active_layer().pixels.get_pixel(3, 2), Some(white));
        assert_eq!(
            Document::new(0, 3, white).err(),
            Some(DocumentError::InvalidDimensions)
        );
        assert_eq!(
            Document::new(16_384, 16_384, white).err(),
            Some(DocumentError::TooLarge)
        );
    }
}
