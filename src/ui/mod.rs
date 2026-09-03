use crate::{
    graphics::{Color, FrameBuffer, Rect},
    platform::{Event, Key, MouseButton},
};

const TEXT_SCALE: u32 = 2;

pub struct SliderResponse {
    pub changed: bool,
    pub started: bool,
    pub dragging: bool,
}

#[derive(Default)]
pub struct UiContext {
    pointer: (i32, i32),
    left_down: bool,
    mouse_pressed_at: Option<(i32, i32)>,
    backspace_pressed: bool,
    activate_pressed: bool,
    pending_text: String,
    focused: Option<u32>,
    dragging: Option<u32>,
    slider_step: i8,
    focus_order: Vec<u32>,
}

impl UiContext {
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::MouseMove { x, y } => self.pointer = (*x, *y),
            Event::MouseDown { button } if *button == MouseButton::Left => {
                self.left_down = true;
                self.mouse_pressed_at = Some(self.pointer);
            }
            Event::MouseUp { button } if *button == MouseButton::Left => {
                self.left_down = false;
            }
            Event::KeyDown { key: Key::Tab } => self.focus_next(),
            Event::KeyDown {
                key: Key::Enter | Key::Space,
            } => self.activate_pressed = true,
            Event::KeyDown {
                key: Key::Backspace,
            } => self.backspace_pressed = true,
            Event::KeyDown {
                key: Key::Left | Key::Down,
            } => self.slider_step = -1,
            Event::KeyDown {
                key: Key::Right | Key::Up,
            } => self.slider_step = 1,
            Event::TextInput { character } if !character.is_control() => {
                self.pending_text.push(*character);
            }
            _ => {}
        }
    }

    pub fn begin_frame(&mut self) {
        self.focus_order.clear();
    }

    pub fn end_frame(&mut self) {
        self.mouse_pressed_at = None;
        self.backspace_pressed = false;
        self.activate_pressed = false;
        self.slider_step = 0;
        self.pending_text.clear();
    }

    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    pub fn panel(&self, framebuffer: &mut FrameBuffer, rect: Rect) {
        framebuffer.fill_rect(rect, Color::rgba(35, 39, 46, 245));
        framebuffer.draw_rect(rect, Color::rgb(88, 94, 105));
    }

    pub fn label(&self, framebuffer: &mut FrameBuffer, x: i32, y: i32, text: &str) {
        self.colored_label(framebuffer, x, y, text, Color::rgb(225, 228, 232));
    }

    pub fn colored_label(
        &self,
        framebuffer: &mut FrameBuffer,
        x: i32,
        y: i32,
        text: &str,
        color: Color,
    ) {
        framebuffer.draw_text(x, y, text, color, TEXT_SCALE);
    }

    pub fn button(
        &mut self,
        framebuffer: &mut FrameBuffer,
        id: u32,
        rect: Rect,
        text: &str,
    ) -> bool {
        self.register_focus(id);
        let hovered = rect.contains(self.pointer.0, self.pointer.1);
        let pressed = self
            .mouse_pressed_at
            .is_some_and(|(x, y)| rect.contains(x, y));
        if pressed {
            self.focused = Some(id);
        }

        let focused = self.focused == Some(id);
        let background = if hovered && self.left_down {
            Color::rgb(65, 105, 160)
        } else if hovered || focused {
            Color::rgb(72, 118, 180)
        } else {
            Color::rgb(58, 92, 140)
        };
        framebuffer.fill_rect(rect, background);
        framebuffer.draw_rect(rect, Color::rgb(150, 170, 195));

        let text_width = FrameBuffer::measure_text(text, TEXT_SCALE);
        let text_x = rect.x + rect.width.saturating_sub(text_width) as i32 / 2;
        let text_y = rect.y + rect.height.saturating_sub(7 * TEXT_SCALE) as i32 / 2;
        framebuffer.draw_text(text_x, text_y, text, Color::rgb(255, 255, 255), TEXT_SCALE);

        pressed || (focused && self.activate_pressed)
    }

    pub fn text_input(
        &mut self,
        framebuffer: &mut FrameBuffer,
        id: u32,
        rect: Rect,
        value: &mut String,
    ) {
        self.register_focus(id);
        let hovered = rect.contains(self.pointer.0, self.pointer.1);
        if let Some((x, y)) = self.mouse_pressed_at {
            if rect.contains(x, y) {
                self.focused = Some(id);
            } else if self.focused == Some(id) {
                self.focused = None;
            }
        }

        let focused = self.focused == Some(id);
        let max_characters = (rect.width.saturating_sub(16) / (6 * TEXT_SCALE)).max(1) as usize;
        if focused {
            if self.backspace_pressed {
                value.pop();
            }
            for character in self.pending_text.chars() {
                if value.chars().count() < max_characters {
                    value.push(character);
                }
            }
        }

        framebuffer.fill_rect(rect, Color::rgb(22, 25, 30));
        framebuffer.draw_rect(
            rect,
            if focused {
                Color::rgb(90, 155, 230)
            } else if hovered {
                Color::rgb(120, 130, 145)
            } else {
                Color::rgb(75, 82, 92)
            },
        );

        let characters: Vec<_> = value.chars().collect();
        let visible: String = characters[characters.len().saturating_sub(max_characters)..]
            .iter()
            .collect();
        let text_x = rect.x + 8;
        let text_y = rect.y + rect.height.saturating_sub(7 * TEXT_SCALE) as i32 / 2;
        framebuffer.draw_text(
            text_x,
            text_y,
            &visible,
            Color::rgb(225, 228, 232),
            TEXT_SCALE,
        );

        if focused {
            let caret_x = text_x + FrameBuffer::measure_text(&visible, TEXT_SCALE) as i32 + 3;
            framebuffer.fill_rect(
                Rect::new(caret_x, text_y, 2, 7 * TEXT_SCALE),
                Color::rgb(225, 228, 232),
            );
        }
    }

    pub fn slider(
        &mut self,
        framebuffer: &mut FrameBuffer,
        id: u32,
        rect: Rect,
        value: &mut u32,
        maximum: u32,
        color: Color,
    ) -> SliderResponse {
        assert!(maximum > 0);
        self.register_focus(id);
        let pressed = self
            .mouse_pressed_at
            .is_some_and(|(x, y)| rect.contains(x, y));
        if pressed {
            self.focused = Some(id);
            self.dragging = Some(id);
        }

        let previous = *value;
        if pressed || self.dragging == Some(id) {
            let track_max = rect.width.saturating_sub(1);
            if track_max > 0 {
                let position = (self.pointer.0 - rect.x).clamp(0, track_max as i32) as u32;
                *value = ((u64::from(position) * u64::from(maximum) + u64::from(track_max) / 2)
                    / u64::from(track_max)) as u32;
            }
            if !self.left_down {
                self.dragging = None;
            }
        } else if self.focused == Some(id) {
            match self.slider_step {
                -1 => *value = value.saturating_sub(1),
                1 => *value = value.saturating_add(1).min(maximum),
                _ => {}
            }
        }

        framebuffer.fill_rect(rect, Color::rgb(22, 25, 30));
        let filled =
            (u64::from(*value) * u64::from(rect.width)).div_ceil(u64::from(maximum)) as u32;
        framebuffer.fill_rect(Rect::new(rect.x, rect.y, filled, rect.height), color);
        framebuffer.draw_rect(
            rect,
            if self.focused == Some(id) {
                Color::rgb(90, 155, 230)
            } else {
                Color::rgb(120, 130, 145)
            },
        );
        let thumb_x = rect.x
            + (u64::from(*value) * u64::from(rect.width.saturating_sub(1)) / u64::from(maximum))
                as i32;
        framebuffer.fill_rect(
            Rect::new(thumb_x - 2, rect.y + 2, 4, rect.height.saturating_sub(4)),
            Color::rgb(255, 255, 255),
        );
        SliderResponse {
            changed: previous != *value,
            started: pressed,
            dragging: self.dragging == Some(id) && self.left_down,
        }
    }

    pub fn radio_button(
        &mut self,
        framebuffer: &mut FrameBuffer,
        id: u32,
        x: i32,
        y: i32,
        text: &str,
        selected: bool,
    ) -> bool {
        self.register_focus(id);
        let hit_area = Rect::new(
            x - 10,
            y - 10,
            30 + FrameBuffer::measure_text(text, TEXT_SCALE),
            20,
        );
        let hovered = hit_area.contains(self.pointer.0, self.pointer.1);
        let pressed = self
            .mouse_pressed_at
            .is_some_and(|(pointer_x, pointer_y)| hit_area.contains(pointer_x, pointer_y));
        if pressed {
            self.focused = Some(id);
        }

        let focused = self.focused == Some(id);
        framebuffer.draw_circle(
            x,
            y,
            8,
            if hovered || focused {
                Color::rgb(90, 155, 230)
            } else {
                Color::rgb(150, 160, 175)
            },
        );
        if selected {
            framebuffer.fill_circle(x, y, 4, Color::rgb(90, 155, 230));
        }
        framebuffer.draw_text(x + 16, y - 7, text, Color::rgb(225, 228, 232), TEXT_SCALE);

        pressed || (focused && self.activate_pressed)
    }

    fn register_focus(&mut self, id: u32) {
        if !self.focus_order.contains(&id) {
            self.focus_order.push(id);
        }
    }

    fn focus_next(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }
        let index = self
            .focused
            .and_then(|focused| self.focus_order.iter().position(|id| *id == focused))
            .map_or(0, |index| (index + 1) % self.focus_order.len());
        self.focused = Some(self.focus_order[index]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_accepts_focus_text_and_backspace() {
        let mut ui = UiContext::default();
        let mut framebuffer = FrameBuffer::default();
        let mut value = String::new();
        let rect = Rect::new(10, 10, 160, 32);
        framebuffer.resize(200, 60);

        ui.handle_event(&Event::MouseMove { x: 20, y: 20 });
        ui.handle_event(&Event::MouseDown {
            button: MouseButton::Left,
        });
        ui.handle_event(&Event::MouseMove { x: 190, y: 50 });
        ui.text_input(&mut framebuffer, 1, rect, &mut value);
        ui.end_frame();
        ui.handle_event(&Event::TextInput { character: 'A' });
        ui.text_input(&mut framebuffer, 1, rect, &mut value);
        ui.end_frame();
        assert_eq!(value, "A");

        ui.handle_event(&Event::KeyDown {
            key: Key::Backspace,
        });
        ui.text_input(&mut framebuffer, 1, rect, &mut value);
        assert!(value.is_empty());
    }

    #[test]
    fn slider_supports_pointer_and_keyboard_adjustment() {
        let mut ui = UiContext::default();
        let mut framebuffer = FrameBuffer::default();
        let mut value = 0;
        let rect = Rect::new(10, 10, 101, 20);
        framebuffer.resize(130, 40);

        ui.handle_event(&Event::MouseMove { x: 110, y: 20 });
        ui.handle_event(&Event::MouseDown {
            button: MouseButton::Left,
        });
        assert!(
            ui.slider(
                &mut framebuffer,
                1,
                rect,
                &mut value,
                255,
                Color::rgb(255, 0, 0)
            )
            .changed
        );
        assert_eq!(value, 255);
        ui.end_frame();
        ui.handle_event(&Event::MouseUp {
            button: MouseButton::Left,
        });
        ui.slider(
            &mut framebuffer,
            1,
            rect,
            &mut value,
            255,
            Color::rgb(255, 0, 0),
        );
        ui.end_frame();

        ui.handle_event(&Event::KeyDown { key: Key::Left });
        assert!(
            ui.slider(
                &mut framebuffer,
                1,
                rect,
                &mut value,
                255,
                Color::rgb(255, 0, 0)
            )
            .changed
        );
        assert_eq!(value, 254);
    }
}
