use super::*;

const PRESET_BASE: u32 = 100;
const RESET_WHITE_BUTTON: u32 = 39;
const POSTAL_PPI: u32 = 300;

struct Preset {
    name: &'static str,
    width: u32,
    height: u32,
}

const fn millimeters_to_pixels(mm: u32) -> u32 {
    (mm * POSTAL_PPI * 10 + 127) / 254
}

const PRESETS: [Preset; 12] = [
    Preset {
        name: "SQUARE",
        width: 1000,
        height: 1000,
    },
    Preset {
        name: "SVGA",
        width: 800,
        height: 600,
    },
    Preset {
        name: "XGA",
        width: 1024,
        height: 768,
    },
    Preset {
        name: "WXGA+",
        width: 1440,
        height: 900,
    },
    Preset {
        name: "SXGA",
        width: 1280,
        height: 1024,
    },
    Preset {
        name: "WSXGA+",
        width: 1680,
        height: 1050,
    },
    Preset {
        name: "UXGA",
        width: 1600,
        height: 1200,
    },
    Preset {
        name: "FHD",
        width: 1920,
        height: 1080,
    },
    Preset {
        name: "WUXGA",
        width: 1920,
        height: 1200,
    },
    Preset {
        name: "4K",
        width: 3840,
        height: 2160,
    },
    Preset {
        name: "POSTAL",
        width: millimeters_to_pixels(100),
        height: millimeters_to_pixels(148),
    },
    Preset {
        name: "STICKER",
        width: 370,
        height: 320,
    },
];

struct Layout {
    panel: Rect,
    sizes: Rect,
    width: Rect,
    height: Rect,
    background: (i32, i32),
    picker: Rect,
    open: Rect,
    cancel: Rect,
    create: Rect,
    maximum_scroll: i32,
}

impl Layout {
    fn new(window: (u32, u32), scroll: i32) -> Self {
        let width = window.0.saturating_sub(32).clamp(360, 820);
        let wide = width >= 740;
        let height = if wide { 560 } else { 984 };
        let maximum_scroll = (height as i64 + 32 - i64::from(window.1)).max(0) as i32;
        let x = (window.0 as i64 - i64::from(width)).max(0) as i32 / 2;
        let y = ((window.1 as i64 - i64::from(height)) / 2).max(16) as i32
            - scroll.clamp(0, maximum_scroll);
        let panel = Rect::new(x, y, width, height);
        let sizes_width = if wide { width - 372 } else { width - 48 };
        let field_width = (sizes_width - 12) / 2;
        let background = if wide {
            (x + width as i32 - 324, y + 80)
        } else {
            (x + 24, y + 490)
        };
        let footer_y = y + height as i32 - 52;
        let button_width = (width - 72) / 3;
        Self {
            panel,
            sizes: Rect::new(x + 24, y + 104, sizes_width, 270),
            width: Rect::new(x + 24, y + 418, field_width, 34),
            height: Rect::new(x + 36 + field_width as i32, y + 418, field_width, 34),
            background,
            picker: Rect::new(background.0, background.1 + 63, 184, 184),
            open: Rect::new(x + 24, footer_y, button_width, 32),
            cancel: Rect::new(x + 36 + button_width as i32, footer_y, button_width, 32),
            create: Rect::new(x + 48 + button_width as i32 * 2, footer_y, button_width, 32),
            maximum_scroll,
        }
    }

    fn preset(&self, index: usize) -> Rect {
        let width = (self.sizes.width - 12) / 2;
        Rect::new(
            self.sizes.x + (index % 2) as i32 * (width as i32 + 12),
            self.sizes.y + (index / 2) as i32 * 46,
            width,
            40,
        )
    }

