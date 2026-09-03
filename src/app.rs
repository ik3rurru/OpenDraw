use crate::{
    graphics::{Color, FrameBuffer, Rect},
    platform::{Event, Key, MouseButton},
};

pub struct App {
    running: bool,
    window_size: (u32, u32),
    pointer: (i32, i32),
    pointer_visible: bool,
    pressed_button: Option<MouseButton>,
    marker_radius: u32,
    accent: Color,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            window_size: (0, 0),
            pointer: (0, 0),
            pointer_visible: false,
            pressed_button: None,
            marker_radius: 16,
            accent: Color::rgb(220, 50, 47),
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::CloseRequested => self.running = false,
            Event::Resized { width, height } => self.window_size = (width, height),
            Event::MouseMove { x, y } => {
                self.pointer = (x, y);
                self.pointer_visible = true;
            }
            Event::MouseDown { button } => self.pressed_button = Some(button),
            Event::MouseUp { button } => {
                if self.pressed_button == Some(button) {
                    self.pressed_button = None;
                }
            }
            Event::MouseWheel { delta } => {
                self.marker_radius = (self.marker_radius as f32 + delta * 2.0)
                    .round()
                    .clamp(4.0, 96.0) as u32;
            }
            Event::KeyDown { key } | Event::KeyUp { key } => self.accent = key_color(key),
            Event::TextInput { character } => {
                self.accent = Color::rgb(40 + (character as u32 % 180) as u8, 120, 200);
            }
        }
    }

    pub fn render(&self, framebuffer: &mut FrameBuffer) {
        framebuffer.checkerboard(24, Color::rgb(224, 224, 224), Color::rgb(176, 176, 176));

        let right = self.window_size.0.saturating_sub(1).min(i32::MAX as u32) as i32;
        let bottom = self.window_size.1.saturating_sub(1).min(i32::MAX as u32) as i32;
        let center_x = right / 2;
        let center_y = bottom / 2;

        framebuffer.draw_line(0, 0, right, bottom, self.accent);
        framebuffer.draw_line(right, 0, 0, bottom, Color::rgb(38, 139, 210));
        framebuffer.fill_rect(
            Rect::new(center_x - 160, center_y - 90, 320, 180),
            Color::rgba(108, 113, 196, 160),
        );
        framebuffer.draw_rect(
            Rect::new(center_x - 160, center_y - 90, 320, 180),
            Color::rgb(88, 90, 100),
        );
        framebuffer.fill_circle(center_x, center_y, 64, Color::rgba(133, 153, 0, 180));
        framebuffer.draw_circle(center_x, center_y, 64, Color::rgb(255, 255, 255));

        if self.pointer_visible {
            let marker = match self.pressed_button {
                Some(MouseButton::Left) => Color::rgba(220, 50, 47, 190),
                Some(MouseButton::Right) => Color::rgba(38, 139, 210, 190),
                Some(MouseButton::Middle) => Color::rgba(133, 153, 0, 190),
                None => Color::rgba(255, 255, 255, 150),
            };
            framebuffer.fill_circle(self.pointer.0, self.pointer.1, self.marker_radius, marker);
            framebuffer.draw_circle(
                self.pointer.0,
                self.pointer.1,
                self.marker_radius,
                self.accent,
            );
        }
    }
}

fn key_color(key: Key) -> Color {
    match key {
        Key::Letter(letter) => Color::rgb(letter as u8, 100, 180),
        Key::Digit(digit) => Color::rgb(40 + digit * 20, 120, 200),
        Key::Function(number) => Color::rgb(180, 40 + number * 6, 100),
        Key::Unknown(code) => Color::rgb(code as u8, 90, 160),
        _ => Color::rgb(203, 75, 22),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_update_application_state() {
        let mut app = App::new();
        app.handle_event(Event::Resized {
            width: 800,
            height: 600,
        });
        app.handle_event(Event::MouseMove { x: 120, y: 100 });
        app.handle_event(Event::MouseWheel { delta: 1.0 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });

        assert_eq!(app.window_size, (800, 600));
        assert_eq!(app.pointer, (120, 100));
        assert_eq!(app.marker_radius, 18);
        assert_eq!(app.pressed_button, Some(MouseButton::Left));

        app.handle_event(Event::CloseRequested);
        assert!(!app.running());
    }
}
