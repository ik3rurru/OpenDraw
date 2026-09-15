#[cfg(test)]
mod antialiasing_validation;
mod app;
mod document;
mod file;
mod graphics;
mod platform;
mod tools;
mod ui;

#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = run() {
        eprintln!("OpenDraw: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
fn run() -> std::io::Result<()> {
    let mut app = app::App::new();
    let mut window = platform::Window::new()?;

    while app.running() {
        let Some(event) = window.next_event()? else {
            break;
        };
        app.handle_event(event);
        while let Some(event) = window.poll_event()? {
            app.handle_event(event);
        }
        app.render(window.framebuffer());
        window.present()?;
        if let Some(command) = app.take_command() {
            handle_command(&mut app, &mut window, command);
            if app.running() {
                app.render(window.framebuffer());
                window.present()?;
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn handle_command(app: &mut app::App, window: &mut platform::Window, command: app::Command) {
    if matches!(
        command,
        app::Command::New | app::Command::Open | app::Command::Exit
    ) && !allow_destructive_action(app, window)
    {
        return;
    }

    match command {
        app::Command::New => app.start_new_document(),
        app::Command::Open => {
            if let Some(path) = dialog_selection(app, window.open_document_path())
                && let Err(error) = app.open_document(&path)
            {
                eprintln!("OpenDraw: {error}");
            }
        }
        app::Command::Save => {
            save_current_document(app, window);
        }
        app::Command::Import => {
            if let Some(path) = dialog_selection(app, window.import_image_path()) {
                match platform::decode_image(&path) {
                    Ok(image) => {
                        if let Err(error) = app.import_image(&path, image) {
                            eprintln!("OpenDraw: {error}");
                        }
                    }
                    Err(error) => {
                        eprintln!("OpenDraw: {error}");
                        app.report_image_import_error();
                    }
                }
            }
        }
        app::Command::Export => {
            if let Some((mut path, filter)) = dialog_selection(app, window.export_image_path()) {
                let format = export_format(&mut path, filter);
                if let Err(error) = app.export_document(&path, format) {
                    eprintln!("OpenDraw: {error}");
                }
            }
        }
        app::Command::Exit => app.exit(),
    }
}

#[cfg(target_os = "windows")]
fn allow_destructive_action(app: &mut app::App, window: &mut platform::Window) -> bool {
    if !app.has_unsaved_changes() {
        return true;
    }
    match window.confirm_save_changes() {
        Ok(platform::SaveChanges::Save) => save_current_document(app, window),
        Ok(platform::SaveChanges::Discard) => true,
        Ok(platform::SaveChanges::Cancel) => false,
        Err(error) => {
            eprintln!("OpenDraw: {error}");
            app.report_file_dialog_error();
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn save_current_document(app: &mut app::App, window: &platform::Window) -> bool {
    let path = app
        .document_path()
        .map(|path| path.to_path_buf())
        .or_else(|| dialog_selection(app, window.save_document_path()));
    let Some(path) = path else {
        return false;
    };
    match app.save_document(&path) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("OpenDraw: {error}");
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn dialog_selection<T>(app: &mut app::App, result: std::io::Result<Option<T>>) -> Option<T> {
    match result {
        Ok(selection) => selection,
        Err(error) => {
            eprintln!("OpenDraw: {error}");
            app.report_file_dialog_error();
            None
        }
    }
}

#[cfg(target_os = "windows")]
fn export_format(path: &mut std::path::PathBuf, filter: u32) -> file::ImageFormat {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let format = match extension.as_deref() {
        Some("bmp") => file::ImageFormat::Bmp,
        Some("png") => file::ImageFormat::Png,
        _ if filter == 2 => file::ImageFormat::Bmp,
        _ => file::ImageFormat::Png,
    };
    if !matches!(extension.as_deref(), Some("bmp" | "png")) {
        path.set_extension(format.extension());
    }
    format
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("OpenDraw: el backend de esta plataforma aún no está implementado");
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn export_format_uses_the_extension_or_selected_filter() {
        let mut named_bmp = std::path::PathBuf::from("drawing.BMP");
        assert_eq!(export_format(&mut named_bmp, 1), file::ImageFormat::Bmp);

        let mut unnamed = std::path::PathBuf::from("drawing");
        assert_eq!(export_format(&mut unnamed, 2), file::ImageFormat::Bmp);
        assert_eq!(unnamed.extension().unwrap(), "bmp");
    }
}
