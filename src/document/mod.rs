mod canvas_view;

use std::fmt;

use crate::graphics::Color;

pub use canvas_view::CanvasView;

pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_LAYERS: usize = 256;

pub struct Document {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    next_layer_number: u32,
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
            next_layer_number: 2,
        })
    }

    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active_layer]
    }

    pub fn active_layer_mut(&mut self) -> &mut Layer {
        &mut self.layers[self.active_layer]
    }

    pub fn composite_pixel(&self, x: u32, y: u32, background: Color) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(
            self.layers
                .iter()
                .filter(|layer| layer.visible)
                .fold(background, |result, layer| {
                    let pixel = layer.pixels.get_pixel(x, y).unwrap();
                    Color::rgba(
                        pixel.red(),
                        pixel.green(),
                        pixel.blue(),
                        ((u16::from(pixel.alpha()) * u16::from(layer.opacity) + 127) / 255) as u8,
                    )
                    .blend_over(result)
                }),
        )
    }

    pub fn add_layer(&mut self) -> Result<(), DocumentError> {
        let pixels_per_layer = u64::from(self.width) * u64::from(self.height);
        if self.layers.len() >= MAX_LAYERS
            || pixels_per_layer * (self.layers.len() as u64 + 1) > MAX_PIXELS
        {
            return Err(DocumentError::TooLarge);
        }
        self.layers.push(Layer {
            name: format!("LAYER {}", self.next_layer_number),
            visible: true,
            opacity: 255,
            pixels: PixelBuffer::new(self.width, self.height, Color::rgba(0, 0, 0, 0))?,
        });
        self.next_layer_number += 1;
        self.active_layer = self.layers.len() - 1;
        Ok(())
    }

    pub fn remove_active_layer(&mut self) -> bool {
        if self.layers.len() == 1 {
            return false;
        }
        self.layers.remove(self.active_layer);
        self.active_layer = self.active_layer.min(self.layers.len() - 1);
        true
    }

    pub fn move_active_down(&mut self) {
        if self.active_layer > 0 {
            self.layers.swap(self.active_layer, self.active_layer - 1);
            self.active_layer -= 1;
        }
    }

    pub fn move_active_up(&mut self) {
        if self.active_layer + 1 < self.layers.len() {
            self.layers.swap(self.active_layer, self.active_layer + 1);
            self.active_layer += 1;
        }
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

    pub(crate) fn stamp_circle(&mut self, center_x: u32, center_y: u32, radius: u32, color: Color) {
        let radius = i64::from(radius);
        let radius_squared = radius * radius;
        for offset_y in -radius..=radius {
            for offset_x in -radius..=radius {
                if offset_x * offset_x + offset_y * offset_y > radius_squared {
                    continue;
                }
                let x = i64::from(center_x) + offset_x;
                let y = i64::from(center_y) + offset_y;
                if x < 0 || y < 0 || x >= i64::from(self.width) || y >= i64::from(self.height) {
                    continue;
                }
                let index = (y as u32 * self.width + x as u32) as usize;
                let background = Color::from_u32(self.pixels[index]);
                self.pixels[index] = color.blend_over(background).as_u32();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_one_initialized_layer_with_safe_limits() {
        let white = Color::rgb(255, 255, 255);
        let mut document = Document::new(4, 3, white).unwrap();
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

        let black = Color::rgb(0, 0, 0);
        document.add_layer().unwrap();
        assert_eq!(document.active_layer().name, "LAYER 2");
        document
            .active_layer_mut()
            .pixels
            .stamp_circle(3, 0, 0, black);
        assert_eq!(
            document.composite_pixel(3, 0, Color::rgba(0, 0, 0, 0)),
            Some(black)
        );
        document.active_layer_mut().opacity = 128;
        assert_eq!(
            document.composite_pixel(3, 0, Color::rgba(0, 0, 0, 0)),
            Some(Color::rgb(127, 127, 127))
        );
        document.active_layer_mut().visible = false;
        assert_eq!(
            document.composite_pixel(3, 0, Color::rgba(0, 0, 0, 0)),
            Some(white)
        );
        document.active_layer_mut().visible = true;
        document.move_active_down();
        assert_eq!(
            document.composite_pixel(3, 0, Color::rgba(0, 0, 0, 0)),
            Some(white)
        );
        document.move_active_up();
        assert!(document.remove_active_layer());
        assert!(!document.remove_active_layer());
    }
}
