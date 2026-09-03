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
const LAYER_OPACITY_SLIDER: u32 = 11;
const MOVE_DOWN_BUTTON: u32 = 13;
const MOVE_UP_BUTTON: u32 = 14;
const ADD_LAYER_BUTTON: u32 = 15;
const DELETE_LAYER_BUTTON: u32 = 16;
const TOOL_SIZE_SLIDER: u32 = 17;
const TOOL_OPACITY_SLIDER: u32 = 20;
const SELECT_BRUSH_BUTTON: u32 = 22;
const SELECT_ERASER_BUTTON: u32 = 23;
const SELECT_EYEDROPPER_BUTTON: u32 = 24;
const SELECT_BUCKET_BUTTON: u32 = 25;
const RED_SLIDER: u32 = 26;
const GREEN_SLIDER: u32 = 27;
const BLUE_SLIDER: u32 = 28;
const UNDO_BUTTON: u32 = 31;
const REDO_BUTTON: u32 = 32;
const COLOR_SQUARE: u32 = 33;
const HUE_SLIDER: u32 = 34;
const HISTORY_BYTE_LIMIT: u64 = 128 * 1024 * 1024;
const EDITOR_LEFT_WIDTH: u32 = 120;

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
    Bucket,
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
    picker_hue: u32,
    control_down: bool,
    undo_history: Vec<Document>,
    redo_history: Vec<Document>,
    layer_opacity_drag_recorded: bool,
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
            picker_hue: 0,
            control_down: false,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            layer_opacity_drag_recorded: false,
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
            Event::KeyDown { key: Key::Control } => self.control_down = true,
            Event::KeyUp { key: Key::Control } => self.control_down = false,
            Event::KeyDown {
                key: Key::Letter('Z'),
            } if self.control_down && self.state == AppState::Editor => self.undo(),
            Event::KeyDown {
                key: Key::Letter('Y'),
            } if self.control_down && self.state == AppState::Editor => self.redo(),
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
        let (tool_radius, tool_opacity) = match self.active_tool {
            ActiveTool::Brush => (self.brush.settings.radius, self.brush.settings.opacity),
            ActiveTool::Eraser => (self.eraser.settings.radius, self.eraser.settings.opacity),
            ActiveTool::Eyedropper | ActiveTool::Bucket => (0, self.brush.settings.opacity),
        };
        let tool_size = tool_radius * 2 + 1;
        let brush_color = self.brush.settings.color;
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
                EDITOR_LEFT_WIDTH as i32,
                window_height - 36,
                self.window_size.0.saturating_sub(300),
                36,
            ),
        );

        self.ui.label(framebuffer, 20, 20, "OPENDRAW");
        if self
            .ui
            .button(framebuffer, UNDO_BUTTON, Rect::new(140, 9, 96, 38), "UNDO")
        {
            self.undo();
        }
        if self
            .ui
            .button(framebuffer, REDO_BUTTON, Rect::new(244, 9, 96, 38), "REDO")
        {
            self.redo();
        }
        self.ui.label(framebuffer, 20, 66, "TOOLS");
        if self.ui.button(
            framebuffer,
            SELECT_BRUSH_BUTTON,
            Rect::new(8, 86, 104, 22),
            "BRUSH",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Brush;
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            SELECT_ERASER_BUTTON,
            Rect::new(8, 109, 104, 22),
            "ERASER",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Eraser;
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            SELECT_EYEDROPPER_BUTTON,
            Rect::new(8, 132, 104, 22),
            "PICKER",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Eyedropper;
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            SELECT_BUCKET_BUTTON,
            Rect::new(8, 155, 104, 22),
            "BUCKET",
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Bucket;
            self.rerender = true;
        }

        if matches!(self.active_tool, ActiveTool::Brush | ActiveTool::Eraser) {
            let preview = Rect::new(38, 185, 44, 44);
            let preview_radius = 1 + tool_radius * 20 / 127;
            framebuffer.fill_rect(preview, Color::rgb(22, 25, 30));
            framebuffer.fill_circle(
                60,
                207,
                preview_radius,
                match self.active_tool {
                    ActiveTool::Brush => Color::rgba(
                        brush_color.red(),
                        brush_color.green(),
                        brush_color.blue(),
                        tool_opacity,
                    ),
                    ActiveTool::Eraser => Color::rgb(205, 210, 218),
                    ActiveTool::Eyedropper | ActiveTool::Bucket => unreachable!(),
                },
            );
            framebuffer.draw_circle(60, 207, preview_radius, Color::rgb(255, 255, 255));
            framebuffer.draw_rect(preview, Color::rgb(120, 130, 145));

            self.ui.label(framebuffer, 20, 237, "SIZE");
            self.ui.label(framebuffer, 76, 237, &format!("{tool_size}"));
            let mut radius = tool_radius;
            if self
                .ui
                .slider(
                    framebuffer,
                    TOOL_SIZE_SLIDER,
                    Rect::new(8, 255, 104, 16),
                    &mut radius,
                    127,
                    Color::rgb(90, 155, 230),
                )
                .changed
            {
                match self.active_tool {
                    ActiveTool::Brush => self.brush.settings.radius = radius,
                    ActiveTool::Eraser => self.eraser.settings.radius = radius,
                    ActiveTool::Eyedropper | ActiveTool::Bucket => unreachable!(),
                }
                self.rerender = true;
            }
        }

        match self.active_tool {
            ActiveTool::Eyedropper => self.ui.label(framebuffer, 20, 185, "VISIBLE"),
            ActiveTool::Bucket => self.ui.label(framebuffer, 20, 185, "ACTIVE"),
            _ => {}
        }
        let alpha_y = 279;

        self.ui.label(
            framebuffer,
            20,
            alpha_y,
            &format!("A {}%", (u16::from(tool_opacity) * 100 + 127) / 255),
        );
        if self.active_tool != ActiveTool::Eyedropper {
            let mut opacity = u32::from(tool_opacity);
            if self
                .ui
                .slider(
                    framebuffer,
                    TOOL_OPACITY_SLIDER,
                    Rect::new(8, alpha_y + 18, 104, 16),
                    &mut opacity,
                    255,
                    Color::rgb(225, 228, 232),
                )
                .changed
            {
                match self.active_tool {
                    ActiveTool::Brush | ActiveTool::Bucket => {
                        self.brush.settings.opacity = opacity as u8
                    }
                    ActiveTool::Eraser => self.eraser.settings.opacity = opacity as u8,
                    ActiveTool::Eyedropper => unreachable!(),
                }
                self.rerender = true;
            }
        }
        self.render_color_picker(framebuffer);
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
            self.checkpoint();
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
        let mut opacity = u32::from(layer_opacity);
        let response = self.ui.slider(
            framebuffer,
            LAYER_OPACITY_SLIDER,
            Rect::new(controls_x, 378, 148, 20),
            &mut opacity,
            255,
            Color::rgb(225, 228, 232),
        );
        if response.started {
            self.layer_opacity_drag_recorded = false;
        }
        if response.changed && !self.layer_opacity_drag_recorded {
            self.checkpoint();
            self.layer_opacity_drag_recorded = response.dragging;
        }
        if response.changed {
            self.document.as_mut().unwrap().active_layer_mut().opacity = opacity as u8;
            self.layer_changed();
        }
        if !response.dragging {
            self.layer_opacity_drag_recorded = false;
        }

        self.ui.label(framebuffer, window_width - 160, 424, "ORDER");
        if self.ui.button(
            framebuffer,
            MOVE_DOWN_BUTTON,
            Rect::new(controls_x, 446, 72, 32),
            "DOWN",
        ) && layer_number > 1
        {
            self.checkpoint();
            self.document.as_mut().unwrap().move_active_down();
            self.layer_changed();
        }
        if self.ui.button(
            framebuffer,
            MOVE_UP_BUTTON,
            Rect::new(controls_x + 76, 446, 72, 32),
            "UP",
        ) && layer_number < layer_count
        {
            self.checkpoint();
            self.document.as_mut().unwrap().move_active_up();
            self.layer_changed();
        }

        if self.ui.button(
            framebuffer,
            ADD_LAYER_BUTTON,
            Rect::new(controls_x, 496, 72, 32),
            "ADD",
        ) {
            let snapshot = self.document.as_ref().unwrap().clone();
            let result = self.document.as_mut().unwrap().add_layer();
            self.editor_notice = match result {
                Ok(()) => {
                    self.remember(snapshot);
                    None
                }
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
            if layer_count > 1 {
                self.checkpoint();
                self.document.as_mut().unwrap().remove_active_layer();
                self.editor_notice = None;
            } else {
                self.editor_notice = Some("KEEP ONE LAYER");
            }
            self.rerender = true;
        }

        self.ui.label(
            framebuffer,
            EDITOR_LEFT_WIDTH as i32 + 16,
            window_height - 25,
            &format!("ZOOM {}%", (self.canvas_view.zoom * 100.0).round() as u32),
        );
        if let Some(notice) = self.editor_notice {
            self.ui.colored_label(
                framebuffer,
                EDITOR_LEFT_WIDTH as i32 + 172,
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

    fn render_color_picker(&mut self, framebuffer: &mut FrameBuffer) {
        self.ui.label(framebuffer, 20, 321, "COLOR");

        let (_, saturation, value) = self.brush.settings.color.to_hsv();
        let mut saturation = u32::from(saturation);
        let mut value = u32::from(value);
        let mut hue = self.picker_hue;
        let color_changed = self.ui.color_square(
            framebuffer,
            COLOR_SQUARE,
            Rect::new(8, 339, 104, 104),
            hue,
            &mut saturation,
            &mut value,
        );
        self.ui.label(framebuffer, 20, 451, &format!("H {hue}"));
        let hue_changed = self
            .ui
            .hue_slider(
                framebuffer,
                HUE_SLIDER,
                Rect::new(8, 469, 104, 16),
                &mut hue,
            )
            .changed;
        if color_changed || hue_changed {
            self.picker_hue = hue;
            self.brush.settings.color = Color::from_hsv(hue, saturation as u8, value as u8);
            self.rerender = true;
        }

        let mut red = u32::from(self.brush.settings.color.red());
        let mut green = u32::from(self.brush.settings.color.green());
        let mut blue = u32::from(self.brush.settings.color.blue());
        let alpha = self.brush.settings.opacity;
        let preview = Rect::new(20, 493, 80, 20);
        framebuffer.fill_rect(preview, Color::rgb(224, 224, 224));
        framebuffer.fill_rect(Rect::new(60, 493, 40, 10), Color::rgb(176, 176, 176));
        framebuffer.fill_rect(Rect::new(20, 503, 40, 10), Color::rgb(176, 176, 176));
        framebuffer.fill_rect(
            preview,
            Color::rgba(red as u8, green as u8, blue as u8, alpha),
        );
        framebuffer.draw_rect(preview, Color::rgb(225, 228, 232));
        self.ui.label(
            framebuffer,
            12,
            519,
            &format!("{red:02X}{green:02X}{blue:02X}{alpha:02X}"),
        );

        self.ui.label(framebuffer, 8, 540, &format!("R{red}"));
        let mut changed = self
            .ui
            .slider(
                framebuffer,
                RED_SLIDER,
                Rect::new(60, 538, 52, 16),
                &mut red,
                255,
                Color::rgb(210, 60, 60),
            )
            .changed;
        self.ui.label(framebuffer, 8, 560, &format!("G{green}"));
        changed |= self
            .ui
            .slider(
                framebuffer,
                GREEN_SLIDER,
                Rect::new(60, 558, 52, 16),
                &mut green,
                255,
                Color::rgb(55, 170, 90),
            )
            .changed;
        self.ui.label(framebuffer, 8, 580, &format!("B{blue}"));
        changed |= self
            .ui
            .slider(
                framebuffer,
                BLUE_SLIDER,
                Rect::new(60, 578, 52, 16),
                &mut blue,
                255,
                Color::rgb(60, 120, 220),
            )
            .changed;
        if changed {
            let color = Color::rgb(red as u8, green as u8, blue as u8);
            let (hue, saturation, _) = color.to_hsv();
            if saturation > 0 {
                self.picker_hue = hue;
            }
            self.brush.settings.color = color;
            self.rerender = true;
        }
    }

    fn checkpoint(&mut self) {
        let snapshot_bytes = Self::document_bytes(self.document.as_ref().unwrap());
        self.prepare_history(snapshot_bytes);
        self.undo_history
            .push(self.document.as_ref().unwrap().clone());
    }

    fn remember(&mut self, snapshot: Document) {
        let snapshot_bytes = Self::document_bytes(&snapshot);
        self.prepare_history(snapshot_bytes);
        self.undo_history.push(snapshot);
    }

    fn prepare_history(&mut self, snapshot_bytes: u64) {
        self.redo_history.clear();
        // ponytail: bounded snapshots; replace with diffs only when this depth is insufficient.
        while !self.undo_history.is_empty()
            && Self::history_bytes(&self.undo_history) + snapshot_bytes > HISTORY_BYTE_LIMIT
        {
            self.undo_history.remove(0);
        }
    }

    fn undo(&mut self) {
        let Some(previous) = self.undo_history.pop() else {
            return;
        };
        self.end_tool();
        let current = self.document.replace(previous).unwrap();
        self.redo_history.push(current);
        self.editor_notice = None;
        self.rerender = true;
    }

    fn redo(&mut self) {
        let Some(next) = self.redo_history.pop() else {
            return;
        };
        self.end_tool();
        let current = self.document.replace(next).unwrap();
        self.undo_history.push(current);
        self.editor_notice = None;
        self.rerender = true;
    }

    fn history_bytes(history: &[Document]) -> u64 {
        history.iter().map(Self::document_bytes).sum()
    }

    fn document_bytes(document: &Document) -> u64 {
        document
            .layers
            .iter()
            .map(|layer| layer.pixels.pixels.len() as u64 * std::mem::size_of::<u32>() as u64)
            .sum()
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
        self.undo_history.clear();
        self.redo_history.clear();
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
            let (hue, saturation, _) = self.brush.settings.color.to_hsv();
            if saturation > 0 {
                self.picker_hue = hue;
            }
            self.rerender = true;
            return;
        }
        if !self.document.as_ref().unwrap().active_layer().visible {
            return;
        }
        let fill_color = Color::rgba(
            self.brush.settings.color.red(),
            self.brush.settings.color.green(),
            self.brush.settings.color.blue(),
            self.brush.settings.opacity,
        );
        let changes_document = match self.active_tool {
            ActiveTool::Brush => self.brush.settings.opacity > 0,
            ActiveTool::Eraser => self.eraser.settings.opacity > 0,
            ActiveTool::Bucket => {
                self.document
                    .as_ref()
                    .unwrap()
                    .active_layer()
                    .pixels
                    .get_pixel(point.0, point.1)
                    != Some(fill_color)
            }
            ActiveTool::Eyedropper => false,
        };
        if !changes_document {
            return;
        }
        self.checkpoint();
        let pixels = &mut self.document.as_mut().unwrap().active_layer_mut().pixels;
        match self.active_tool {
            ActiveTool::Brush => self.brush.pointer_down(pixels, point),
            ActiveTool::Eraser => self.eraser.pointer_down(pixels, point),
            ActiveTool::Bucket => pixels.flood_fill(point.0, point.1, fill_color),
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
            ActiveTool::Eyedropper | ActiveTool::Bucket => {}
        }
    }

    fn tool_is_active(&self) -> bool {
        match self.active_tool {
            ActiveTool::Brush => self.brush.is_active(),
            ActiveTool::Eraser => self.eraser.is_active(),
            ActiveTool::Eyedropper | ActiveTool::Bucket => false,
        }
    }

    fn end_tool(&mut self) {
        match self.active_tool {
            ActiveTool::Brush => self.brush.pointer_up(),
            ActiveTool::Eraser => self.eraser.pointer_up(),
            ActiveTool::Eyedropper | ActiveTool::Bucket => {}
        }
    }

    fn break_tool_segment(&mut self) {
        match self.active_tool {
            ActiveTool::Brush => self.brush.break_segment(),
            ActiveTool::Eraser => self.eraser.break_segment(),
            ActiveTool::Eyedropper | ActiveTool::Bucket => {}
        }
    }

    fn editor_viewport(&self) -> Rect {
        Rect::new(
            EDITOR_LEFT_WIDTH as i32,
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

        let (x, y) = app.canvas_view.canvas_to_screen(0.5, 0.5);
        let (target_x, target_y) = app.canvas_view.canvas_to_screen(3.5, 2.5);
        app.handle_event(Event::MouseMove {
            x: x.round() as i32,
            y: y.round() as i32,
        });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.handle_event(Event::MouseMove {
            x: target_x.round() as i32,
            y: target_y.round() as i32,
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

    #[test]
    fn undo_and_redo_restore_snapshots_and_new_edits_clear_redo() {
        let white = Color::rgb(255, 255, 255);
        let black = Color::rgb(0, 0, 0);
        let red = Color::rgb(255, 0, 0);
        let mut app = App::new();
        app.state = AppState::Editor;
        app.document = Some(Document::new(3, 3, white).unwrap());
        app.checkpoint();
        app.document
            .as_mut()
            .unwrap()
            .active_layer_mut()
            .pixels
            .stamp_circle(1, 1, 0, black);

        app.handle_event(Event::KeyDown { key: Key::Control });
        app.handle_event(Event::KeyDown {
            key: Key::Letter('Z'),
        });
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .active_layer()
                .pixels
                .get_pixel(1, 1),
            Some(white)
        );
        app.handle_event(Event::KeyDown {
            key: Key::Letter('Y'),
        });
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .active_layer()
                .pixels
                .get_pixel(1, 1),
            Some(black)
        );

        app.handle_event(Event::KeyDown {
            key: Key::Letter('Z'),
        });
        app.checkpoint();
        app.document
            .as_mut()
            .unwrap()
            .active_layer_mut()
            .pixels
            .stamp_circle(1, 1, 0, red);
        app.handle_event(Event::KeyDown {
            key: Key::Letter('Y'),
        });
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .active_layer()
                .pixels
                .get_pixel(1, 1),
            Some(red)
        );
    }

    #[test]
    fn layer_opacity_drag_is_one_undoable_change() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        app.document = Some(Document::new(4, 4, Color::rgb(255, 255, 255)).unwrap());
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700);

        app.handle_event(Event::MouseMove { x: 900, y: 388 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        app.handle_event(Event::MouseMove { x: 850, y: 388 });
        app.render(&mut framebuffer);
        app.handle_event(Event::MouseUp {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);

        assert_eq!(app.undo_history.len(), 1);
        assert!(app.document.as_ref().unwrap().active_layer().opacity < 255);
        app.undo();
        assert_eq!(app.document.as_ref().unwrap().active_layer().opacity, 255);
    }

    #[test]
    fn persistent_color_picker_applies_click_immediately() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        app.document = Some(Document::new(4, 4, Color::rgb(255, 255, 255)).unwrap());
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700);

        app.handle_event(Event::MouseMove { x: 111, y: 339 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);

        assert_eq!(app.brush.settings.color, Color::rgb(255, 0, 0));
        assert!(app.undo_history.is_empty());
    }
}
