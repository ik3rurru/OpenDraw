use crate::{
    graphics::{Color, FrameBuffer, Rect},
    platform::{Event, Key},
    ui::UiContext,
};

const MAX_DOCUMENT_SIZE: u32 = 16_384;
const WIDTH_INPUT: u32 = 1;
const HEIGHT_INPUT: u32 = 2;
const TRANSPARENT_RADIO: u32 = 3;
const WHITE_RADIO: u32 = 4;
const CANCEL_BUTTON: u32 = 5;
const CREATE_BUTTON: u32 = 6;
const NEW_DOCUMENT_BUTTON: u32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Background {
    Transparent,
    White,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppState {
    NewDocument,
    Editor {
        width: u32,
        height: u32,
        background: Background,
    },
}

pub struct App {
    running: bool,
    window_size: (u32, u32),
    ui: UiContext,
    state: AppState,
    width_input: String,
    height_input: String,
    background: Background,
    validation_error: Option<&'static str>,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            window_size: (0, 0),
            ui: UiContext::default(),
            state: AppState::NewDocument,
            width_input: String::from("1920"),
            height_input: String::from("1080"),
            background: Background::White,
            validation_error: None,
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
            Event::TextInput { character } if !character.is_control() => {
                self.validation_error = None
            }
            Event::KeyDown {
                key: Key::Backspace,
            } => self.validation_error = None,
            _ => {}
        }
    }

    pub fn render(&mut self, framebuffer: &mut FrameBuffer) {
        let previous_state = self.state;
        self.render_current_state(framebuffer);
        if self.state != previous_state && self.running {
            self.render_current_state(framebuffer);
        }
    }

    fn render_current_state(&mut self, framebuffer: &mut FrameBuffer) {
        framebuffer.checkerboard(24, Color::rgb(224, 224, 224), Color::rgb(176, 176, 176));
        self.ui.begin_frame();

        match self.state {
            AppState::NewDocument => self.render_new_document(framebuffer),
            AppState::Editor {
                width,
                height,
                background,
            } => self.render_editor(framebuffer, width, height, background),
        }

        self.ui.end_frame();
    }

    fn render_new_document(&mut self, framebuffer: &mut FrameBuffer) {
        let (center_x, center_y) = self.window_center();
        let panel = Rect::new(center_x - 280, center_y - 215, 560, 430);
        self.ui.panel(framebuffer, panel);
        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 26, "NEW DOCUMENT");
        framebuffer.draw_line(
            panel.x + 486,
            panel.y + 46,
            panel.x + 518,
            panel.y + 22,
            Color::rgb(90, 155, 230),
        );

        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 86, "WIDTH");
        self.ui.text_input(
            framebuffer,
            WIDTH_INPUT,
            Rect::new(panel.x + 250, panel.y + 70, 220, 42),
            &mut self.width_input,
        );
        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 140, "HEIGHT");
        self.ui.text_input(
            framebuffer,
            HEIGHT_INPUT,
            Rect::new(panel.x + 250, panel.y + 124, 220, 42),
            &mut self.height_input,
        );

        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 198, "BACKGROUND");
        if self.ui.radio_button(
            framebuffer,
            TRANSPARENT_RADIO,
            panel.x + 44,
            panel.y + 238,
            "TRANSPARENT",
            self.background == Background::Transparent,
        ) {
            self.background = Background::Transparent;
        }
        if self.ui.radio_button(
            framebuffer,
            WHITE_RADIO,
            panel.x + 44,
            panel.y + 274,
            "WHITE",
            self.background == Background::White,
        ) {
            self.background = Background::White;
        }

        if let Some(error) = self.validation_error {
            self.ui.colored_label(
                framebuffer,
                panel.x + 28,
                panel.y + 310,
                error,
                Color::rgb(230, 90, 80),
            );
        }

        if self.ui.button(
            framebuffer,
            CANCEL_BUTTON,
            Rect::new(panel.x + 188, panel.y + 356, 150, 48),
            "CANCEL",
        ) {
            self.running = false;
        }
        if self.ui.button(
            framebuffer,
            CREATE_BUTTON,
            Rect::new(panel.x + 354, panel.y + 356, 150, 48),
            "CREATE",
        ) {
            self.create_document();
        }
    }

    fn render_editor(
        &mut self,
        framebuffer: &mut FrameBuffer,
        width: u32,
        height: u32,
        background: Background,
    ) {
        let (center_x, center_y) = self.window_center();
        let panel = Rect::new(center_x - 300, center_y - 190, 600, 380);
        self.ui.panel(framebuffer, panel);
        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 26, "EDITOR");

        let preview = Rect::new(panel.x + 28, panel.y + 70, 544, 200);
        framebuffer.fill_rect(
            preview,
            match background {
                Background::Transparent => Color::rgb(176, 176, 176),
                Background::White => Color::rgb(255, 255, 255),
            },
        );
        framebuffer.draw_rect(preview, Color::rgb(90, 155, 230));

        let dimensions = format!("{width} X {height}");
        self.ui
            .label(framebuffer, panel.x + 28, panel.y + 294, &dimensions);
        self.ui.label(
            framebuffer,
            panel.x + 28,
            panel.y + 330,
            match background {
                Background::Transparent => "TRANSPARENT",
                Background::White => "WHITE BACKGROUND",
            },
        );

        if self.ui.button(
            framebuffer,
            NEW_DOCUMENT_BUTTON,
            Rect::new(panel.x + 372, panel.y + 302, 200, 48),
            "NEW DOCUMENT",
        ) {
            self.state = AppState::NewDocument;
            self.ui.clear_focus();
        }
    }

    fn create_document(&mut self) {
        let Ok(width) = self.width_input.parse::<u32>() else {
            self.validation_error = Some("ENTER NUMERIC WIDTH");
            return;
        };
        let Ok(height) = self.height_input.parse::<u32>() else {
            self.validation_error = Some("ENTER NUMERIC HEIGHT");
            return;
        };
        if width == 0 || height == 0 {
            self.validation_error = Some("SIZE MUST BE ABOVE 0");
            return;
        }
        if width > MAX_DOCUMENT_SIZE || height > MAX_DOCUMENT_SIZE {
            self.validation_error = Some("MAX SIZE IS 16384");
            return;
        }

        self.validation_error = None;
        self.state = AppState::Editor {
            width,
            height,
            background: self.background,
        };
        self.ui.clear_focus();
    }

    fn window_center(&self) -> (i32, i32) {
        (
            self.window_size.0.min(i32::MAX as u32) as i32 / 2,
            self.window_size.1.min(i32::MAX as u32) as i32 / 2,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_dimensions_before_entering_the_editor() {
        let mut app = App::new();
        app.width_input = String::from("0");
        app.create_document();
        assert_eq!(app.state, AppState::NewDocument);
        assert_eq!(app.validation_error, Some("SIZE MUST BE ABOVE 0"));
        app.handle_event(Event::TextInput { character: '\u{8}' });
        assert_eq!(app.validation_error, Some("SIZE MUST BE ABOVE 0"));
        app.handle_event(Event::TextInput { character: '1' });
        assert_eq!(app.validation_error, None);

        app.width_input = String::from("640");
        app.height_input = String::from("480");
        app.create_document();
        assert_eq!(
            app.state,
            AppState::Editor {
                width: 640,
                height: 480,
                background: Background::White,
            }
        );
    }
}
