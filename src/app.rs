use crate::{
    graphics::{Color, FrameBuffer, Rect},
    platform::Event,
    ui::UiContext,
};

const NAME_INPUT: u32 = 1;
const CREATE_BUTTON: u32 = 2;

pub struct App {
    running: bool,
    window_size: (u32, u32),
    ui: UiContext,
    document_name: String,
    created: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            window_size: (0, 0),
            ui: UiContext::default(),
            document_name: String::from("UNTITLED"),
            created: false,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn handle_event(&mut self, event: Event) {
        self.ui.handle_event(&event);
        match event {
            Event::CloseRequested => self.running = false,
            Event::Resized { width, height } => self.window_size = (width, height),
            _ => {}
        }
    }

    pub fn render(&mut self, framebuffer: &mut FrameBuffer) {
        framebuffer.checkerboard(24, Color::rgb(224, 224, 224), Color::rgb(176, 176, 176));
        self.ui.begin_frame();

        let center_x = self.window_size.0.min(i32::MAX as u32) as i32 / 2;
        let center_y = self.window_size.1.min(i32::MAX as u32) as i32 / 2;
        let panel = Rect::new(center_x - 220, center_y - 150, 440, 300);
        self.ui.panel(framebuffer, panel);
        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 28, "OPENDRAW");
        let icon_x = panel.x + 384;
        let icon_y = panel.y + 42;
        framebuffer.fill_circle(icon_x, icon_y, 16, Color::rgb(72, 118, 180));
        framebuffer.draw_circle(icon_x, icon_y, 16, Color::rgb(225, 228, 232));
        framebuffer.draw_line(
            icon_x - 8,
            icon_y + 8,
            icon_x + 8,
            icon_y - 8,
            Color::rgb(225, 228, 232),
        );
        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 82, "DOCUMENT NAME");
        self.ui.text_input(
            framebuffer,
            NAME_INPUT,
            Rect::new(panel.x + 28, panel.y + 110, 384, 42),
            &mut self.document_name,
        );

        if self.ui.button(
            framebuffer,
            CREATE_BUTTON,
            Rect::new(panel.x + 252, panel.y + 205, 160, 48),
            "CREATE",
        ) {
            self.created = true;
        }

        self.ui.label(
            framebuffer,
            panel.x + 28,
            panel.y + 220,
            if self.created { "CREATED" } else { "READY" },
        );
        self.ui.end_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_event_stops_the_application() {
        let mut app = App::new();
        app.handle_event(Event::CloseRequested);
        assert!(!app.running());
    }
}
