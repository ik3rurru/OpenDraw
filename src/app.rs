use crate::{
    document::{CanvasView, Document, DocumentError},
    graphics::{Color, FrameBuffer, Rect},
    platform::{Event, Key, MouseButton},
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
const PREVIOUS_LAYER_BUTTON: u32 = 8;
const NEXT_LAYER_BUTTON: u32 = 9;
const VISIBILITY_BUTTON: u32 = 10;
const OPACITY_DOWN_BUTTON: u32 = 11;
const OPACITY_UP_BUTTON: u32 = 12;
const MOVE_DOWN_BUTTON: u32 = 13;
const MOVE_UP_BUTTON: u32 = 14;
const ADD_LAYER_BUTTON: u32 = 15;
const DELETE_LAYER_BUTTON: u32 = 16;
const PENCIL_COLOR: Color = Color::rgb(24, 24, 24);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Background {
    Transparent,
    White,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppState {
    NewDocument,
    Editor,
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
    document: Option<Document>,
    canvas_view: CanvasView,
    pointer: (i32, i32),
    panning: bool,
    drawing: bool,
    last_draw_point: Option<(u32, u32)>,
    editor_notice: Option<&'static str>,
    rerender: bool,
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
            document: None,
            canvas_view: CanvasView::default(),
            pointer: (0, 0),
            panning: false,
            drawing: false,
            last_draw_point: None,
            editor_notice: None,
            rerender: false,
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
            Event::MouseMove { x, y } => {
                if self.panning {
                    self.canvas_view.pan(x - self.pointer.0, y - self.pointer.1);
                }
                self.pointer = (x, y);
                if self.drawing {
                    self.draw_to(x, y);
                }
            }
            Event::MouseDown {
                button: MouseButton::Middle,
            } if self.state == AppState::Editor
                && self
                    .editor_viewport()
                    .contains(self.pointer.0, self.pointer.1) =>
            {
                self.panning = true;
            }
            Event::MouseDown {
                button: MouseButton::Left,
            } if self.state == AppState::Editor
                && self
                    .canvas_pixel_at(self.pointer.0, self.pointer.1)
                    .is_some() =>
            {
                self.drawing = true;
                self.last_draw_point = None;
                self.draw_to(self.pointer.0, self.pointer.1);
            }
            Event::MouseUp {
                button: MouseButton::Middle,
            } => self.panning = false,
            Event::MouseUp {
                button: MouseButton::Left,
            } => {
                self.drawing = false;
                self.last_draw_point = None;
            }
            Event::MouseWheel { delta }
                if self.state == AppState::Editor
                    && self
                        .editor_viewport()
                        .contains(self.pointer.0, self.pointer.1) =>
            {
                self.canvas_view
                    .zoom_at(1.1_f32.powf(delta), self.pointer.0, self.pointer.1);
            }
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
        self.rerender = false;
        let previous_state = self.state;
        self.render_current_state(framebuffer);
        if (self.state != previous_state || self.rerender) && self.running {
            self.rerender = false;
            self.render_current_state(framebuffer);
        }
    }

    fn render_current_state(&mut self, framebuffer: &mut FrameBuffer) {
        self.ui.begin_frame();

        match self.state {
            AppState::NewDocument => {
                framebuffer.checkerboard(24, Color::rgb(224, 224, 224), Color::rgb(176, 176, 176));
                self.render_new_document(framebuffer);
            }
            AppState::Editor => self.render_editor(framebuffer),
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
            self.rerender = true;
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
            self.rerender = true;
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

    fn render_editor(&mut self, framebuffer: &mut FrameBuffer) {
        let viewport = self.editor_viewport();
        let window_width = self.window_size.0.min(i32::MAX as u32) as i32;
        let window_height = self.window_size.1.min(i32::MAX as u32) as i32;

        framebuffer.clear(Color::rgb(27, 30, 35));
        framebuffer.fill_rect(viewport, Color::rgb(45, 49, 56));
        let document = self.document.as_ref().expect("editor needs a document");
        self.canvas_view.render(document, framebuffer, viewport);
        let document_size = (document.width, document.height);
        let layer_name = document.active_layer().name.clone();
        let layer_number = document.active_layer + 1;
        let layer_count = document.layers.len();
        let layer_visible = document.active_layer().visible;
        let layer_opacity = document.active_layer().opacity;
        framebuffer.draw_rect(viewport, Color::rgb(80, 86, 96));

        self.ui
            .panel(framebuffer, Rect::new(0, 0, self.window_size.0, 56));
        self.ui.panel(
            framebuffer,
            Rect::new(0, 56, 120, self.window_size.1.saturating_sub(56)),
        );
        self.ui.panel(
            framebuffer,
            Rect::new(
                window_width - 180,
                56,
                180,
                self.window_size.1.saturating_sub(56),
            ),
        );
        self.ui.panel(
            framebuffer,
            Rect::new(
                120,
                window_height - 36,
                self.window_size.0.saturating_sub(300),
                36,
            ),
        );

        self.ui.label(framebuffer, 20, 20, "OPENDRAW");
        self.ui.label(framebuffer, 20, 82, "TOOLS");
        self.ui.label(framebuffer, 20, 116, "PENCIL");
        self.ui.label(framebuffer, 20, 140, "LEFT");
        self.ui.label(framebuffer, 20, 184, "PAN");
        self.ui.label(framebuffer, 20, 208, "MIDDLE");
        self.ui
            .label(framebuffer, window_width - 160, 82, "DOCUMENT");
        self.ui.label(
            framebuffer,
            window_width - 160,
            116,
            &format!("{} X {}", document_size.0, document_size.1),
        );
        self.ui
            .label(framebuffer, window_width - 160, 160, "LAYERS");
        self.ui
            .label(framebuffer, window_width - 160, 194, &layer_name);
        self.ui.label(
            framebuffer,
            window_width - 160,
            218,
            &format!("{} / {}", layer_number, layer_count),
        );

        let controls_x = window_width - 164;
        if self.ui.button(
            framebuffer,
            PREVIOUS_LAYER_BUTTON,
            Rect::new(controls_x, 242, 72, 32),
            "PREV",
        ) {
            let document = self.document.as_mut().unwrap();
            document.active_layer = document.active_layer.saturating_sub(1);
            self.layer_changed();
        }
        if self.ui.button(
            framebuffer,
            NEXT_LAYER_BUTTON,
            Rect::new(controls_x + 76, 242, 72, 32),
            "NEXT",
        ) {
            let document = self.document.as_mut().unwrap();
            document.active_layer = (document.active_layer + 1).min(document.layers.len() - 1);
            self.layer_changed();
        }

        self.ui.label(
            framebuffer,
            window_width - 160,
            288,
            if layer_visible { "VISIBLE" } else { "HIDDEN" },
        );
        if self.ui.button(
            framebuffer,
            VISIBILITY_BUTTON,
            Rect::new(controls_x, 310, 148, 32),
            if layer_visible { "HIDE" } else { "SHOW" },
        ) {
            let layer = self.document.as_mut().unwrap().active_layer_mut();
            layer.visible = !layer.visible;
            self.layer_changed();
        }

        self.ui.label(
            framebuffer,
            window_width - 160,
            356,
            &format!("OPACITY {}%", (u16::from(layer_opacity) * 100 / 255)),
        );
        if self.ui.button(
            framebuffer,
            OPACITY_DOWN_BUTTON,
            Rect::new(controls_x, 378, 72, 32),
            "LESS",
        ) {
            let opacity = &mut self.document.as_mut().unwrap().active_layer_mut().opacity;
            *opacity = opacity.saturating_sub(32);
            self.layer_changed();
        }
        if self.ui.button(
            framebuffer,
            OPACITY_UP_BUTTON,
            Rect::new(controls_x + 76, 378, 72, 32),
            "MORE",
        ) {
            let opacity = &mut self.document.as_mut().unwrap().active_layer_mut().opacity;
            *opacity = opacity.saturating_add(32);
            self.layer_changed();
        }

        self.ui.label(framebuffer, window_width - 160, 424, "ORDER");
        if self.ui.button(
            framebuffer,
            MOVE_DOWN_BUTTON,
            Rect::new(controls_x, 446, 72, 32),
            "DOWN",
        ) {
            self.document.as_mut().unwrap().move_active_down();
            self.layer_changed();
        }
        if self.ui.button(
            framebuffer,
            MOVE_UP_BUTTON,
            Rect::new(controls_x + 76, 446, 72, 32),
            "UP",
        ) {
            self.document.as_mut().unwrap().move_active_up();
            self.layer_changed();
        }

        if self.ui.button(
            framebuffer,
            ADD_LAYER_BUTTON,
            Rect::new(controls_x, 496, 72, 32),
            "ADD",
        ) {
            self.editor_notice = match self.document.as_mut().unwrap().add_layer() {
                Ok(()) => None,
                Err(DocumentError::AllocationFailed) => Some("NOT ENOUGH MEMORY"),
                Err(_) => Some("LAYER LIMIT REACHED"),
            };
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            DELETE_LAYER_BUTTON,
            Rect::new(controls_x + 76, 496, 72, 32),
            "DELETE",
        ) {
            self.editor_notice = (!self.document.as_mut().unwrap().remove_active_layer())
                .then_some("KEEP ONE LAYER");
            self.rerender = true;
        }

        self.ui.label(
            framebuffer,
            136,
            window_height - 25,
            &format!("ZOOM {}%", (self.canvas_view.zoom * 100.0).round() as u32),
        );
        if let Some(notice) = self.editor_notice {
            self.ui.colored_label(
                framebuffer,
                300,
                window_height - 25,
                notice,
                Color::rgb(230, 90, 80),
            );
        }

        if self.ui.button(
            framebuffer,
            NEW_DOCUMENT_BUTTON,
            Rect::new(window_width - 168, 9, 156, 38),
            "NEW DOC",
        ) {
            self.state = AppState::NewDocument;
            self.panning = false;
            self.drawing = false;
            self.last_draw_point = None;
            self.editor_notice = None;
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

        let background = match self.background {
            Background::Transparent => Color::rgba(0, 0, 0, 0),
            Background::White => Color::rgb(255, 255, 255),
        };
        let document = match Document::new(width, height, background) {
            Ok(document) => document,
            Err(DocumentError::TooLarge) => {
                self.validation_error = Some("MAX 64 MILLION PIXELS");
                return;
            }
            Err(DocumentError::AllocationFailed) => {
                self.validation_error = Some("NOT ENOUGH MEMORY");
                return;
            }
            Err(DocumentError::InvalidDimensions) => {
                self.validation_error = Some("SIZE MUST BE ABOVE 0");
                return;
            }
        };

        self.canvas_view = CanvasView::fit(&document, self.editor_viewport());
        self.document = Some(document);
        self.validation_error = None;
        self.state = AppState::Editor;
        self.panning = false;
        self.drawing = false;
        self.last_draw_point = None;
        self.editor_notice = None;
        self.ui.clear_focus();
    }

    fn layer_changed(&mut self) {
        self.editor_notice = None;
        self.rerender = true;
        self.drawing = false;
        self.last_draw_point = None;
    }

    fn canvas_pixel_at(&self, screen_x: i32, screen_y: i32) -> Option<(u32, u32)> {
        if !self.editor_viewport().contains(screen_x, screen_y) {
            return None;
        }
        let document = self.document.as_ref()?;
        let (x, y) = self
            .canvas_view
            .screen_to_canvas(screen_x as f32, screen_y as f32);
        (x >= 0.0 && y >= 0.0 && x < document.width as f32 && y < document.height as f32)
            .then_some((x as u32, y as u32))
    }

    fn draw_to(&mut self, screen_x: i32, screen_y: i32) {
        let Some(point) = self.canvas_pixel_at(screen_x, screen_y) else {
            self.last_draw_point = None;
            return;
        };
        let previous = self.last_draw_point.unwrap_or(point);
        if !self.document.as_ref().unwrap().active_layer().visible {
            self.last_draw_point = None;
            return;
        }
        self.document
            .as_mut()
            .expect("drawing needs a document")
            .active_layer_mut()
            .pixels
            .draw_line(previous.0, previous.1, point.0, point.1, PENCIL_COLOR);
        self.last_draw_point = Some(point);
    }

    fn editor_viewport(&self) -> Rect {
        Rect::new(
            120,
            56,
            self.window_size.0.saturating_sub(300),
            self.window_size.1.saturating_sub(92),
        )
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
        app.window_size = (1000, 700);
        app.create_document();
        assert_eq!(app.state, AppState::Editor);
        let document = app.document.as_ref().unwrap();
        assert_eq!((document.width, document.height), (640, 480));
        assert_eq!(
            document.active_layer().pixels.get_pixel(0, 0),
            Some(Color::rgb(255, 255, 255))
        );

        let (x, y) = app.canvas_view.canvas_to_screen(0.0, 0.0);
        app.handle_event(Event::MouseMove {
            x: x as i32,
            y: y as i32,
        });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.handle_event(Event::MouseMove {
            x: x as i32 + 3,
            y: y as i32 + 2,
        });
        app.handle_event(Event::MouseUp {
            button: MouseButton::Left,
        });
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .active_layer()
                .pixels
                .get_pixel(3, 2),
            Some(PENCIL_COLOR)
        );
    }
}
