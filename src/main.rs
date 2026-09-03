mod app;
mod document;
mod graphics;
mod platform;
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
        app.render(window.framebuffer());
        window.present();
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("OpenDraw: el backend de esta plataforma aún no está implementado");
}
