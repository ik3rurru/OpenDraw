#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(u32);

impl Color {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self(((alpha as u32) << 24) | ((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
    }

    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }

    pub const fn red(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub const fn green(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub const fn blue(self) -> u8 {
        self.0 as u8
    }

    pub const fn alpha(self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn blend_over(self, background: Self) -> Self {
        let source_alpha = self.alpha() as u32;
        if source_alpha == 0 {
            return background;
        }
        if source_alpha == 255 {
            return self;
        }

        let background_alpha = background.alpha() as u32;
        let inverse_alpha = 255 - source_alpha;
        let alpha_numerator = source_alpha * 255 + background_alpha * inverse_alpha;
        let channel = |source: u8, destination: u8| {
            ((source as u32 * source_alpha * 255
                + destination as u32 * background_alpha * inverse_alpha
                + alpha_numerator / 2)
                / alpha_numerator) as u8
        };

        Self::rgba(
            channel(self.red(), background.red()),
            channel(self.green(), background.green()),
            channel(self.blue(), background.blue()),
            ((alpha_numerator + 127) / 255) as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_blending_uses_source_over() {
        let purple = Color::rgba(255, 0, 0, 128).blend_over(Color::rgb(0, 0, 255));
        assert_eq!(purple, Color::rgb(128, 0, 127));

        let red = Color::rgba(255, 0, 0, 128).blend_over(Color::rgba(0, 0, 0, 0));
        assert_eq!(red, Color::rgba(255, 0, 0, 128));
    }
}
