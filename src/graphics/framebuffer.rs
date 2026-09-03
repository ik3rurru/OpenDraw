use super::Color;

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
                self.pixels[(y * self.width + x) as usize] = color.as_bgrx();
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
            .map(Color::as_bgrx)
        );
    }
}