    fn control(&self, id: u32) -> Option<Rect> {
        let (x, y) = self.background;
        Some(match id {
            WIDTH_INPUT => self.width,
            HEIGHT_INPUT => self.height,
            COLOR_RADIO | TRANSPARENT_RADIO => Rect::new(x, y + 27, 300, 20),
            RESET_WHITE_BUTTON => Rect::new(x + 200, y + 63, 100, 32),
            OPEN_DOCUMENT_BUTTON => self.open,
            CANCEL_BUTTON => self.cancel,
            CREATE_BUTTON => self.create,
            id if (PRESET_BASE..PRESET_BASE + PRESETS.len() as u32).contains(&id) => {
                self.preset((id - PRESET_BASE) as usize)
            }
            COLOR_PICKER_BASE => self.picker,
            id if id == COLOR_PICKER_BASE + 1 => Rect::new(
                self.picker.x,
                self.picker.y + self.picker.height as i32 + 26,
                self.picker.width,
                16,
            ),
            id if (COLOR_PICKER_BASE + 2..COLOR_PICKER_BASE + 5).contains(&id) => Rect::new(
                self.picker.x,
                self.picker.y
                    + self.picker.height as i32
                    + 95
                    + (id - COLOR_PICKER_BASE - 2) as i32 * 20,
                self.picker.width,
                16,
            ),
            _ => return None,
        })
    }
}

impl App {
    pub(super) fn reveal_new_document_focus(&mut self) {
        let layout = Layout::new(self.window_size, self.new_document_scroll);
        let Some(rect) = self.ui.focused_id().and_then(|id| layout.control(id)) else {
            return;
        };
        let bottom = rect.y + rect.height as i32;
        let viewport_bottom = self.window_size.1.min(i32::MAX as u32) as i32 - 16;
        let shift = if rect.y < 16 {
            rect.y - 16
        } else if bottom > viewport_bottom {
            bottom - viewport_bottom
        } else {
            0
        };
        self.new_document_scroll =
            (self.new_document_scroll + shift).clamp(0, layout.maximum_scroll);
    }

    fn selected_preset(&self) -> Option<usize> {
        let dimensions = (
            self.width_input.parse::<u32>().ok()?,
            self.height_input.parse::<u32>().ok()?,
        );
        PRESETS
            .iter()
            .position(|preset| (preset.width, preset.height) == dimensions)
    }

    fn select_preset(&mut self, index: usize) {
        let preset = &PRESETS[index];
        self.width_input = preset.width.to_string();
        self.height_input = preset.height.to_string();
        self.validation_error = None;
        self.rerender = true;
    }

    pub(super) fn scroll_new_document(&mut self, delta: f32) {
        let layout = Layout::new(self.window_size, self.new_document_scroll);
        self.new_document_scroll =
            (self.new_document_scroll - (delta * 48.0) as i32).clamp(0, layout.maximum_scroll);
        self.ui.release_pointer();
        self.rerender = true;
    }

