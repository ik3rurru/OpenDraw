use super::{Color, Rect};

#[derive(Default)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl FrameBuffer {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixels.resize(width as usize * height as usize, 0);
        self.clear(Color::rgba(0, 0, 0, 0));
    }

    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color.as_u32());
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if let Some(index) = self.pixel_index(x, y) {
            self.pixels[index] = color.as_u32();
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Option<Color> {
        self.pixel_index(x, y)
            .map(|index| Color::from_u32(self.pixels[index]))
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Color) {
        if let Some(background) = self.get_pixel(x, y) {
            self.set_pixel(x, y, color.blend_over(background));
        }
    }

    pub fn checkerboard(&mut self, cell_size: u32, light: Color, dark: Color) {
        assert!(cell_size > 0);

        for y in 0..self.height {
            for x in 0..self.width {
                let color = if (x / cell_size + y / cell_size).is_multiple_of(2) {
                    light
                } else {
                    dark
                };
                self.pixels[(y * self.width + x) as usize] = color.as_u32();
            }
        }
    }

    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let (mut x0, mut y0, x1, y1) = (x0 as i64, y0 as i64, x1 as i64, y1 as i64);
        let dx = (x1 - x0).abs();
        let step_x = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let step_y = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.blend_pixel(x0 as i32, y0 as i32, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let doubled_error = 2 * error;
            if doubled_error >= dy {
                error += dy;
                x0 += step_x;
            }
            if doubled_error <= dx {
                error += dx;
                y0 += step_y;
            }
        }
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }

        let top = rect.y as i64;
        let bottom = top + rect.height as i64 - 1;
        let left = rect.x as i64;
        let right = left + rect.width as i64 - 1;

        for x in left..=right {
            self.blend_at(x, top, color);
            if bottom != top {
                self.blend_at(x, bottom, color);
            }
        }
        for y in top + 1..bottom {
            self.blend_at(left, y, color);
            if right != left {
                self.blend_at(right, y, color);
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let top = (rect.y as i64).max(0);
        let bottom = (rect.y as i64 + rect.height as i64).min(self.height as i64);
        let left = rect.x as i64;
        let right = left + rect.width as i64;

        for y in top..bottom {
            self.blend_span(y, left, right, color);
        }
    }

    pub fn draw_circle(&mut self, center_x: i32, center_y: i32, radius: u32, color: Color) {
        let Ok(radius) = i32::try_from(radius) else {
            return;
        };
        let mut x = radius as i64;
        let mut y = 0_i64;
        let mut error = 1 - x;

        while x >= y {
            self.blend_circle_points(center_x, center_y, x, y, color);
            y += 1;
            if error < 0 {
                error += 2 * y + 1;
            } else {
                x -= 1;
                error += 2 * (y - x + 1);
            }
        }
    }

    pub fn fill_circle(&mut self, center_x: i32, center_y: i32, radius: u32, color: Color) {
        let Ok(radius) = i32::try_from(radius) else {
            return;
        };
        let radius = radius as i64;
        let radius_squared = radius * radius;

        for offset_y in -radius..=radius {
            let span = (radius_squared - offset_y * offset_y).isqrt();
            self.blend_span(
                center_y as i64 + offset_y,
                center_x as i64 - span,
                center_x as i64 + span + 1,
                color,
            );
        }
    }

    fn pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    fn blend_at(&mut self, x: i64, y: i64, color: Color) {
        if let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) {
            self.blend_pixel(x, y, color);
        }
    }

    fn blend_span(&mut self, y: i64, left: i64, right: i64, color: Color) {
        if y < 0 || y >= self.height as i64 {
            return;
        }
        let left = left.max(0);
        let right = right.min(self.width as i64);
        for x in left..right {
            self.blend_at(x, y, color);
        }
    }

    fn blend_circle_points(&mut self, center_x: i32, center_y: i32, x: i64, y: i64, color: Color) {
        let points = [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ];

        for (index, &(offset_x, offset_y)) in points.iter().enumerate() {
            if !points[..index].contains(&(offset_x, offset_y)) {
                self.blend_at(
                    center_x as i64 + offset_x,
                    center_y as i64 + offset_y,
                    color,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkerboard_alternates_cells() {
        let light = Color::rgb(255, 255, 255);
        let dark = Color::rgb(0, 0, 0);
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(4, 4);
        framebuffer.checkerboard(2, light, dark);

        assert_eq!(
            framebuffer.pixels,
            [
                light, light, dark, dark, light, light, dark, dark, dark, dark, light, light, dark,
                dark, light, light,
            ]
            .map(Color::as_u32)
        );
    }

    #[test]
    fn primitives_clip_to_the_framebuffer() {
        let white = Color::rgb(255, 255, 255);
        let black = Color::rgb(0, 0, 0);
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(5, 5);
        framebuffer.clear(black);

        framebuffer.draw_line(-2, -2, 2, 2, white);
        framebuffer.draw_rect(Rect::new(3, 3, 4, 4), white);
        framebuffer.fill_circle(2, 2, 1, Color::rgba(255, 0, 0, 128));

        assert_eq!(framebuffer.get_pixel(0, 0), Some(white));
        assert_eq!(framebuffer.get_pixel(4, 3), Some(white));
        assert_eq!(framebuffer.get_pixel(2, 2), Some(Color::rgb(255, 127, 127)));
        assert_eq!(framebuffer.get_pixel(-1, 0), None);
    }
}
