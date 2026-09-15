use super::*;

impl UiContext {
    /// Shared HSV/RGB picker. `square` controls its position and size; the hue,
    /// preview, hexadecimal value and RGB sliders follow below it. Uses five IDs.
    pub fn color_picker(
        &mut self,
        framebuffer: &mut FrameBuffer,
        base_id: u32,
        square: Rect,
        color: &mut Color,
        hue: &mut u32,
    ) -> bool {
        let before = (*color, *hue);
        let (_, saturation, value) = color.to_hsv();
        let mut saturation = u32::from(saturation);
        let mut value = u32::from(value);
        let alpha = color.alpha();
        let bottom = square.y + square.height as i32;
        let color_changed = self.color_square(
            framebuffer,
            base_id,
            square,
            *hue,
            &mut saturation,
            &mut value,
        );
        self.label(framebuffer, square.x + 12, bottom + 8, &format!("H {hue}"));
        let hue_changed = self
            .hue_slider(
                framebuffer,
                base_id + 1,
                Rect::new(square.x, bottom + 26, square.width, 16),
                hue,
            )
            .changed;
        if color_changed || hue_changed {
            let rgb = Color::from_hsv(*hue, saturation as u8, value as u8);
            *color = Color::rgba(rgb.red(), rgb.green(), rgb.blue(), alpha);
        }

        let mut channels = [
            u32::from(color.red()),
            u32::from(color.green()),
            u32::from(color.blue()),
        ];
        let preview = Rect::new(
            square.x + 12,
            bottom + 50,
            square.width.saturating_sub(24),
            20,
        );
        framebuffer.fill_rect(preview, Color::rgb(224, 224, 224));
        let half = preview.width / 2;
        framebuffer.fill_rect(
            Rect::new(preview.x + half as i32, preview.y, preview.width - half, 10),
            Color::rgb(176, 176, 176),
        );
        framebuffer.fill_rect(
            Rect::new(preview.x, preview.y + 10, half, 10),
            Color::rgb(176, 176, 176),
        );
        framebuffer.fill_rect(preview, *color);
        framebuffer.draw_rect(preview, Color::rgb(225, 228, 232));
        self.label(
            framebuffer,
            square.x + 4,
            bottom + 76,
            &format!(
                "{:02X}{:02X}{:02X}{alpha:02X}",
                channels[0], channels[1], channels[2]
            ),
        );

        let mut changed = false;
        for (index, (label, track)) in [
            ('R', Color::rgb(210, 60, 60)),
            ('G', Color::rgb(55, 170, 90)),
            ('B', Color::rgb(60, 120, 220)),
        ]
        .into_iter()
        .enumerate()
        {
            let y = bottom + 95 + index as i32 * 20;
            self.label(
                framebuffer,
                square.x,
                y + 2,
                &format!("{label}{}", channels[index]),
            );
            changed |= self
                .slider(
                    framebuffer,
                    base_id + 2 + index as u32,
                    Rect::new(square.x + 52, y, square.width.saturating_sub(52), 16),
                    &mut channels[index],
                    255,
                    track,
                )
                .changed;
        }
        if changed {
            *color = Color::rgba(
                channels[0] as u8,
                channels[1] as u8,
                channels[2] as u8,
                alpha,
            );
            let (new_hue, saturation, _) = color.to_hsv();
            if saturation > 0 {
                *hue = new_hue;
            }
        }
        (*color, *hue) != before
    }
}