    pub(super) fn render_new_document(&mut self, framebuffer: &mut FrameBuffer) {
        let layout = Layout::new(self.window_size, self.new_document_scroll);
        self.new_document_scroll = self.new_document_scroll.clamp(0, layout.maximum_scroll);
        let panel = layout.panel;
        self.ui.panel(framebuffer, panel);
        self.ui
            .label(framebuffer, panel.x + 24, panel.y + 24, "NEW DOCUMENT");
        framebuffer.draw_text(
            panel.x + 24,
            panel.y + 52,
            "CHOOSE A PRESET OR ENTER YOUR OWN DIMENSIONS",
            Color::rgb(168, 185, 207),
            1,
        );
        self.ui
            .label(framebuffer, panel.x + 24, panel.y + 80, "PRESETS");
        let selected = self.selected_preset();
        for (index, preset) in PRESETS.iter().enumerate() {
            let rect = layout.preset(index);
            let response = self.ui.click_target(PRESET_BASE + index as u32, rect);
            let active = selected == Some(index);
            framebuffer.fill_rect(
                rect,
                if active {
                    Color::rgb(43, 77, 115)
                } else if response.hovered {
                    Color::rgb(55, 63, 76)
                } else {
                    Color::rgb(29, 34, 42)
                },
            );
            framebuffer.draw_rect(
                rect,
                if active || response.focused {
                    Color::rgb(102, 172, 249)
                } else {
                    Color::rgb(70, 80, 96)
                },
            );
            framebuffer.draw_text(
                rect.x + 10,
                rect.y + 6,
                preset.name,
                Color::rgb(232, 237, 244),
                2,
            );
            let dimensions = if preset.name == "POSTAL" {
                format!("100 X 148 MM / {POSTAL_PPI} PPI")
            } else {
                format!("{} X {} PX", preset.width, preset.height)
            };
            framebuffer.draw_text(
                rect.x + 10,
                rect.y + 26,
                &dimensions,
                Color::rgb(168, 185, 207),
                1,
            );
            if response.activated {
                self.select_preset(index);
            }
        }

        self.ui
            .label(framebuffer, layout.width.x, panel.y + 396, "WIDTH (PX)");
        self.ui.text_input(
            framebuffer,
            WIDTH_INPUT,
            layout.width,
            &mut self.width_input,
        );
        self.ui
            .label(framebuffer, layout.height.x, panel.y + 396, "HEIGHT (PX)");
        self.ui.text_input(
            framebuffer,
            HEIGHT_INPUT,
            layout.height,
            &mut self.height_input,
        );
        if selected != self.selected_preset() {
            self.rerender = true;
        }
        let note = match self.selected_preset() {
            Some(index) if PRESETS[index].name == "POSTAL" => {
                format!("100 X 148 MM AT {POSTAL_PPI} PPI")
            }
            Some(index) => format!("{} / DIMENSIONS CAN BE EDITED", PRESETS[index].name),
            None => String::from("CUSTOM SIZE"),
        };
        framebuffer.draw_text(
            layout.width.x,
            panel.y + 464,
            &note,
            Color::rgb(168, 185, 207),
            1,
        );
        self.render_background_picker(framebuffer, &layout);

        if let Some(error) = self.validation_error.or(self.editor_notice) {
            framebuffer.draw_text(
                panel.x + 24,
                layout.open.y - 24,
                error,
                Color::rgb(244, 113, 102),
                1,
            );
        }
        if self
            .ui
            .button(framebuffer, OPEN_DOCUMENT_BUTTON, layout.open, "OPEN")
        {
            self.pending_command = Some(Command::Open);
        }
        if self
            .ui
            .button(framebuffer, CANCEL_BUTTON, layout.cancel, "CANCEL")
        {
            self.pending_command = Some(Command::Exit);
        }
        if self
            .ui
            .button(framebuffer, CREATE_BUTTON, layout.create, "CREATE")
        {
            self.create_document();
        }
        if layout.maximum_scroll > 0 {
            let track = self.window_size.1.saturating_sub(32);
            let thumb =
                ((u64::from(track) * u64::from(track)) / u64::from(panel.height)).max(16) as u32;
            let travel = track.saturating_sub(thumb);
            let top = 16
                + (i64::from(self.new_document_scroll) * i64::from(travel)
                    / i64::from(layout.maximum_scroll)) as i32;
            framebuffer.fill_rect(
                Rect::new(panel.x + panel.width as i32 + 4, 16, 4, track),
                Color::rgb(55, 63, 76),
            );
            framebuffer.fill_rect(
                Rect::new(panel.x + panel.width as i32 + 4, top, 4, thumb),
                Color::rgb(102, 172, 249),
            );
        }
    }

