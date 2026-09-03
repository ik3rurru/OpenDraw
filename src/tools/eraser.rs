use crate::document::PixelBuffer;

use super::{Stroke, Tool};

pub struct EraserSettings {
    pub radius: u32,
    pub opacity: u8,
}

pub struct EraserTool {
    pub settings: EraserSettings,
    stroke: Stroke,
}

impl Default for EraserTool {
    fn default() -> Self {
        Self {
            settings: EraserSettings {
                radius: 8,
                opacity: 255,
            },
            stroke: Stroke::default(),
        }
    }
}

impl Tool for EraserTool {
    fn pointer_down(&mut self, pixels: &mut PixelBuffer, point: (u32, u32)) {
        let radius = self.settings.radius;
        let opacity = self.settings.opacity;
        self.stroke
            .pointer_down(point, |x, y| pixels.erase_circle(x, y, radius, opacity));
    }

    fn pointer_move(&mut self, pixels: &mut PixelBuffer, point: (u32, u32)) {
        let radius = self.settings.radius;
        let opacity = self.settings.opacity;
        self.stroke.pointer_move(point, radius, |x, y| {
            pixels.erase_circle(x, y, radius, opacity)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::Document, graphics::Color, tools::BrushTool};

    #[test]
    fn erases_alpha_with_the_shared_continuous_stroke() {
        let mut document = Document::new(24, 16, Color::rgba(0, 0, 0, 0)).unwrap();
        let pixels = &mut document.active_layer_mut().pixels;
        let mut brush = BrushTool::default();
        brush.settings.radius = 3;
        brush.pointer_down(pixels, (0, 4));
        brush.pointer_move(pixels, (20, 4));
        brush.pointer_up();

        let mut eraser = EraserTool::default();
        eraser.settings.radius = 2;
        eraser.settings.opacity = 128;
        eraser.pointer_down(pixels, (0, 4));
        assert_eq!(pixels.get_pixel(0, 4).unwrap().alpha(), 127);
        eraser.pointer_move(pixels, (20, 4));
        eraser.pointer_up();

        assert!(pixels.get_pixel(10, 4).unwrap().alpha() < 255);
        assert_eq!(pixels.get_pixel(10, 9).unwrap().alpha(), 0);
        assert!(!eraser.is_active());
    }
}
