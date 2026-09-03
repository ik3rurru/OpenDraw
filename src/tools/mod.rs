mod brush;
mod eraser;

pub use brush::BrushTool;
pub use eraser::EraserTool;

use crate::{document::PixelBuffer, graphics::rasterize_line};

pub trait Tool {
    fn pointer_down(&mut self, pixels: &mut PixelBuffer, point: (u32, u32));
    fn pointer_move(&mut self, pixels: &mut PixelBuffer, point: (u32, u32));
    fn pointer_up(&mut self);
    fn break_segment(&mut self);
    fn is_active(&self) -> bool;
}

#[derive(Default)]
struct Stroke {
    active: bool,
    previous_position: Option<(u32, u32)>,
}

impl Stroke {
    fn pointer_down(&mut self, point: (u32, u32), mut stamp: impl FnMut(u32, u32)) {
        self.active = true;
        self.previous_position = Some(point);
        stamp(point.0, point.1);
    }

    fn pointer_move(&mut self, point: (u32, u32), radius: u32, mut stamp: impl FnMut(u32, u32)) {
        if !self.active {
            return;
        }
        let Some(previous) = self.previous_position else {
            self.previous_position = Some(point);
            stamp(point.0, point.1);
            return;
        };

        if radius == 0 {
            let mut first = true;
            rasterize_line(
                previous.0.into(),
                previous.1.into(),
                point.0.into(),
                point.1.into(),
                |x, y| {
                    if first {
                        first = false;
                    } else {
                        stamp(x as u32, y as u32);
                    }
                },
            );
        } else {
            let delta_x = point.0 as f32 - previous.0 as f32;
            let delta_y = point.1 as f32 - previous.1 as f32;
            let distance = delta_x.hypot(delta_y);
            let spacing = ((radius * 2 + 1) as f32 / 4.0).max(1.0);
            let steps = (distance / spacing).ceil().max(1.0) as u32;
            for step in 1..=steps {
                let progress = step as f32 / steps as f32;
                stamp(
                    (previous.0 as f32 + delta_x * progress).round() as u32,
                    (previous.1 as f32 + delta_y * progress).round() as u32,
                );
            }
        }
        self.previous_position = Some(point);
    }

    fn pointer_up(&mut self) {
        self.active = false;
        self.previous_position = None;
    }

    fn break_segment(&mut self) {
        self.previous_position = None;
    }
}
