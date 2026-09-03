use crate::{document::PixelBuffer, graphics::Color};

use super::{Stroke, Tool};

pub struct BrushSettings {
    pub radius: u32,
    pub color: Color,
    pub opacity: u8,
}

pub struct BrushTool {
    pub settings: BrushSettings,
    stroke: Stroke,
}

impl Default for BrushTool {
    fn default() -> Self {
        Self {
            settings: BrushSettings {
                radius: 4,
                color: Color::rgb(24, 24, 24),
                opacity: 255,
            },
            stroke: Stroke::default(),
        }
    }
}

impl Tool for BrushTool {
    fn pointer_down(&mut self, pixels: &mut PixelBuffer, point: (u32, u32)) {
        let radius = self.settings.radius;
        let color = self.paint_color();
        self.stroke
            .pointer_down(point, |x, y| pixels.stamp_circle(x, y, radius, color));
    }

    fn pointer_move(&mut self, pixels: &mut PixelBuffer, point: (u32, u32)) {
        let radius = self.settings.radius;
        let color = self.paint_color();
        self.stroke.pointer_move(point, radius, |x, y| {
            pixels.stamp_circle(x, y, radius, color)
        });
    }

    fn pointer_up(&mut self) {
        self.stroke.pointer_up();
    }

    fn break_segment(&mut self) {
        self.stroke.break_segment();
    }

    fn is_active(&self) -> bool {
        self.stroke.active
    }
}

impl BrushTool {
    fn paint_color(&self) -> Color {
        Color::rgba(
            self.settings.color.red(),
            self.settings.color.green(),
            self.settings.color.blue(),
            self.settings.opacity,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn paints_clipped_interpolated_circular_strokes_with_alpha() {
        let mut document = Document::new(24, 16, Color::rgba(0, 0, 0, 0)).unwrap();
        let mut brush = BrushTool {
            settings: BrushSettings {
                radius: 2,
                color: Color::rgb(220, 40, 30),
                opacity: 128,
            },
            ..BrushTool::default()
        };
        let pixels = &mut document.active_layer_mut().pixels;

        brush.pointer_down(pixels, (0, 0));
        assert_eq!(pixels.get_pixel(0, 0).unwrap().alpha(), 128);
        brush.pointer_move(pixels, (20, 8));
        brush.pointer_up();

        assert!(pixels.get_pixel(10, 4).unwrap().alpha() > 0);
        assert_eq!(pixels.get_pixel(10, 8).unwrap().alpha(), 0);
        assert!(!brush.is_active());
    }
}
