use crate::graphics::{Color, FrameBuffer, Rect};

use super::Document;

#[derive(Clone, Copy, Debug)]
pub struct CanvasView {
    pub zoom: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl CanvasView {
    pub fn fit(document: &Document, viewport: Rect) -> Self {
        let zoom = (viewport.width as f32 / document.width as f32)
            .min(viewport.height as f32 / document.height as f32)
            .min(1.0)
            .clamp(0.05, 32.0);
        Self {
            zoom,
            offset_x: viewport.x as f32
                + (viewport.width as f32 - document.width as f32 * zoom) / 2.0,
            offset_y: viewport.y as f32
                + (viewport.height as f32 - document.height as f32 * zoom) / 2.0,
        }
    }

    pub fn screen_to_canvas(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.offset_x) / self.zoom,
            (y - self.offset_y) / self.zoom,
        )
    }

    pub fn canvas_to_screen(&self, x: f32, y: f32) -> (f32, f32) {
        (self.offset_x + x * self.zoom, self.offset_y + y * self.zoom)
    }

    pub fn pan(&mut self, delta_x: i32, delta_y: i32) {
        self.offset_x += delta_x as f32;
        self.offset_y += delta_y as f32;
    }

    pub fn zoom_at(&mut self, factor: f32, screen_x: i32, screen_y: i32) {
        let (canvas_x, canvas_y) = self.screen_to_canvas(screen_x as f32, screen_y as f32);
        self.zoom = (self.zoom * factor).clamp(0.05, 32.0);
        self.offset_x = screen_x as f32 - canvas_x * self.zoom;
        self.offset_y = screen_y as f32 - canvas_y * self.zoom;
    }

    pub fn render(&self, document: &Document, framebuffer: &mut FrameBuffer, viewport: Rect) {
        let (document_left, document_top) = self.canvas_to_screen(0.0, 0.0);
        let (document_right, document_bottom) =
            self.canvas_to_screen(document.width as f32, document.height as f32);
        let left = (document_left.floor() as i64).max(viewport.x as i64).max(0);
        let top = (document_top.floor() as i64).max(viewport.y as i64).max(0);
        let right = (document_right.ceil() as i64)
            .min(viewport.x as i64 + viewport.width as i64)
            .min(framebuffer.width as i64);
        let bottom = (document_bottom.ceil() as i64)
            .min(viewport.y as i64 + viewport.height as i64)
            .min(framebuffer.height as i64);
        let layer = document.active_layer();

        for screen_y in top..bottom {
            for screen_x in left..right {
                let checker = if (((screen_x - document_left.floor() as i64) / 12)
                    + ((screen_y - document_top.floor() as i64) / 12))
                    % 2
                    == 0
                {
                    Color::rgb(224, 224, 224)
                } else {
                    Color::rgb(176, 176, 176)
                };
                framebuffer.set_pixel(screen_x as i32, screen_y as i32, checker);

                if !layer.visible {
                    continue;
                }
                let (canvas_x, canvas_y) = self.screen_to_canvas(screen_x as f32, screen_y as f32);
                let Some(pixel) = layer.pixels.get_pixel(canvas_x as u32, canvas_y as u32) else {
                    continue;
                };
                let pixel = Color::rgba(
                    pixel.red(),
                    pixel.green(),
                    pixel.blue(),
                    ((u16::from(pixel.alpha()) * u16::from(layer.opacity) + 127) / 255) as u8,
                );
                framebuffer.blend_pixel(screen_x as i32, screen_y as i32, pixel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_coordinates_and_keeps_zoom_anchor_fixed() {
        let mut view = CanvasView {
            zoom: 2.0,
            offset_x: 10.0,
            offset_y: 20.0,
        };
        assert_eq!(view.canvas_to_screen(3.0, 4.0), (16.0, 28.0));
        assert_eq!(view.screen_to_canvas(16.0, 28.0), (3.0, 4.0));

        view.zoom_at(2.0, 16, 28);
        assert_eq!(view.screen_to_canvas(16.0, 28.0), (3.0, 4.0));

        let document = Document::new(2, 2, Color::rgb(255, 255, 255)).unwrap();
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(40, 40);
        framebuffer.clear(Color::rgb(0, 0, 0));
        view.render(&document, &mut framebuffer, Rect::new(0, 0, 40, 40));
        assert_eq!(
            framebuffer.get_pixel(10, 18),
            Some(Color::rgb(255, 255, 255))
        );
        assert_eq!(framebuffer.get_pixel(0, 0), Some(Color::rgb(0, 0, 0)));
    }
}
