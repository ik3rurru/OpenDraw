use crate::document::PixelBuffer;

use super::{BrushDynamics, BrushSample, Stroke, Tool};

pub struct EraserSettings {
    pub radius: u32,
    pub opacity: u8,
    pub dynamics: BrushDynamics,
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
                dynamics: BrushDynamics::default(),
            },
            stroke: Stroke::default(),
        }
    }
}

impl Tool for EraserTool {
    fn pointer_down(&mut self, pixels: &mut PixelBuffer, sample: BrushSample) {
        let settings = &self.settings;
        self.stroke.pointer_down(
            sample,
            |s| settings.spacing(s),
            |s| settings.stamp(pixels, s),
        );
    }

    fn pointer_move(&mut self, pixels: &mut PixelBuffer, sample: BrushSample) {
        let settings = &self.settings;
        self.stroke.pointer_move(
            sample,
            |s| settings.spacing(s),
            |s| settings.stamp(pixels, s),
        );
    }

    fn pointer_up(&mut self, pixels: &mut PixelBuffer) {
        let settings = &self.settings;
        self.stroke.pointer_up(|s| settings.stamp(pixels, s));
    }

    fn cancel(&mut self) {
        self.stroke.cancel();
    }

    fn break_segment(&mut self, pixels: &mut PixelBuffer) {
        let settings = &self.settings;
        self.stroke.break_segment(|s| settings.stamp(pixels, s));
    }

    fn is_active(&self) -> bool {
        self.stroke.active
    }
}

impl EraserSettings {
    fn spacing(&self, sample: BrushSample) -> f32 {
        self.dynamics.radius(self.radius, sample.pressure) / 2.0
    }

    fn stamp(&self, pixels: &mut PixelBuffer, sample: BrushSample) {
        pixels.erase_circle(
            sample.x,
            sample.y,
            self.dynamics.radius(self.radius, sample.pressure),
            self.dynamics.opacity(self.opacity, sample.pressure),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::stroke::sample;
    use crate::{document::Document, graphics::Color, tools::BrushTool};

    #[test]
    fn erases_alpha_with_the_shared_continuous_stroke() {
        let mut document = Document::new(24, 16, Color::rgba(0, 0, 0, 0)).unwrap();
        let pixels = &mut document.active_layer_mut().pixels;
        let mut brush = BrushTool::default();
        brush.settings.radius = 3;
        brush.pointer_down(pixels, sample(0.0, 4.0, 1.0));
        brush.pointer_move(pixels, sample(20.0, 4.0, 1.0));
        brush.pointer_up(pixels);

        let mut eraser = EraserTool::default();
        eraser.settings.radius = 2;
        eraser.settings.opacity = 128;
        eraser.pointer_down(pixels, sample(0.0, 4.0, 1.0));
        assert_eq!(pixels.get_pixel(0, 4).unwrap().alpha(), 127);
        eraser.pointer_move(pixels, sample(20.0, 4.0, 1.0));
        eraser.pointer_up(pixels);

        assert!(pixels.get_pixel(10, 4).unwrap().alpha() < 255);
        assert_eq!(pixels.get_pixel(10, 9).unwrap().alpha(), 0);
        assert!(!eraser.is_active());
    }

    #[test]
    fn eraser_pressure_and_fractional_edges_are_independent_of_packet_count() {
        fn draw(steps: u32) -> Vec<u32> {
            let mut document = Document::new(100, 60, Color::rgba(40, 80, 190, 180)).unwrap();
            let pixels = &mut document.active_layer_mut().pixels;
            let mut eraser = EraserTool::default();
            eraser.settings.radius = 7;
            eraser.settings.opacity = 120;
            eraser.pointer_down(pixels, sample(10.25, 15.75, 0.25));
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                let point = sample(10.25 + 64.0 * t, 15.75 + 32.0 * t, 0.25 + 0.75 * t);
                for _ in 0..3 {
                    eraser.pointer_move(pixels, point);
                }
            }
            eraser.pointer_up(pixels);
            pixels.pixels.clone()
        }
        assert_eq!(draw(1), draw(1024));
        let pixels = draw(1);
        assert!(
            pixels
                .iter()
                .any(|&p| (1..180).contains(&crate::graphics::Color::from_u32(p).alpha()))
        );
    }
}
