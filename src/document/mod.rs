#[cfg(test)]
mod antialiasing_tests;
mod canvas_view;

use std::fmt;

use crate::graphics::Color;

pub use canvas_view::CanvasView;

pub const MAX_PIXELS: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_LAYERS: usize = 256;

#[derive(Clone)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    next_layer_number: u32,
}

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: u8,
    pub pixels: PixelBuffer,
}

#[derive(Clone)]
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
    /// A user-created document starts with a separate background and an empty
    /// active drawing layer. Validate both buffers before allocating either.
    pub fn with_background(
        width: u32,
        height: u32,
        background: Color,
    ) -> Result<Self, DocumentError> {
        if u64::from(width) * u64::from(height) > MAX_PIXELS / 2 {
            return Err(DocumentError::TooLarge);
        }
        let mut document = Self::new(width, height, background)?;
        document.layers[0].name = String::from("BACKGROUND");
        document.add_layer()?;
        Ok(document)
    }

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

    pub(crate) fn from_layers(
        width: u32,
        height: u32,
        layers: Vec<Layer>,
        active_layer: usize,
    ) -> Result<Self, DocumentError> {
        let pixels_per_layer = u64::from(width) * u64::from(height);
        let expected = usize::try_from(pixels_per_layer).map_err(|_| DocumentError::TooLarge)?;
        if width == 0
            || height == 0
            || layers.is_empty()
            || layers.len() > MAX_LAYERS
            || active_layer >= layers.len()
            || pixels_per_layer
                .checked_mul(layers.len() as u64)
                .is_none_or(|total| total > MAX_PIXELS)
            || layers.iter().any(|layer| {
                layer.pixels.width != width
                    || layer.pixels.height != height
                    || layer.pixels.pixels.len() != expected
            })
        {
            return Err(DocumentError::InvalidDimensions);
        }
        let next_layer_number = layers
            .iter()
            .filter_map(|layer| layer.name.strip_prefix("LAYER ")?.parse::<u32>().ok())
            .max()
            .unwrap_or(layers.len() as u32)
            .saturating_add(1);
        Ok(Self {
            width,
            height,
            layers,
            active_layer,
            next_layer_number,
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

    pub fn import_layer(
        &mut self,
        name: String,
        width: u32,
        height: u32,
        pixels: Vec<u32>,
    ) -> Result<(), DocumentError> {
        let imported_pixels = u64::from(width) * u64::from(height);
        if width == 0 || height == 0 || usize::try_from(imported_pixels).ok() != Some(pixels.len())
        {
            return Err(DocumentError::InvalidDimensions);
        }
        let new_width = self.width.max(width);
        let new_height = self.height.max(height);
        let pixels_per_layer = u64::from(new_width) * u64::from(new_height);
        if self.layers.len() >= MAX_LAYERS
            || pixels_per_layer
                .checked_mul(self.layers.len() as u64 + 1)
                .is_none_or(|total| total > MAX_PIXELS)
        {
            return Err(DocumentError::TooLarge);
        }

        let imported = PixelBuffer {
            width,
            height,
            pixels,
        };
        let mut layers = Vec::new();
        layers
            .try_reserve_exact(self.layers.len() + 1)
            .map_err(|_| DocumentError::AllocationFailed)?;
        for layer in &self.layers {
            layers.push(Layer {
                name: layer.name.clone(),
                visible: layer.visible,
                opacity: layer.opacity,
                pixels: layer.pixels.centered_in(new_width, new_height)?,
            });
        }
        layers.push(Layer {
            name,
            visible: true,
            opacity: 255,
            pixels: imported.centered_in(new_width, new_height)?,
        });
        self.width = new_width;
        self.height = new_height;
        self.active_layer = layers.len() - 1;
        self.layers = layers;
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

    fn centered_in(&self, width: u32, height: u32) -> Result<Self, DocumentError> {
        let mut result = Self::new(width, height, Color::rgba(0, 0, 0, 0))?;
        let offset_x = (width - self.width) / 2;
        let offset_y = (height - self.height) / 2;
        for y in 0..self.height {
            let source = (y * self.width) as usize;
            let destination = ((y + offset_y) * width + offset_x) as usize;
            result.pixels[destination..destination + self.width as usize]
                .copy_from_slice(&self.pixels[source..source + self.width as usize]);
        }
        Ok(result)
    }

    pub(crate) fn flood_fill(&mut self, x: u32, y: u32, color: Color) {
        let Some(source) = self.get_pixel(x, y).map(Color::as_u32) else {
            return;
        };
        let replacement = color.as_u32();
        if source == replacement {
            return;
        }

        let width = self.width as usize;
        let start = (y * self.width + x) as usize;
        self.pixels[start] = replacement;
        let mut pending = vec![start];

        // ponytail: exact 4-neighbor fill; add scanline/tolerance only after profiling large fills.
        while let Some(index) = pending.pop() {
            let pixel_x = index % width;
            let neighbors = [
                (pixel_x > 0).then(|| index - 1),
                (pixel_x + 1 < width).then(|| index + 1),
                (index >= width).then(|| index - width),
                (index + width < self.pixels.len()).then(|| index + width),
            ];
            for neighbor in neighbors.into_iter().flatten() {
                if self.pixels[neighbor] == source {
                    self.pixels[neighbor] = replacement;
                    pending.push(neighbor);
                }
            }
        }
    }

    pub(crate) fn stamp_circle(&mut self, center_x: f32, center_y: f32, radius: f32, color: Color) {
        if color.alpha() == 0 {
            return;
        }
        self.edit_circle(center_x, center_y, radius, |background, coverage| {
            if coverage == 1.0 {
                return color.blend_over(background);
            }
            Color::rgba(
                color.red(),
                color.green(),
                color.blue(),
                (f32::from(color.alpha()) * coverage).round() as u8,
            )
            .blend_over(background)
        });
    }

    pub(crate) fn erase_circle(&mut self, center_x: f32, center_y: f32, radius: f32, opacity: u8) {
        if opacity == 0 {
            return;
        }
        let full_remaining = 255 - u16::from(opacity);
        self.edit_circle(center_x, center_y, radius, |pixel, coverage| {
            let remaining = if coverage == 1.0 {
                full_remaining
            } else {
                255 - (f32::from(opacity) * coverage).round() as u16
            };
            Color::rgba(
                pixel.red(),
                pixel.green(),
                pixel.blue(),
                ((u16::from(pixel.alpha()) * remaining + 127) / 255) as u8,
            )
        });
    }

    fn edit_circle(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        mut edit: impl FnMut(Color, f32) -> Color,
    ) {
        if !center_x.is_finite() || !center_y.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return;
        }
        // Document coordinates describe pixel edges; evaluate the mask at each
        // pixel's center. Radius is geometric (half the effective diameter).
        // A one-pixel distance ramp approximates coverage at the contour; this
        // is not the exact circle/pixel intersection area.
        let outer = radius + 0.5;
        let inner_squared = (radius - 0.5).max(0.0).powi(2);
        let outer_squared = outer * outer;
        let left = (center_x - outer).floor().max(0.0).min(self.width as f32) as u32;
        let top = (center_y - outer).floor().max(0.0).min(self.height as f32) as u32;
        let right = (center_x + outer).ceil().max(0.0).min(self.width as f32) as u32;
        let bottom = (center_y + outer).ceil().max(0.0).min(self.height as f32) as u32;
        for y in top..bottom {
            let dy = y as f32 + 0.5 - center_y;
            let dy_squared = dy * dy;
            let row = y as usize * self.width as usize;
            for x in left..right {
                let dx = x as f32 + 0.5 - center_x;
                let distance_squared = dx * dx + dy_squared;
                if distance_squared >= outer_squared {
                    continue;
                }
                let coverage = if radius >= 0.5 && distance_squared <= inner_squared {
                    1.0
                } else {
                    outer - distance_squared.sqrt()
                };
                let index = row + x as usize;
                self.pixels[index] = edit(Color::from_u32(self.pixels[index]), coverage).as_u32();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_background_documents_have_an_empty_active_layer_and_safe_combined_limits() {
        for background in [
            Color::rgb(255, 255, 255),
            Color::rgb(30, 90, 170),
            Color::rgba(0, 0, 0, 0),
        ] {
            let mut document = Document::with_background(7, 5, background).unwrap();
            assert_eq!(document.layers.len(), 2);
            assert_eq!(document.layers[0].name, "BACKGROUND");
            assert_eq!(
                document.layers[0].pixels.pixels,
                vec![background.as_u32(); 35]
            );
            assert_eq!(document.active_layer, 1);
            assert_eq!(document.active_layer().name, "LAYER 2");
            assert!(
                document
                    .active_layer()
                    .pixels
                    .pixels
                    .iter()
                    .all(|&p| p == 0)
            );
            document.add_layer().unwrap();
            assert_eq!(document.active_layer().name, "LAYER 3");
        }
        assert_eq!(
            Document::with_background(0, 5, Color::rgb(0, 0, 0)).err(),
            Some(DocumentError::InvalidDimensions)
        );
        assert_eq!(
            Document::with_background(8192, 8192, Color::rgb(0, 0, 0)).err(),
            Some(DocumentError::TooLarge)
        );
        assert_eq!(
            Document::with_background(u32::MAX, u32::MAX, Color::rgb(0, 0, 0)).err(),
            Some(DocumentError::TooLarge)
        );
    }

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
            .stamp_circle(3.5, 0.5, 0.5, black);
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

    #[test]
    fn flood_fill_replaces_only_the_bounded_matching_region() {
        let white = Color::rgb(255, 255, 255);
        let black = Color::rgb(0, 0, 0);
        let red = Color::rgba(220, 40, 30, 128);
        let mut document = Document::new(7, 5, white).unwrap();
        let pixels = &mut document.active_layer_mut().pixels;
        for y in 0..pixels.height {
            pixels.stamp_circle(3.5, y as f32 + 0.5, 0.5, black);
        }

        pixels.flood_fill(1, 2, red);

        assert_eq!(pixels.get_pixel(0, 0), Some(red));
        assert_eq!(pixels.get_pixel(2, 4), Some(red));
        assert_eq!(pixels.get_pixel(3, 2), Some(black));
        assert_eq!(pixels.get_pixel(4, 2), Some(white));
        pixels.flood_fill(99, 99, black);
        assert_eq!(pixels.get_pixel(4, 2), Some(white));
    }

    #[test]
    fn imported_layers_expand_and_center_the_canvas_without_cropping() {
        let white = Color::rgb(255, 255, 255);
        let red = Color::rgba(230, 40, 80, 128);
        let mut document = Document::new(2, 4, white).unwrap();

        document
            .import_layer(String::from("LOGO"), 4, 2, vec![red.as_u32(); 8])
            .unwrap();

        assert_eq!((document.width, document.height), (4, 4));
        assert_eq!(document.active_layer, 1);
        assert_eq!(document.active_layer().name, "LOGO");
        assert_eq!(document.layers[0].pixels.get_pixel(1, 0), Some(white));
        assert_eq!(
            document.layers[0].pixels.get_pixel(0, 0),
            Some(Color::rgba(0, 0, 0, 0))
        );
        assert_eq!(document.active_layer().pixels.get_pixel(0, 1), Some(red));
        assert_eq!(
            document.active_layer().pixels.get_pixel(0, 0),
            Some(Color::rgba(0, 0, 0, 0))
        );
    }
}
