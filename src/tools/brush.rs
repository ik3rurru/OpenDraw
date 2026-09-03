use crate::{
    document::PixelBuffer,
    graphics::{Color, rasterize_line},
};

pub struct BrushSettings {
    pub radius: u32,
    pub color: Color,
    pub opacity: u8,
}

pub struct BrushTool {
    pub settings: BrushSettings,
    active: bool,
    previous_position: Option<(u32, u32)>,
}

impl Default for BrushTool {
    fn default() -> Self {
        Self {
            settings: BrushSettings {
                radius: 4,
                color: Color::rgb(24, 24, 24),
                opacity: 255,
            },
            active: false,
            previous_position: None,
        }
    }
}

impl BrushTool {
    pub fn pointer_down(&mut self, pixels: &mut PixelBuffer, point: (u32, u32)) {
        self.active = true;
        self.previous_position = Some(point);
        pixels.stamp_circle(point.0, point.1, self.settings.radius, self.paint_color());
    }

    pub fn pointer_move(&mut self, pixels: &mut PixelBuffer, point: (u32, u32)) {
        if !self.active {
            return;
        }
        let Some(previous) = self.previous_position else {
            self.pointer_down(pixels, point);
            return;
        };
        let color = self.paint_color();
        if self.settings.radius == 0 {
            rasterize_line(
                previous.0.into(),
                previous.1.into(),
                point.0.into(),
                point.1.into(),
                |x, y| pixels.stamp_circle(x as u32, y as u32, 0, color),
            );
        } else {
            let delta_x = point.0 as f32 - previous.0 as f32;
            let delta_y = point.1 as f32 - previous.1 as f32;
            let distance = delta_x.hypot(delta_y);
            let spacing = ((self.settings.radius * 2 + 1) as f32 / 4.0).max(1.0);
            let steps = (distance / spacing).ceil().max(1.0) as u32;
            for step in 1..=steps {
                let progress = step as f32 / steps as f32;
                pixels.stamp_circle(
                    (previous.0 as f32 + delta_x * progress).round() as u32,
                    (previous.1 as f32 + delta_y * progress).round() as u32,
                    self.settings.radius,
                    color,
                );
            }
        }
        self.previous_position = Some(point);
    }

    pub fn break_segment(&mut self) {
        self.previous_position = None;
    }

    pub fn pointer_up(&mut self) {
        self.active = false;
        self.previous_position = None;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

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
