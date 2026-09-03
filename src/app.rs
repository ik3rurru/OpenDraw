use crate::{
    document::{CanvasView, Document, DocumentError},
    graphics::{Color, FrameBuffer, Rect},
    platform::{Event, Key, MouseButton},
    tools::{BrushTool, EraserTool, Tool},
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
const BRUSH_SIZE_DOWN_BUTTON: u32 = 17;
const BRUSH_SIZE_UP_BUTTON: u32 = 18;
const BRUSH_COLOR_BUTTON: u32 = 19;
const BRUSH_ALPHA_DOWN_BUTTON: u32 = 20;
const BRUSH_ALPHA_UP_BUTTON: u32 = 21;
const SELECT_BRUSH_BUTTON: u32 = 22;
const SELECT_ERASER_BUTTON: u32 = 23;
const SELECT_EYEDROPPER_BUTTON: u32 = 24;
const BRUSH_COLORS: [Color; 4] = [
    Color::rgb(24, 24, 24),
    Color::rgb(210, 60, 60),
    Color::rgb(55, 170, 90),
    Color::rgb(60, 120, 220),
];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveTool {
    Brush,
    Eraser,
    Eyedropper,
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
    brush: BrushTool,
    eraser: EraserTool,
    active_tool: ActiveTool,
    brush_color: usize,
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
            brush: BrushTool::default(),
            eraser: EraserTool::default(),
            active_tool: ActiveTool::Brush,
            brush_color: 0,
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
                if self.tool_is_active() {
                    self.move_tool(x, y);
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
            } if self.state == AppState::Editor => {
                self.begin_tool(self.pointer.0, self.pointer.1);
            }
            Event::MouseUp {
                button: MouseButton::Middle,
            } => self.panning = false,
            Event::MouseUp {
                button: MouseButton::Left,
            } => self.end_tool(),
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
        let (tool_name, tool_radius, tool_opacity) = match self.active_tool {
            ActiveTool::Brush => (
                "BRUSH",
                self.brush.settings.radius,
                self.brush.settings.opacity,
            ),
            ActiveTool::Eraser => (
                "ERASER",
                self.eraser.settings.radius,
                self.eraser.settings.opacity,
            ),
            ActiveTool::Eyedropper => ("EYEDROPPER", 0, self.brush.settings.opacity),
        };
        let tool_size = tool_radius * 2 + 1;
        let brush_color = self.brush.settings.color;
        let brush_color_hex = format!(
            "{:02X}{:02X}{:02X}",
            brush_color.red(),
            brush_color.green(),
            brush_color.blue()
        );
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
        if self.ui.button(
            framebuffer,
            SELECT_BRUSH_BUTTON,
            Rect::new(8, 108, 104, 24),
            "BRUSH",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Brush;
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            SELECT_ERASER_BUTTON,
            Rect::new(8, 134, 104, 24),
            "ERASER",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Eraser;
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            SELECT_EYEDROPPER_BUTTON,
            Rect::new(8, 160, 104, 24),
            "PICKER",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Eyedropper;
            self.rerender = true;
        }

        if self.active_tool != ActiveTool::Eyedropper {
            self.ui
                .label(framebuffer, 20, 192, &format!("{tool_name} SIZE"));
            self.ui.label(framebuffer, 20, 216, &format!("{tool_size}"));
            if self.ui.button(
                framebuffer,
                BRUSH_SIZE_DOWN_BUTTON,
                Rect::new(8, 238, 50, 32),
                "LESS",
            ) {
                match self.active_tool {
                    ActiveTool::Brush => {
                        self.brush.settings.radius = self.brush.settings.radius.saturating_sub(1)
                    }
                    ActiveTool::Eraser => {
                        self.eraser.settings.radius = self.eraser.settings.radius.saturating_sub(1)
                    }
                    ActiveTool::Eyedropper => {}
                }
                self.rerender = true;
            }
            if self.ui.button(
                framebuffer,
                BRUSH_SIZE_UP_BUTTON,
                Rect::new(62, 238, 50, 32),
                "MORE",
            ) {
                match self.active_tool {
                    ActiveTool::Brush => {
                        self.brush.settings.radius = (self.brush.settings.radius + 1).min(127)
                    }
                    ActiveTool::Eraser => {
                        self.eraser.settings.radius = (self.eraser.settings.radius + 1).min(127)
                    }
                    ActiveTool::Eyedropper => {}
                }
                self.rerender = true;
            }
        }

        let alpha_y = match self.active_tool {
            ActiveTool::Brush => {
                self.ui.label(framebuffer, 20, 286, "COLOR");
                framebuffer.fill_rect(Rect::new(88, 282, 20, 20), brush_color);
                framebuffer.draw_rect(Rect::new(88, 282, 20, 20), Color::rgb(225, 228, 232));
                self.ui.label(framebuffer, 20, 310, &brush_color_hex);
                if self.ui.button(
                    framebuffer,
                    BRUSH_COLOR_BUTTON,
                    Rect::new(8, 332, 104, 32),
                    "NEXT",
                ) {
                    self.brush_color = (self.brush_color + 1) % BRUSH_COLORS.len();
                    self.brush.settings.color = BRUSH_COLORS[self.brush_color];
                    self.rerender = true;
                }
                380
            }
            ActiveTool::Eraser => 286,
            ActiveTool::Eyedropper => {
                self.ui.label(framebuffer, 20, 192, "VISIBLE");
                self.ui.label(framebuffer, 20, 238, "COLOR");
                framebuffer.fill_rect(Rect::new(88, 234, 20, 20), brush_color);
                framebuffer.draw_rect(Rect::new(88, 234, 20, 20), Color::rgb(225, 228, 232));
                self.ui.label(framebuffer, 20, 262, &brush_color_hex);
                302
            }
        };

        self.ui.label(framebuffer, 20, alpha_y, "ALPHA");
        self.ui.label(
            framebuffer,
            20,
            alpha_y + 24,
            &format!("{}%", (u16::from(tool_opacity) * 100 + 127) / 255),
        );
        if self.active_tool != ActiveTool::Eyedropper {
            if self.ui.button(
                framebuffer,
                BRUSH_ALPHA_DOWN_BUTTON,
                Rect::new(8, alpha_y + 46, 50, 32),
                "LESS",
            ) {
                match self.active_tool {
                    ActiveTool::Brush => {
                        self.brush.settings.opacity = self.brush.settings.opacity.saturating_sub(32)
                    }
                    ActiveTool::Eraser => {
                        self.eraser.settings.opacity =
                            self.eraser.settings.opacity.saturating_sub(32)
                    }
                    ActiveTool::Eyedropper => {}
                }
                self.rerender = true;
            }
            if self.ui.button(
                framebuffer,
                BRUSH_ALPHA_UP_BUTTON,
                Rect::new(62, alpha_y + 46, 50, 32),
                "MORE",
            ) {
                match self.active_tool {
                    ActiveTool::Brush => {
                        self.brush.settings.opacity = self.brush.settings.opacity.saturating_add(32)
                    }
                    ActiveTool::Eraser => {
                        self.eraser.settings.opacity =
                            self.eraser.settings.opacity.saturating_add(32)
                    }
                    ActiveTool::Eyedropper => {}
                }
                self.rerender = true;
            }
        }

        self.ui.label(framebuffer, 20, 484, "PAN");
        self.ui.label(framebuffer, 20, 508, "MIDDLE");
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
            self.end_tool();
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
        self.end_tool();
        self.editor_notice = None;
        self.ui.clear_focus();
    }

    fn layer_changed(&mut self) {
        self.editor_notice = None;
        self.rerender = true;
        self.end_tool();
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

    fn begin_tool(&mut self, screen_x: i32, screen_y: i32) {
        let Some(point) = self.canvas_pixel_at(screen_x, screen_y) else {
            return;
        };
        if self.active_tool == ActiveTool::Eyedropper {
            let color = self
                .document
                .as_ref()
                .unwrap()
                .composite_pixel(point.0, point.1, Color::rgba(0, 0, 0, 0))
                .unwrap();
            self.brush.settings.color = Color::rgb(color.red(), color.green(), color.blue());
            self.brush.settings.opacity = color.alpha();
            self.rerender = true;
            return;
        }
        if !self.document.as_ref().unwrap().active_layer().visible {
            return;
        }
        let pixels = &mut self.document.as_mut().unwrap().active_layer_mut().pixels;
        match self.active_tool {
            ActiveTool::Brush => self.brush.pointer_down(pixels, point),
            ActiveTool::Eraser => self.eraser.pointer_down(pixels, point),
            ActiveTool::Eyedropper => {}
        }
    }

    fn move_tool(&mut self, screen_x: i32, screen_y: i32) {
        let Some(point) = self.canvas_pixel_at(screen_x, screen_y) else {
            self.break_tool_segment();
            return;
        };
        if !self.document.as_ref().unwrap().active_layer().visible {
            self.break_tool_segment();
            return;
        }
        let pixels = &mut self.document.as_mut().unwrap().active_layer_mut().pixels;
        match self.active_tool {
            ActiveTool::Brush => self.brush.pointer_move(pixels, point),
            ActiveTool::Eraser => self.eraser.pointer_move(pixels, point),
            ActiveTool::Eyedropper => {}
        }
    }

    fn tool_is_active(&self) -> bool {
        match self.active_tool {
            ActiveTool::Brush => self.brush.is_active(),
            ActiveTool::Eraser => self.eraser.is_active(),
            ActiveTool::Eyedropper => false,
        }
    }

    fn end_tool(&mut self) {
        match self.active_tool {
            ActiveTool::Brush => self.brush.pointer_up(),
            ActiveTool::Eraser => self.eraser.pointer_up(),
            ActiveTool::Eyedropper => {}
        }
    }

    fn break_tool_segment(&mut self) {
        match self.active_tool {
            ActiveTool::Brush => self.brush.break_segment(),
            ActiveTool::Eraser => self.eraser.break_segment(),
            ActiveTool::Eyedropper => {}
        }
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
            Some(app.brush.settings.color)
        );
    }

    #[test]
    fn eyedropper_samples_the_visible_composition_into_the_brush_color() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        let mut document = Document::new(4, 4, Color::rgba(0, 0, 255, 128)).unwrap();
        document.add_layer().unwrap();
        document.active_layer_mut().opacity = 128;
        document
            .active_layer_mut()
            .pixels
            .stamp_circle(1, 1, 0, Color::rgb(255, 0, 0));
        let expected = document
            .composite_pixel(1, 1, Color::rgba(0, 0, 0, 0))
            .unwrap();
        app.canvas_view = CanvasView::fit(&document, app.editor_viewport());
        app.document = Some(document);
        app.state = AppState::Editor;
        app.active_tool = ActiveTool::Eyedropper;

        let (x, y) = app.canvas_view.canvas_to_screen(1.0, 1.0);
        app.begin_tool(x as i32, y as i32);

        assert_eq!(
            app.brush.settings.color,
            Color::rgb(expected.red(), expected.green(), expected.blue())
        );
        assert_eq!(app.brush.settings.opacity, expected.alpha());
    }
}