    fn render_background_picker(&mut self, framebuffer: &mut FrameBuffer, layout: &Layout) {
        let (x, y) = layout.background;
        self.ui.label(framebuffer, x, y, "BACKGROUND");
        if self.ui.radio_button(
            framebuffer,
            COLOR_RADIO,
            x + 12,
            y + 37,
            "COLOR",
            self.background == Background::Color,
        ) {
            self.background = Background::Color;
            self.rerender = true;
        }
        if self.ui.radio_button(
            framebuffer,
            TRANSPARENT_RADIO,
            x + 146,
            y + 37,
            "TRANSPARENT",
            self.background == Background::Transparent,
        ) {
            self.background = Background::Transparent;
            self.rerender = true;
        }
        if self.ui.color_picker(
            framebuffer,
            COLOR_PICKER_BASE,
            layout.picker,
            &mut self.background_color,
            &mut self.background_hue,
        ) {
            self.background = Background::Color;
            self.rerender = true;
        }
        if self.ui.button(
            framebuffer,
            RESET_WHITE_BUTTON,
            Rect::new(x + 200, y + 63, 100, 32),
            "WHITE",
        ) {
            self.background = Background::Color;
            self.background_color = Color::rgb(255, 255, 255);
            self.background_hue = 0;
            self.rerender = true;
        }

        framebuffer.draw_text(x + 200, y + 117, "LAYERS", Color::rgb(168, 185, 207), 1);
        let transparent = Color::rgba(0, 0, 0, 0);
        let background = if self.background == Background::Transparent {
            transparent
        } else {
            self.background_color
        };
        for (index, color) in [transparent, background].into_iter().enumerate() {
            let rect = Rect::new(x + 200, y + 135 + index as i32 * 54, 100, 42);
            for py in 0..rect.height {
                for px in 0..rect.width {
                    let checker = if ((px / 8) + (py / 8)) % 2 == 0 {
                        Color::rgb(224, 224, 224)
                    } else {
                        Color::rgb(176, 176, 176)
                    };
                    framebuffer.set_pixel(
                        rect.x + px as i32,
                        rect.y + py as i32,
                        color.blend_over(checker),
                    );
                }
            }
            framebuffer.draw_rect(
                rect,
                if index == 0 {
                    Color::rgb(102, 172, 249)
                } else {
                    Color::rgb(120, 130, 145)
                },
            );
            let label = if index == 0 {
                "PAINT / ACTIVE"
            } else {
                "BACKGROUND"
            };
            framebuffer.fill_rect(
                Rect::new(rect.x + 1, rect.y + 26, rect.width - 2, 15),
                Color::rgba(24, 28, 36, 230),
            );
            framebuffer.draw_text(rect.x + 5, rect.y + 30, label, Color::rgb(235, 238, 243), 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(size: (u32, u32)) -> (App, FrameBuffer) {
        let mut app = App::new();
        app.window_size = size;
        let mut frame = FrameBuffer::default();
        frame.resize(size.0, size.1).unwrap();
        app.render(&mut frame);
        (app, frame)
    }

    fn click(app: &mut App, frame: &mut FrameBuffer, x: i32, y: i32) {
        app.handle_event(Event::MouseMove { x, y });
        app.handle_event(Event::MouseDown {
            button: MouseButton::Left,
        });
        app.handle_event(Event::MouseUp {
            button: MouseButton::Left,
        });
        app.render(frame);
    }

    #[test]
    fn all_requested_presets_update_editable_dimensions() {
        let expected = [
            (1000, 1000),
            (800, 600),
            (1024, 768),
            (1440, 900),
            (1280, 1024),
            (1680, 1050),
            (1600, 1200),
            (1920, 1080),
            (1920, 1200),
            (3840, 2160),
            (1181, 1748),
            (370, 320),
        ];
        let (mut app, mut frame) = setup((944, 601));
        assert_eq!(app.selected_preset(), Some(7));
        let layout = Layout::new(app.window_size, 0);
        for (index, (width, height)) in expected.into_iter().enumerate() {
            app.validation_error = Some("ENTER NUMERIC WIDTH");
            let rect = layout.preset(index);
            click(&mut app, &mut frame, rect.x + 20, rect.y + 20);
            assert_eq!(
                (app.width_input.as_str(), app.height_input.as_str()),
                (width.to_string().as_str(), height.to_string().as_str())
            );
            assert_eq!(app.selected_preset(), Some(index));
            assert_eq!(app.validation_error, None);
        }
        click(
            &mut app,
            &mut frame,
            layout.width.x + 10,
            layout.width.y + 10,
        );
        app.handle_event(Event::KeyDown {
            key: Key::Backspace,
        });
        app.render(&mut frame);
        app.handle_event(Event::TextInput { character: '1' });
        app.render(&mut frame);
        assert_eq!(app.width_input, "371");
        assert_eq!(app.height_input, "320");
        assert_eq!(app.selected_preset(), None);
    }

    #[test]
    fn background_picker_is_shared_independent_of_brush_and_defaults_to_white() {
        let (mut app, mut frame) = setup((944, 601));
        let layout = Layout::new(app.window_size, 0);
        let brush_color = app.brush.settings.color;
        assert_eq!(app.background, Background::Color);
        assert_eq!(app.background_color, Color::rgb(255, 255, 255));
        click(
            &mut app,
            &mut frame,
            layout.picker.x + 135,
            layout.picker.y + 40,
        );
        let chosen = app.background_color;
        assert_ne!(chosen, Color::rgb(255, 255, 255));
        assert_eq!(chosen.alpha(), 255);
        assert_eq!(app.brush.settings.color, brush_color);
        app.width_input = String::from("32");
        app.height_input = String::from("24");
        click(
            &mut app,
            &mut frame,
            layout.create.x + 10,
            layout.create.y + 10,
        );
        assert_eq!(app.state, AppState::Editor);
        let document = app.document.as_ref().unwrap();
        assert_eq!(document.layers[0].pixels.get_pixel(0, 0), Some(chosen));
        assert_eq!(document.active_layer, 1);
        assert!(
            document
                .active_layer()
                .pixels
                .pixels
                .iter()
                .all(|&p| p == 0)
        );
        app.start_new_document();
        assert_eq!(app.background, Background::Color);
        assert_eq!(app.background_color, Color::rgb(255, 255, 255));
        assert_eq!(app.brush.settings.color, brush_color);
    }

    #[test]
    fn transparent_background_and_white_reset_work_with_pen_input() {
        let (mut app, mut frame) = setup((944, 601));
        let layout = Layout::new(app.window_size, 0);
        let (x, y) = layout.background;
        let pen = PenSample {
            pointer_id: 7,
            ..PenSample::mouse((x + 146) as f32, (y + 37) as f32, true)
        };
        app.handle_event(Event::PenDown(pen));
        app.handle_event(Event::PenUp(PenSample {
            in_contact: false,
            ..pen
        }));
        app.render(&mut frame);
        assert_eq!(app.background, Background::Transparent);
        app.width_input = String::from("8");
        app.height_input = String::from("6");
        app.create_document();
        let document = app.document.as_ref().unwrap();
        assert_eq!(document.layers.len(), 2);
        assert_eq!(document.active_layer, 1);
        assert!(
            document
                .layers
                .iter()
                .all(|layer| layer.pixels.pixels.iter().all(|&p| p == 0))
        );
        app.start_new_document();
        app.background_color = Color::rgb(70, 100, 180);
        app.background = Background::Transparent;
        app.render(&mut frame);
        click(&mut app, &mut frame, x + 210, y + 73);
        assert_eq!(app.background, Background::Color);
        assert_eq!(app.background_color, Color::rgb(255, 255, 255));
    }

    #[test]
    fn erasing_the_drawing_layer_preserves_the_background_and_layered_save() {
        let (mut app, _) = setup((944, 601));
        let background = Color::rgb(55, 95, 170);
        app.background_color = background;
        app.width_input = String::from("32");
        app.height_input = String::from("24");
        app.create_document();
        let (x, y) = app.canvas_view.canvas_to_screen(10.5, 10.5);
        let contact = PenSample::mouse(x, y, true);
        app.brush.settings.radius = 2;
        app.begin_tool(contact);
        app.end_tool();
        let ink = app
            .document
            .as_ref()
            .unwrap()
            .active_layer()
            .pixels
            .get_pixel(10, 10)
            .unwrap();
        assert_eq!(ink, app.brush.settings.color);
        app.active_tool = ActiveTool::Eraser;
        app.eraser.settings.radius = 4;
        app.begin_tool(contact);
        app.end_tool();
        let document = app.document.as_ref().unwrap();
        assert_eq!(document.layers.len(), 2);
        assert_eq!(
            document
                .active_layer()
                .pixels
                .get_pixel(10, 10)
                .unwrap()
                .alpha(),
            0
        );
        assert_eq!(
            document.composite_pixel(10, 10, Color::rgba(0, 0, 0, 0)),
            Some(background)
        );
        assert!(
            document.layers[0]
                .pixels
                .pixels
                .iter()
                .all(|&p| p == background.as_u32())
        );
        app.undo();
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .active_layer()
                .pixels
                .get_pixel(10, 10),
            Some(ink)
        );
        app.redo();
        let path = std::env::temp_dir().join(format!(
            "opendraw-{}-new-background.odraw",
            std::process::id()
        ));
        app.save_document(&path).unwrap();
        let loaded = file::load(&path).unwrap();
        assert_eq!(loaded.layers.len(), 2);
        assert_eq!(loaded.layers[0].name, "BACKGROUND");
        assert_eq!(loaded.active_layer, 1);
        assert_eq!(
            loaded.composite_pixel(10, 10, Color::rgba(0, 0, 0, 0)),
            Some(background)
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn small_windows_scroll_and_tab_reveal_every_control() {
        let (mut app, mut frame) = setup((640, 480));
        assert!(Layout::new(app.window_size, 0).maximum_scroll > 0);
        for _ in 0..26 {
            app.handle_event(Event::KeyDown { key: Key::Tab });
            app.render(&mut frame);
            let layout = Layout::new(app.window_size, app.new_document_scroll);
            let focused = app.ui.focused_id().unwrap();
            let rect = layout.control(focused).unwrap();
            assert!(
                rect.y >= 16 && rect.y + rect.height as i32 <= 464,
                "id={focused}, rect={rect:?}"
            );
        }
        app.handle_event(Event::MouseWheel { delta: -100.0 });
        app.render(&mut frame);
        let layout = Layout::new(app.window_size, app.new_document_scroll);
        assert!(layout.create.y >= 0 && layout.create.y + 32 <= 480);
        app.handle_event(Event::Resized {
            width: 1000,
            height: 700,
        });
        frame.resize(1000, 700).unwrap();
        app.render(&mut frame);
        assert_eq!(app.new_document_scroll, 0);
    }

    #[test]
    #[ignore = "exports actual startup/editor frames to target/new-document for visual QA"]
    fn export_startup_previews() {
        let directory = Path::new("target/new-document");
        std::fs::create_dir_all(directory).unwrap();
        let export = |name: &str, frame: &FrameBuffer| {
            let mut image =
                Document::new(frame.width, frame.height, Color::rgba(0, 0, 0, 0)).unwrap();
            image
                .active_layer_mut()
                .pixels
                .pixels
                .clone_from(&frame.pixels);
            file::export(&image, &directory.join(name), ImageFormat::Png).unwrap();
        };
        let (mut app, mut frame) = setup((944, 601));
        export("startup-default.png", &frame);
        app.select_preset(10);
        app.background_color = Color::rgb(174, 205, 238);
        app.background_hue = 211;
        app.render(&mut frame);
        export("startup-postal.png", &frame);
        app.create_document();
        app.render(&mut frame);
        export("editor-background.png", &frame);
        let (mut small, mut frame) = setup((640, 480));
        export("startup-small-top.png", &frame);
        small.scroll_new_document(-100.0);
        small.render(&mut frame);
        export("startup-small-bottom.png", &frame);
    }
}
