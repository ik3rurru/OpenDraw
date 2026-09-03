use crate::{
    graphics::{Color, FrameBuffer, Rect},
    platform::{Event, Key, MouseButton},
};

const TEXT_SCALE: u32 = 2;

#[derive(Default)]
pub struct UiContext {
    pointer: (i32, i32),
    left_down: bool,
    mouse_pressed: bool,
    backspace_pressed: bool,
    activate_pressed: bool,
    pending_text: String,
    focused: Option<u32>,
    focus_order: Vec<u32>,
}

impl UiContext {
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::MouseMove { x, y } => self.pointer = (*x, *y),
            Event::MouseDown { button } if *button == MouseButton::Left => {
                self.left_down = true;
                self.mouse_pressed = true;
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
        self.mouse_pressed = false;
        self.backspace_pressed = false;
        self.activate_pressed = false;
        self.pending_text.clear();
    }

    pub fn panel(&self, framebuffer: &mut FrameBuffer, rect: Rect) {
        framebuffer.fill_rect(rect, Color::rgba(35, 39, 46, 245));
        framebuffer.draw_rect(rect, Color::rgb(88, 94, 105));
    }

    pub fn label(&self, framebuffer: &mut FrameBuffer, x: i32, y: i32, text: &str) {
        framebuffer.draw_text(x, y, text, Color::rgb(225, 228, 232), TEXT_SCALE);
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
        if hovered && self.mouse_pressed {
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

        (hovered && self.mouse_pressed) || (focused && self.activate_pressed)
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
        if self.mouse_pressed {
            if hovered {
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
}
