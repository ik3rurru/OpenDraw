use crate::{document::PixelBuffer, graphics::Color};

use super::{BrushDynamics, BrushSample, Stroke, Tool};

pub struct BrushSettings {
    pub radius: u32,
    pub color: Color,
    pub opacity: u8,
    pub dynamics: BrushDynamics,
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
                dynamics: BrushDynamics::default(),
            },
            stroke: Stroke::default(),
        }
    }
}

impl Tool for BrushTool {
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

impl BrushSettings {
    fn spacing(&self, sample: BrushSample) -> f32 {
        self.dynamics.radius(self.radius, sample.pressure) / 2.0
    }

    fn stamp(&self, pixels: &mut PixelBuffer, sample: BrushSample) {
        let color = Color::rgba(
            self.color.red(),
            self.color.green(),
            self.color.blue(),
            self.dynamics.opacity(self.opacity, sample.pressure),
        );
        pixels.stamp_circle(
            sample.x,
            sample.y,
            self.dynamics.radius(self.radius, sample.pressure),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::tools::stroke::sample;

    #[test]
    fn paints_clipped_interpolated_circular_strokes_with_alpha() {
        let mut document = Document::new(24, 16, Color::rgba(0, 0, 0, 0)).unwrap();
        let mut brush = BrushTool {
            settings: BrushSettings {
                radius: 2,
                color: Color::rgb(220, 40, 30),
                opacity: 128,
                dynamics: BrushDynamics::default(),
            },
            ..BrushTool::default()
        };
        let pixels = &mut document.active_layer_mut().pixels;

        brush.pointer_down(pixels, sample(0.0, 0.0, 1.0));
        assert_eq!(pixels.get_pixel(0, 0).unwrap().alpha(), 128);
        brush.pointer_move(pixels, sample(20.0, 8.0, 1.0));
        brush.pointer_up(pixels);

        assert!(pixels.get_pixel(10, 4).unwrap().alpha() > 0);
        assert_eq!(pixels.get_pixel(10, 8).unwrap().alpha(), 0);
        assert!(!brush.is_active());
    }

    #[test]
    fn pressure_is_interpolated_into_width_and_opacity_along_a_segment() {
        let mut document = Document::new(100, 50, Color::rgba(0, 0, 0, 0)).unwrap();
        let pixels = &mut document.active_layer_mut().pixels;
        let mut brush = BrushTool::default();
        brush.settings.radius = 10;
        brush.pointer_down(pixels, sample(10.0, 25.0, 0.2));
        assert_eq!(pixels.get_pixel(10, 25).unwrap().alpha(), 51);
        assert_eq!(pixels.get_pixel(10, 29).unwrap().alpha(), 0);
        brush.pointer_move(pixels, sample(85.0, 25.0, 1.0));
        brush.pointer_up(pixels);
        for x in 10..=85 {
            assert!(pixels.get_pixel(x, 25).unwrap().alpha() > 0);
        }
        assert_eq!(pixels.get_pixel(20, 34).unwrap().alpha(), 0);
        assert!(pixels.get_pixel(50, 29).unwrap().alpha() > 0);
        assert!(pixels.get_pixel(85, 34).unwrap().alpha() > 0);
    }

    #[test]
    fn sample_rate_does_not_change_the_rendered_stroke() {
        fn draw(steps: u32) -> Vec<u32> {
            let mut document = Document::new(100, 50, Color::rgba(0, 0, 0, 0)).unwrap();
            let pixels = &mut document.active_layer_mut().pixels;
            let mut brush = BrushTool::default();
            brush.settings.radius = 10;
            brush.settings.opacity = 90;
            brush.pointer_down(pixels, sample(10.0, 25.0, 0.25));
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                brush.pointer_move(pixels, sample(10.0 + 64.0 * t, 25.0, 0.25 + 0.75 * t));
            }
            brush.pointer_up(pixels);
            pixels.pixels.clone()
        }
        assert_eq!(draw(1), draw(256));
    }

    #[test]
    fn fractional_diagonals_and_pressure_ramps_are_independent_of_packet_count() {
        fn draw(radius: u32, steps: u32, duplicates: bool) -> Vec<u32> {
            let mut document = Document::new(100, 60, Color::rgba(0, 0, 0, 0)).unwrap();
            let pixels = &mut document.active_layer_mut().pixels;
            let mut brush = BrushTool::default();
            brush.settings.radius = radius;
            brush.settings.opacity = 90;
            brush.pointer_down(pixels, sample(10.25, 15.75, 0.25));
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                let point = sample(10.25 + 64.0 * t, 15.75 + 32.0 * t, 0.25 + 0.75 * t);
                brush.pointer_move(pixels, point);
                if duplicates {
                    for _ in 0..8 {
                        brush.pointer_move(pixels, point);
                    }
                }
            }
            brush.pointer_up(pixels);
            pixels.pixels.clone()
        }
        for radius in [0, 1, 4, 10] {
            let expected = draw(radius, 1, false);
            for steps in [2, 16, 256, 1024] {
                assert_eq!(
                    draw(radius, steps, false),
                    expected,
                    "r={radius}, packets={steps}"
                );
                assert_eq!(draw(radius, steps, true), expected, "duplicate packets");
            }
        }
    }

    #[test]
    fn minimum_brush_is_visible_and_duplicate_contacts_preserve_its_opacity() {
        for pressure in [0.1, 0.5, 1.0] {
            let mut document = Document::new(4, 4, Color::rgba(0, 0, 0, 0)).unwrap();
            let pixels = &mut document.active_layer_mut().pixels;
            let mut brush = BrushTool::default();
            brush.settings.radius = 0;
            let point = sample(1.5, 1.5, pressure);
            brush.pointer_down(pixels, point);
            let initial = pixels.pixels.clone();
            for _ in 0..100 {
                brush.pointer_move(pixels, point);
            }
            brush.pointer_up(pixels);
            assert_eq!(pixels.pixels, initial);
            assert!(pixels.get_pixel(1, 1).unwrap().alpha() >= 25);
            assert_eq!(pixels.get_pixel(2, 1).unwrap().alpha(), 0);
        }
    }

    #[test]
    fn breaking_a_segment_does_not_connect_across_the_outside_of_the_canvas() {
        let mut document = Document::new(30, 1, Color::rgba(0, 0, 0, 0)).unwrap();
        let pixels = &mut document.active_layer_mut().pixels;
        let mut brush = BrushTool::default();
        brush.settings.radius = 0;
        brush.pointer_down(pixels, sample(2.5, 0.5, 1.0));
        brush.break_segment(pixels);
        brush.pointer_move(pixels, sample(25.5, 0.5, 1.0));
        brush.pointer_up(pixels);
        assert_eq!(pixels.get_pixel(15, 0).unwrap().alpha(), 0);
        assert_eq!(pixels.get_pixel(25, 0).unwrap().alpha(), 255);
    }
}
