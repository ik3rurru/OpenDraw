use std::path::{Path, PathBuf};

use crate::{
    document::{CanvasView, Document, DocumentError, Layer},
    file::{self, ImageFormat, OdrawError},
    graphics::{Color, FrameBuffer, Rect},
    platform::{DecodedImage, Event, Key, MouseButton},
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
const LAYER_OPACITY_SLIDER: u32 = 11;
const LAYER_NAME_INPUT: u32 = 12;
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
const OPEN_DOCUMENT_BUTTON: u32 = 35;
const SAVE_DOCUMENT_BUTTON: u32 = 36;
const EXPORT_IMAGE_BUTTON: u32 = 37;
const IMPORT_IMAGE_BUTTON: u32 = 38;
const LAYER_ROW_BASE: u32 = 1_000;
const LAYER_VISIBILITY_BASE: u32 = 2_000;
const HISTORY_BYTE_LIMIT: u64 = 128 * 1024 * 1024;
const EDITOR_LEFT_WIDTH: u32 = 120;
const EDITOR_RIGHT_WIDTH: u32 = 220;
const LAYER_ROW_HEIGHT: u32 = 58;

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

enum EditorIcon {
    Pencil,
    Eraser,
    Eyedropper,
    Bucket,
    ArrowLeft,
    ArrowRight,
    Add,
    Trash,
    Folder,
    Save,
    Import,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    New,
    Open,
    Save,
    Import,
    Export,
    Exit,
}

struct Snapshot {
    document: Document,
    revision: u64,
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
    undo_history: Vec<Snapshot>,
    redo_history: Vec<Snapshot>,
    current_revision: u64,
    saved_revision: Option<u64>,
    next_revision: u64,
    layer_scroll: usize,
    layer_name_edit_recorded: Option<usize>,
    layer_opacity_drag_recorded: bool,
    pending_command: Option<Command>,
    document_path: Option<PathBuf>,
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
            current_revision: 0,
            saved_revision: None,
            next_revision: 1,
            layer_scroll: 0,
            layer_name_edit_recorded: None,
            layer_opacity_drag_recorded: false,
            pending_command: None,
            document_path: None,
            editor_notice: None,
            rerender: false,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn take_command(&mut self) -> Option<Command> {
        let command = self.pending_command.take();
        if command.is_some() {
            self.ui.release_pointer();
        }
        command
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.document.is_some() && self.saved_revision != Some(self.current_revision)
    }

    pub fn start_new_document(&mut self) {
        self.end_tool();
        self.state = AppState::NewDocument;
        self.document = None;
        self.document_path = None;
        self.current_revision = 0;
        self.saved_revision = None;
        self.panning = false;
        self.undo_history.clear();
        self.redo_history.clear();
        self.layer_scroll = 0;
        self.layer_name_edit_recorded = None;
        self.layer_opacity_drag_recorded = false;
        self.editor_notice = None;
        self.ui.clear_focus();
        self.rerender = true;
    }

    pub fn exit(&mut self) {
        self.running = false;
    }

    pub fn document_path(&self) -> Option<&Path> {
        self.document_path.as_deref()
    }

    pub fn save_document(&mut self, path: &Path) -> Result<(), OdrawError> {
        self.end_tool();
        self.layer_name_edit_recorded = None;
        self.layer_opacity_drag_recorded = false;
        let result = file::save(self.document.as_ref().unwrap(), path);
        self.editor_notice = Some(match &result {
            Ok(()) => "DOCUMENT SAVED",
            Err(error) => Self::file_error_notice(error),
        });
        if result.is_ok() {
            self.document_path = Some(path.to_path_buf());
            self.saved_revision = Some(self.current_revision);
        }
        self.rerender = true;
        result
    }

    pub fn open_document(&mut self, path: &Path) -> Result<(), OdrawError> {
        let document = match file::load(path) {
            Ok(document) => document,
            Err(error) => {
                self.editor_notice = Some(Self::file_error_notice(&error));
                self.rerender = true;
                return Err(error);
            }
        };
        self.end_tool();
        self.canvas_view = CanvasView::fit(&document, self.editor_viewport());
        self.document = Some(document);
        self.document_path = Some(path.to_path_buf());
        self.validation_error = None;
        self.state = AppState::Editor;
        self.panning = false;
        self.undo_history.clear();
        self.redo_history.clear();
        self.advance_revision();
        self.saved_revision = Some(self.current_revision);
        self.layer_scroll = 0;
        self.layer_name_edit_recorded = None;
        self.layer_opacity_drag_recorded = false;
        self.editor_notice = Some("DOCUMENT OPENED");
        self.ui.clear_focus();
        self.rerender = true;
        Ok(())
    }

    pub fn export_document(&mut self, path: &Path, format: ImageFormat) -> std::io::Result<()> {
        let result = file::export(self.document.as_ref().unwrap(), path, format);
        self.editor_notice = Some(if result.is_ok() {
            "IMAGE EXPORTED"
        } else {
            "EXPORT FAILED"
        });
        self.rerender = true;
        result
    }

    pub fn import_image(&mut self, path: &Path, image: DecodedImage) -> Result<(), DocumentError> {
        self.end_tool();
        let snapshot = self.document.as_ref().unwrap().clone();
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("IMPORTED PNG")
            .to_owned();
        let result = self.document.as_mut().unwrap().import_layer(
            name,
            image.width,
            image.height,
            image.pixels,
        );
        self.editor_notice = Some(match result {
            Ok(()) => {
                self.remember(snapshot);
                let viewport = self.editor_viewport();
                self.canvas_view = CanvasView::fit(self.document.as_ref().unwrap(), viewport);
                self.reveal_active_layer();
                "IMAGE IMPORTED"
            }
            Err(DocumentError::AllocationFailed) => "NOT ENOUGH MEMORY",
            Err(DocumentError::TooLarge) => "IMAGE TOO LARGE",
            Err(DocumentError::InvalidDimensions) => "INVALID IMAGE",
        });
        self.rerender = true;
        result
    }

    pub fn report_image_import_error(&mut self) {
        self.editor_notice = Some("IMPORT FAILED");
        self.rerender = true;
    }

    pub fn report_file_dialog_error(&mut self) {
        self.editor_notice = Some("FILE DIALOG FAILED");
        self.rerender = true;
    }

    pub fn handle_event(&mut self, event: Event) {
        self.ui.handle_event(&event);
        match event {
            Event::CloseRequested => self.pending_command = Some(Command::Exit),
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
                        .layer_list_rect()
                        .contains(self.pointer.0, self.pointer.1) =>
            {
                let maximum = self
                    .document
                    .as_ref()
                    .unwrap()
                    .layers
                    .len()
                    .saturating_sub(self.layer_list_capacity());
                if delta < 0.0 {
                    self.layer_scroll = (self.layer_scroll + 1).min(maximum);
                } else if delta > 0.0 {
                    self.layer_scroll = self.layer_scroll.saturating_sub(1);
                }
                self.rerender = true;
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
            Event::KeyDown { key: Key::Control } => self.control_down = true,
            Event::KeyUp { key: Key::Control } => self.control_down = false,
            Event::KeyDown {
                key: Key::Letter('Z'),
            } if self.control_down && self.state == AppState::Editor => self.undo(),
            Event::KeyDown {
                key: Key::Letter('Y'),
            } if self.control_down && self.state == AppState::Editor => self.redo(),
            Event::KeyDown {
                key: Key::Letter('O'),
            } if self.control_down => self.pending_command = Some(Command::Open),
            Event::KeyDown {
                key: Key::Letter('S'),
            } if self.control_down && self.state == AppState::Editor => {
                self.pending_command = Some(Command::Save)
            }
            Event::KeyDown {
                key: Key::Letter('E'),
            } if self.control_down && self.state == AppState::Editor => {
                self.pending_command = Some(Command::Export)
            }
            Event::KeyDown {
                key: Key::Letter('I'),
            } if self.control_down && self.state == AppState::Editor => {
                self.pending_command = Some(Command::Import)
            }
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

        if let Some(error) = self.validation_error.or(self.editor_notice) {
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
            OPEN_DOCUMENT_BUTTON,
            Rect::new(panel.x + 28, panel.y + 356, 144, 48),
            "OPEN",
        ) {
            self.pending_command = Some(Command::Open);
        }
        if self.ui.button(
            framebuffer,
            CANCEL_BUTTON,
            Rect::new(panel.x + 188, panel.y + 356, 150, 48),
            "CANCEL",
        ) {
            self.pending_command = Some(Command::Exit);
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
                window_width - EDITOR_RIGHT_WIDTH as i32,
                56,
                EDITOR_RIGHT_WIDTH,
                self.window_size.1.saturating_sub(56),
            ),
        );
        self.ui.panel(
            framebuffer,
            Rect::new(
                EDITOR_LEFT_WIDTH as i32,
                window_height - 36,
                self.window_size
                    .0
                    .saturating_sub(EDITOR_LEFT_WIDTH + EDITOR_RIGHT_WIDTH),
                36,
            ),
        );

        self.ui.label(framebuffer, 20, 20, "OPENDRAW");
        if self.icon_button(
            framebuffer,
            UNDO_BUTTON,
            Rect::new(140, 9, 96, 38),
            EditorIcon::ArrowLeft,
        ) {
            self.undo();
        }
        if self.icon_button(
            framebuffer,
            REDO_BUTTON,
            Rect::new(244, 9, 96, 38),
            EditorIcon::ArrowRight,
        ) {
            self.redo();
        }
        if self.icon_button(
            framebuffer,
            OPEN_DOCUMENT_BUTTON,
            Rect::new(348, 9, 48, 38),
            EditorIcon::Folder,
        ) {
            self.pending_command = Some(Command::Open);
        }
        if self.icon_button(
            framebuffer,
            SAVE_DOCUMENT_BUTTON,
            Rect::new(404, 9, 48, 38),
            EditorIcon::Save,
        ) {
            self.pending_command = Some(Command::Save);
        }
        if self.icon_button(
            framebuffer,
            IMPORT_IMAGE_BUTTON,
            Rect::new(460, 9, 48, 38),
            EditorIcon::Import,
        ) {
            self.pending_command = Some(Command::Import);
        }
        if self.icon_button(
            framebuffer,
            EXPORT_IMAGE_BUTTON,
            Rect::new(516, 9, 48, 38),
            EditorIcon::Export,
        ) {
            self.pending_command = Some(Command::Export);
        }
        self.ui.label(framebuffer, 20, 66, "TOOLS");
        if self.icon_button(
            framebuffer,
            SELECT_BRUSH_BUTTON,
            Rect::new(8, 86, 104, 22),
            EditorIcon::Pencil,
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Brush;
            self.rerender = true;
        }
        if self.icon_button(
            framebuffer,
            SELECT_ERASER_BUTTON,
            Rect::new(8, 109, 104, 22),
            EditorIcon::Eraser,
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Eraser;
            self.rerender = true;
        }
        if self.icon_button(
            framebuffer,
            SELECT_EYEDROPPER_BUTTON,
            Rect::new(8, 132, 104, 22),
            EditorIcon::Eyedropper,
        ) {
            self.end_tool();
            self.active_tool = ActiveTool::Eyedropper;
            self.rerender = true;
        }
        if self.icon_button(
            framebuffer,
            SELECT_BUCKET_BUTTON,
            Rect::new(8, 155, 104, 22),
            EditorIcon::Bucket,
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
        let panel_x = window_width - EDITOR_RIGHT_WIDTH as i32;
        self.ui.label(framebuffer, panel_x + 16, 72, "DOCUMENT");
        self.ui.label(
            framebuffer,
            panel_x + 16,
            96,
            &format!("{} X {}", document_size.0, document_size.1),
        );
        let layer_count = self.document.as_ref().unwrap().layers.len();
        self.ui.label(
            framebuffer,
            panel_x + 16,
            124,
            &format!("LAYERS {layer_count}"),
        );
        self.render_layer_list(framebuffer);

        let controls_top = self.layer_controls_top();
        let controls_x = panel_x + 16;
        let (active_layer, layer_count, layer_opacity, mut layer_name) = {
            let document = self.document.as_ref().unwrap();
            (
                document.active_layer,
                document.layers.len(),
                document.active_layer().opacity,
                document.active_layer().name.clone(),
            )
        };
        let previous_name = layer_name.clone();
        self.ui.label(framebuffer, controls_x, controls_top, "NAME");
        let name_focused = self.ui.text_input(
            framebuffer,
            LAYER_NAME_INPUT,
            Rect::new(controls_x, controls_top + 18, 188, 28),
            &mut layer_name,
        );
        if layer_name != previous_name {
            if self.layer_name_edit_recorded != Some(active_layer) {
                self.checkpoint();
                self.layer_name_edit_recorded = Some(active_layer);
            }
            self.document.as_mut().unwrap().layers[active_layer].name = layer_name;
            self.editor_notice = None;
            self.rerender = true;
        }
        if !name_focused {
            self.layer_name_edit_recorded = None;
        }
        self.ui.label(
            framebuffer,
            controls_x,
            controls_top + 54,
            &format!("OPACITY {}%", (u16::from(layer_opacity) * 100 / 255)),
        );
        let mut opacity = u32::from(layer_opacity);
        let response = self.ui.slider(
            framebuffer,
            LAYER_OPACITY_SLIDER,
            Rect::new(controls_x, controls_top + 74, 188, 18),
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

        if self.ui.button(
            framebuffer,
            MOVE_UP_BUTTON,
            Rect::new(controls_x, controls_top + 102, 92, 28),
            "UP",
        ) && active_layer + 1 < layer_count
        {
            self.checkpoint();
            self.document.as_mut().unwrap().move_active_up();
            self.layer_changed();
            self.reveal_active_layer();
        }
        if self.ui.button(
            framebuffer,
            MOVE_DOWN_BUTTON,
            Rect::new(controls_x + 96, controls_top + 102, 92, 28),
            "DOWN",
        ) && active_layer > 0
        {
            self.checkpoint();
            self.document.as_mut().unwrap().move_active_down();
            self.layer_changed();
            self.reveal_active_layer();
        }

        if self.icon_button(
            framebuffer,
            ADD_LAYER_BUTTON,
            Rect::new(controls_x, controls_top + 138, 92, 28),
            EditorIcon::Add,
        ) {
            let snapshot = self.document.as_ref().unwrap().clone();
            let result = self.document.as_mut().unwrap().add_layer();
            self.editor_notice = match result {
                Ok(()) => {
                    self.remember(snapshot);
                    self.layer_scroll = 0;
                    None
                }
                Err(DocumentError::AllocationFailed) => Some("NOT ENOUGH MEMORY"),
                Err(_) => Some("LAYER LIMIT REACHED"),
            };
            self.rerender = true;
        }
        if self.icon_button(
            framebuffer,
            DELETE_LAYER_BUTTON,
            Rect::new(controls_x + 96, controls_top + 138, 92, 28),
            EditorIcon::Trash,
        ) {
            if layer_count > 1 {
                self.checkpoint();
                self.document.as_mut().unwrap().remove_active_layer();
                self.reveal_active_layer();
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
                if matches!(
                    notice,
                    "DOCUMENT SAVED" | "DOCUMENT OPENED" | "IMAGE IMPORTED" | "IMAGE EXPORTED"
                ) {
                    Color::rgb(90, 205, 130)
                } else {
                    Color::rgb(230, 90, 80)
                },
            );
        }

        if self.ui.button(
            framebuffer,
            NEW_DOCUMENT_BUTTON,
            Rect::new(window_width - 168, 9, 156, 38),
            "NEW DOC",
        ) {
            self.pending_command = Some(Command::New);
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

    fn render_layer_list(&mut self, framebuffer: &mut FrameBuffer) {
        let list = self.layer_list_rect();
        let capacity = self.layer_list_capacity();
        let layer_count = self.document.as_ref().unwrap().layers.len();
        self.layer_scroll = self.layer_scroll.min(layer_count.saturating_sub(capacity));

        for row in 0..capacity.min(layer_count - self.layer_scroll) {
            let index = layer_count - 1 - self.layer_scroll - row;
            let row_rect = Rect::new(
                list.x,
                list.y + row as i32 * LAYER_ROW_HEIGHT as i32,
                list.width,
                LAYER_ROW_HEIGHT,
            );
            let row_response = self.ui.click_target(
                LAYER_ROW_BASE + index as u32,
                Rect::new(
                    row_rect.x + 36,
                    row_rect.y,
                    row_rect.width - 36,
                    row_rect.height,
                ),
            );
            let visibility_rect = Rect::new(row_rect.x + 4, row_rect.y + 15, 28, 28);
            let visibility_response = self
                .ui
                .click_target(LAYER_VISIBILITY_BASE + index as u32, visibility_rect);

            if row_response.activated && self.document.as_ref().unwrap().active_layer != index {
                self.document.as_mut().unwrap().active_layer = index;
                self.layer_changed();
            }
            if visibility_response.activated {
                self.checkpoint();
                let visible = &mut self.document.as_mut().unwrap().layers[index].visible;
                *visible = !*visible;
                self.layer_changed();
            }

            let (selected, visible, opacity, name) = {
                let document = self.document.as_ref().unwrap();
                let layer = &document.layers[index];
                (
                    document.active_layer == index,
                    layer.visible,
                    layer.opacity,
                    layer.name.clone(),
                )
            };
            framebuffer.fill_rect(
                row_rect,
                if selected {
                    Color::rgb(65, 82, 112)
                } else if row_response.hovered {
                    Color::rgb(48, 54, 64)
                } else {
                    Color::rgb(35, 39, 46)
                },
            );
            framebuffer.draw_rect(
                row_rect,
                if selected || row_response.focused {
                    Color::rgb(90, 155, 230)
                } else {
                    Color::rgb(75, 82, 92)
                },
            );
            framebuffer.fill_rect(
                visibility_rect,
                if visibility_response.hovered || visibility_response.focused {
                    Color::rgb(58, 92, 140)
                } else {
                    Color::rgb(27, 30, 35)
                },
            );
            framebuffer.draw_rect(visibility_rect, Color::rgb(88, 94, 105));
            Self::draw_eye(
                framebuffer,
                visibility_rect.x + 14,
                visibility_rect.y + 14,
                visible,
            );

            let thumbnail = Rect::new(row_rect.x + 40, row_rect.y + 5, 48, 48);
            Self::draw_layer_thumbnail(
                framebuffer,
                &self.document.as_ref().unwrap().layers[index],
                thumbnail,
            );
            let display_name: String = name.chars().take(8).collect();
            framebuffer.draw_text(
                row_rect.x + 94,
                row_rect.y + 9,
                &display_name,
                Color::rgb(235, 238, 242),
                2,
            );
            framebuffer.draw_text(
                row_rect.x + 94,
                row_rect.y + 35,
                &format!("{}%", (u16::from(opacity) * 100 + 127) / 255),
                Color::rgb(180, 186, 196),
                1,
            );
        }

        if layer_count > capacity {
            let track = Rect::new(list.x + list.width as i32 - 5, list.y, 4, list.height);
            framebuffer.fill_rect(track, Color::rgb(22, 25, 30));
            let thumb_height = (list.height * capacity as u32 / layer_count as u32).max(16);
            let travel = list.height.saturating_sub(thumb_height);
            let maximum = layer_count - capacity;
            let thumb_y = list.y + (self.layer_scroll as u32 * travel / maximum as u32) as i32;
            framebuffer.fill_rect(
                Rect::new(track.x, thumb_y, track.width, thumb_height),
                Color::rgb(120, 130, 145),
            );
        }
    }

    fn draw_layer_thumbnail(framebuffer: &mut FrameBuffer, layer: &Layer, rect: Rect) {
        for y in 0..rect.height {
            for x in 0..rect.width {
                let checker = if (x / 6 + y / 6).is_multiple_of(2) {
                    Color::rgb(210, 210, 210)
                } else {
                    Color::rgb(160, 160, 160)
                };
                let pixel = layer
                    .pixels
                    .get_pixel(
                        x * layer.pixels.width / rect.width,
                        y * layer.pixels.height / rect.height,
                    )
                    .unwrap();
                let pixel = Color::rgba(
                    pixel.red(),
                    pixel.green(),
                    pixel.blue(),
                    ((u16::from(pixel.alpha()) * u16::from(layer.opacity) + 127) / 255) as u8,
                );
                let preview = pixel.blend_over(checker);
                framebuffer.set_pixel(
                    rect.x + x as i32,
                    rect.y + y as i32,
                    if layer.visible {
                        preview
                    } else {
                        Color::rgba(27, 30, 35, 120).blend_over(preview)
                    },
                );
            }
        }
        framebuffer.draw_rect(rect, Color::rgb(150, 160, 175));
    }

    fn draw_eye(framebuffer: &mut FrameBuffer, x: i32, y: i32, visible: bool) {
        let color = Color::rgb(210, 216, 224);
        framebuffer.draw_line(x - 9, y, x - 4, y - 5, color);
        framebuffer.draw_line(x - 4, y - 5, x + 4, y - 5, color);
        framebuffer.draw_line(x + 4, y - 5, x + 9, y, color);
        framebuffer.draw_line(x - 9, y, x - 4, y + 5, color);
        framebuffer.draw_line(x - 4, y + 5, x + 4, y + 5, color);
        framebuffer.draw_line(x + 4, y + 5, x + 9, y, color);
        if visible {
            framebuffer.fill_circle(x, y, 3, color);
        } else {
            framebuffer.draw_line(x - 8, y - 8, x + 8, y + 8, Color::rgb(220, 90, 80));
        }
    }

    fn icon_button(
        &mut self,
        framebuffer: &mut FrameBuffer,
        id: u32,
        rect: Rect,
        icon: EditorIcon,
    ) -> bool {
        let activated = self.ui.icon_button(framebuffer, id, rect);
        Self::draw_icon(framebuffer, rect, icon);
        activated
    }

    fn draw_icon(framebuffer: &mut FrameBuffer, rect: Rect, icon: EditorIcon) {
        let x = rect.x + rect.width as i32 / 2;
        let y = rect.y + rect.height as i32 / 2;
        let color = Color::rgb(255, 255, 255);
        match icon {
            EditorIcon::Pencil => {
                framebuffer.draw_line(x - 8, y + 6, x + 5, y - 7, color);
                framebuffer.draw_line(x - 5, y + 8, x + 8, y - 5, color);
                framebuffer.draw_line(x + 5, y - 7, x + 8, y - 5, color);
                framebuffer.draw_line(x - 8, y + 6, x - 5, y + 8, color);
                framebuffer.draw_line(x - 8, y + 6, x - 9, y + 9, color);
            }
            EditorIcon::Eraser => {
                framebuffer.draw_line(x - 8, y + 1, x - 2, y - 7, color);
                framebuffer.draw_line(x - 2, y - 7, x + 8, y, color);
                framebuffer.draw_line(x + 8, y, x + 2, y + 7, color);
                framebuffer.draw_line(x + 2, y + 7, x - 8, y + 1, color);
                framebuffer.draw_line(x - 4, y - 3, x + 6, y + 4, color);
            }
            EditorIcon::Eyedropper => {
                framebuffer.draw_circle(x + 5, y - 5, 3, color);
                framebuffer.draw_line(x + 2, y - 2, x - 6, y + 6, color);
                framebuffer.draw_line(x + 5, y + 1, x - 3, y + 9, color);
                framebuffer.draw_line(x - 6, y + 6, x - 3, y + 9, color);
                framebuffer.fill_circle(x - 6, y + 7, 1, color);
            }
            EditorIcon::Bucket => {
                framebuffer.draw_line(x - 7, y - 4, x + 7, y - 4, color);
                framebuffer.draw_line(x - 6, y - 4, x - 4, y + 7, color);
                framebuffer.draw_line(x - 4, y + 7, x + 4, y + 7, color);
                framebuffer.draw_line(x + 4, y + 7, x + 6, y - 4, color);
                framebuffer.draw_line(x - 4, y - 5, x - 2, y - 8, color);
                framebuffer.draw_line(x - 2, y - 8, x + 2, y - 8, color);
                framebuffer.draw_line(x + 2, y - 8, x + 4, y - 5, color);
                framebuffer.draw_line(x - 5, y + 2, x + 5, y + 2, color);
                framebuffer.fill_circle(x + 9, y + 7, 1, color);
            }
            EditorIcon::ArrowLeft => {
                framebuffer.draw_line(x - 12, y, x + 12, y, color);
                framebuffer.draw_line(x - 12, y, x - 4, y - 8, color);
                framebuffer.draw_line(x - 12, y, x - 4, y + 8, color);
            }
            EditorIcon::ArrowRight => {
                framebuffer.draw_line(x - 12, y, x + 12, y, color);
                framebuffer.draw_line(x + 12, y, x + 4, y - 8, color);
                framebuffer.draw_line(x + 12, y, x + 4, y + 8, color);
            }
            EditorIcon::Add => {
                framebuffer.fill_rect(Rect::new(x - 2, y - 9, 5, 19), color);
                framebuffer.fill_rect(Rect::new(x - 9, y - 2, 19, 5), color);
            }
            EditorIcon::Trash => {
                framebuffer.draw_rect(Rect::new(x - 6, y - 5, 13, 14), color);
                framebuffer.draw_line(x - 8, y - 7, x + 8, y - 7, color);
                framebuffer.draw_line(x - 3, y - 9, x + 3, y - 9, color);
                framebuffer.draw_line(x - 3, y - 9, x - 3, y - 7, color);
                framebuffer.draw_line(x + 3, y - 9, x + 3, y - 7, color);
                framebuffer.draw_line(x - 2, y - 2, x - 2, y + 6, color);
                framebuffer.draw_line(x + 2, y - 2, x + 2, y + 6, color);
            }
            EditorIcon::Folder => {
                framebuffer.draw_line(x - 12, y - 7, x - 4, y - 7, color);
                framebuffer.draw_line(x - 4, y - 7, x, y - 11, color);
                framebuffer.draw_line(x, y - 11, x + 7, y - 11, color);
                framebuffer.draw_line(x + 7, y - 11, x + 10, y - 7, color);
                framebuffer.draw_rect(Rect::new(x - 12, y - 7, 25, 17), color);
            }
            EditorIcon::Save => {
                framebuffer.draw_rect(Rect::new(x - 10, y - 11, 21, 22), color);
                framebuffer.draw_rect(Rect::new(x - 5, y - 9, 10, 7), color);
                framebuffer.draw_rect(Rect::new(x - 6, y + 3, 13, 8), color);
            }
            EditorIcon::Import => {
                framebuffer.draw_rect(Rect::new(x - 12, y - 9, 25, 19), color);
                framebuffer.fill_circle(x + 7, y - 4, 2, color);
                framebuffer.draw_line(x - 9, y + 6, x - 3, y, color);
                framebuffer.draw_line(x - 3, y, x + 1, y + 4, color);
                framebuffer.draw_line(x + 1, y + 4, x + 4, y + 1, color);
                framebuffer.draw_line(x + 4, y + 1, x + 10, y + 7, color);
            }
            EditorIcon::Export => {
                framebuffer.draw_line(x, y - 11, x, y + 3, color);
                framebuffer.draw_line(x, y + 3, x - 6, y - 3, color);
                framebuffer.draw_line(x, y + 3, x + 6, y - 3, color);
                framebuffer.draw_line(x - 10, y + 10, x + 10, y + 10, color);
                framebuffer.draw_line(x - 10, y + 4, x - 10, y + 10, color);
                framebuffer.draw_line(x + 10, y + 4, x + 10, y + 10, color);
            }
        }
    }

    fn file_error_notice(error: &OdrawError) -> &'static str {
        match error {
            OdrawError::UnsupportedVersion(_) => "UNSUPPORTED ODRAW VERSION",
            OdrawError::UnexpectedEndOfFile | OdrawError::InvalidFile => "INVALID ODRAW FILE",
            OdrawError::InvalidDimensions => "INVALID DOCUMENT SIZE",
            OdrawError::AllocationFailed => "NOT ENOUGH MEMORY",
            OdrawError::Io(_) => "FILE ERROR",
        }
    }

    fn checkpoint(&mut self) {
        let snapshot_bytes = Self::document_bytes(self.document.as_ref().unwrap());
        self.prepare_history(snapshot_bytes);
        self.undo_history.push(Snapshot {
            document: self.document.as_ref().unwrap().clone(),
            revision: self.current_revision,
        });
        self.advance_revision();
    }

    fn remember(&mut self, snapshot: Document) {
        let snapshot_bytes = Self::document_bytes(&snapshot);
        self.prepare_history(snapshot_bytes);
        self.undo_history.push(Snapshot {
            document: snapshot,
            revision: self.current_revision,
        });
        self.advance_revision();
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
        let dimensions_changed = self.document.as_ref().is_some_and(|current| {
            (current.width, current.height) != (previous.document.width, previous.document.height)
        });
        let current = self.document.replace(previous.document).unwrap();
        self.redo_history.push(Snapshot {
            document: current,
            revision: self.current_revision,
        });
        self.current_revision = previous.revision;
        if dimensions_changed {
            self.canvas_view =
                CanvasView::fit(self.document.as_ref().unwrap(), self.editor_viewport());
        }
        self.layer_name_edit_recorded = None;
        self.reveal_active_layer();
        self.editor_notice = None;
        self.rerender = true;
    }

    fn redo(&mut self) {
        let Some(next) = self.redo_history.pop() else {
            return;
        };
        self.end_tool();
        let dimensions_changed = self.document.as_ref().is_some_and(|current| {
            (current.width, current.height) != (next.document.width, next.document.height)
        });
        let current = self.document.replace(next.document).unwrap();
        self.undo_history.push(Snapshot {
            document: current,
            revision: self.current_revision,
        });
        self.current_revision = next.revision;
        if dimensions_changed {
            self.canvas_view =
                CanvasView::fit(self.document.as_ref().unwrap(), self.editor_viewport());
        }
        self.layer_name_edit_recorded = None;
        self.reveal_active_layer();
        self.editor_notice = None;
        self.rerender = true;
    }

    fn history_bytes(history: &[Snapshot]) -> u64 {
        history
            .iter()
            .map(|snapshot| Self::document_bytes(&snapshot.document))
            .sum()
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
        self.document_path = None;
        self.advance_revision();
        self.saved_revision = None;
        self.validation_error = None;
        self.state = AppState::Editor;
        self.panning = false;
        self.end_tool();
        self.undo_history.clear();
        self.redo_history.clear();
        self.layer_scroll = 0;
        self.layer_name_edit_recorded = None;
        self.editor_notice = None;
        self.ui.clear_focus();
    }

    fn advance_revision(&mut self) {
        self.current_revision = self.next_revision;
        self.next_revision = self
            .next_revision
            .checked_add(1)
            .expect("document revision counter exhausted");
    }

    fn layer_changed(&mut self) {
        self.layer_name_edit_recorded = None;
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
            self.window_size
                .0
                .saturating_sub(EDITOR_LEFT_WIDTH + EDITOR_RIGHT_WIDTH),
            self.window_size.1.saturating_sub(92),
        )
    }

    fn layer_controls_top(&self) -> i32 {
        (self.window_size.1.min(i32::MAX as u32) as i32 - 174).max(230)
    }

    fn layer_list_rect(&self) -> Rect {
        let panel_x = self.window_size.0.min(i32::MAX as u32) as i32 - EDITOR_RIGHT_WIDTH as i32;
        let top = 148;
        Rect::new(
            panel_x + 8,
            top,
            EDITOR_RIGHT_WIDTH - 16,
            (self.layer_controls_top() - top).max(0) as u32,
        )
    }

    fn layer_list_capacity(&self) -> usize {
        (self.layer_list_rect().height / LAYER_ROW_HEIGHT).max(1) as usize
    }

    fn reveal_active_layer(&mut self) {
        let capacity = self.layer_list_capacity();
        let document = self.document.as_ref().unwrap();
        let position_from_top = document.layers.len() - 1 - document.active_layer;
        if position_from_top < self.layer_scroll {
            self.layer_scroll = position_from_top;
        } else if position_from_top >= self.layer_scroll + capacity {
            self.layer_scroll = position_from_top - capacity + 1;
        }
        self.layer_scroll = self
            .layer_scroll
            .min(document.layers.len().saturating_sub(capacity));
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
        assert!(app.has_unsaved_changes());
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
        framebuffer.resize(1000, 700).unwrap();

        app.handle_event(Event::MouseMove { x: 900, y: 609 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        app.handle_event(Event::MouseMove { x: 810, y: 609 });
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
    fn layer_rename_is_one_undoable_edit() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        app.document = Some(Document::new(4, 4, Color::rgb(255, 255, 255)).unwrap());
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700).unwrap();

        app.handle_event(Event::MouseMove { x: 900, y: 555 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        app.handle_event(Event::MouseUp {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        for _ in 0..7 {
            app.handle_event(Event::KeyDown {
                key: Key::Backspace,
            });
            app.render(&mut framebuffer);
        }
        for character in "INK".chars() {
            app.handle_event(Event::TextInput { character });
        }
        app.render(&mut framebuffer);

        assert_eq!(app.document.as_ref().unwrap().active_layer().name, "INK");
        assert_eq!(app.undo_history.len(), 1);
        app.undo();
        assert_eq!(
            app.document.as_ref().unwrap().active_layer().name,
            "LAYER 1"
        );
    }

    #[test]
    fn visual_layer_list_selects_hides_and_scrolls_layers() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        let mut document = Document::new(4, 4, Color::rgb(255, 255, 255)).unwrap();
        for _ in 0..6 {
            document.add_layer().unwrap();
        }
        app.document = Some(document);
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700).unwrap();

        app.handle_event(Event::MouseMove { x: 900, y: 220 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        assert_eq!(app.document.as_ref().unwrap().active_layer, 5);
        app.handle_event(Event::MouseUp {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);

        app.handle_event(Event::MouseMove { x: 800, y: 230 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        assert!(!app.document.as_ref().unwrap().layers[5].visible);
        assert_eq!(app.undo_history.len(), 1);

        app.handle_event(Event::MouseMove { x: 900, y: 400 });
        app.handle_event(Event::MouseWheel { delta: -1.0 });
        assert_eq!(app.layer_scroll, 1);
    }

    #[test]
    fn icon_tool_button_draws_and_remains_clickable() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        app.active_tool = ActiveTool::Eraser;
        app.document = Some(Document::new(4, 4, Color::rgb(255, 255, 255)).unwrap());
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700).unwrap();

        app.handle_event(Event::MouseMove { x: 50, y: 96 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);

        assert_eq!(app.active_tool, ActiveTool::Brush);
        let white_pixels = (86..108)
            .flat_map(|y| (8..112).map(move |x| (x, y)))
            .filter(|&(x, y)| framebuffer.get_pixel(x, y) == Some(Color::rgb(255, 255, 255)))
            .count();
        assert!(white_pixels > 5);
    }

    #[test]
    fn persistent_color_picker_applies_click_immediately() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        app.document = Some(Document::new(4, 4, Color::rgb(255, 255, 255)).unwrap());
        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700).unwrap();

        app.handle_event(Event::MouseMove { x: 111, y: 339 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);

        assert_eq!(app.brush.settings.color, Color::rgb(255, 0, 0));
        assert!(app.undo_history.is_empty());
    }

    #[test]
    fn saves_opens_and_preserves_the_current_document_on_invalid_files() {
        let path = std::env::temp_dir().join(format!(
            "opendraw-{}-document-roundtrip.odraw",
            std::process::id()
        ));
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        let mut document = Document::new(3, 2, Color::rgb(255, 255, 255)).unwrap();
        document.active_layer_mut().name = String::from("SAVED");
        document.add_layer().unwrap();
        document.active_layer_mut().opacity = 91;
        app.document = Some(document);

        app.handle_event(Event::KeyDown { key: Key::Control });
        app.handle_event(Event::KeyDown {
            key: Key::Letter('S'),
        });
        assert_eq!(app.take_command(), Some(Command::Save));
        app.save_document(&path).unwrap();
        assert_eq!(app.document_path(), Some(path.as_path()));
        assert!(!app.has_unsaved_changes());
        app.checkpoint();
        app.document.as_mut().unwrap().layers[0].name = String::from("CHANGED");
        assert!(app.has_unsaved_changes());
        app.undo();
        assert!(!app.has_unsaved_changes());
        app.redo();
        assert!(app.has_unsaved_changes());
        app.open_document(&path).unwrap();
        assert_eq!(app.document.as_ref().unwrap().layers[0].name, "SAVED");
        assert_eq!(app.document.as_ref().unwrap().layers[1].opacity, 91);
        assert!(app.undo_history.is_empty());
        assert!(!app.has_unsaved_changes());

        app.checkpoint();
        app.document.as_mut().unwrap().layers[0].name = String::from("CURRENT");
        std::fs::write(&path, b"invalid").unwrap();
        assert!(app.open_document(&path).is_err());
        assert_eq!(app.document.as_ref().unwrap().layers[0].name, "CURRENT");
        assert!(app.has_unsaved_changes());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn close_request_waits_for_the_unsaved_changes_decision() {
        let mut app = App::new();
        app.state = AppState::Editor;
        app.document = Some(Document::new(2, 2, Color::rgb(255, 255, 255)).unwrap());

        app.handle_event(Event::CloseRequested);

        assert!(app.running());
        assert!(app.has_unsaved_changes());
        assert_eq!(app.take_command(), Some(Command::Exit));
        assert!(app.document.is_some());

        app.start_new_document();
        assert_eq!(app.state, AppState::NewDocument);
        assert!(!app.has_unsaved_changes());
        assert!(app.document.is_none());
    }

    #[test]
    fn exports_png_and_bmp_from_the_keyboard_command() {
        let base = std::env::temp_dir().join(format!("opendraw-{}-export", std::process::id()));
        let png_path = base.with_extension("png");
        let bmp_path = base.with_extension("bmp");
        let mut app = App::new();
        app.state = AppState::Editor;
        app.document = Some(Document::new(2, 2, Color::rgba(0, 0, 0, 0)).unwrap());

        app.handle_event(Event::KeyDown { key: Key::Control });
        app.handle_event(Event::KeyDown {
            key: Key::Letter('E'),
        });
        assert_eq!(app.take_command(), Some(Command::Export));
        app.export_document(&png_path, ImageFormat::Png).unwrap();
        app.export_document(&bmp_path, ImageFormat::Bmp).unwrap();
        assert_eq!(
            &std::fs::read(&png_path).unwrap()[..8],
            b"\x89PNG\r\n\x1a\n"
        );
        assert_eq!(&std::fs::read(&bmp_path).unwrap()[..2], b"BM");

        std::fs::remove_file(png_path).unwrap();
        std::fs::remove_file(bmp_path).unwrap();
    }

    #[test]
    fn imports_png_as_an_undoable_layer() {
        let mut app = App::new();
        app.window_size = (1000, 700);
        app.state = AppState::Editor;
        app.document = Some(Document::new(2, 4, Color::rgb(255, 255, 255)).unwrap());

        app.handle_event(Event::KeyDown { key: Key::Control });
        app.handle_event(Event::KeyDown {
            key: Key::Letter('I'),
        });
        assert_eq!(app.take_command(), Some(Command::Import));

        let mut framebuffer = FrameBuffer::default();
        framebuffer.resize(1000, 700).unwrap();
        app.handle_event(Event::MouseMove { x: 480, y: 28 });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.render(&mut framebuffer);
        assert_eq!(app.take_command(), Some(Command::Import));
        app.import_image(
            Path::new("logo.png"),
            DecodedImage {
                width: 4,
                height: 2,
                pixels: vec![Color::rgb(20, 40, 60).as_u32(); 8],
            },
        )
        .unwrap();
        assert_eq!(
            (
                app.document.as_ref().unwrap().width,
                app.document.as_ref().unwrap().height
            ),
            (4, 4)
        );
        assert_eq!(app.document.as_ref().unwrap().active_layer().name, "logo");

        app.undo();
        assert_eq!(
            (
                app.document.as_ref().unwrap().width,
                app.document.as_ref().unwrap().height
            ),
            (2, 4)
        );
        app.redo();
        assert_eq!(app.document.as_ref().unwrap().layers.len(), 2);
    }
}
