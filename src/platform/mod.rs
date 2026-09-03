#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub fn run() -> std::io::Result<()> {
    windows::run()
}

#[cfg(not(target_os = "windows"))]
pub fn run() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "el backend de esta plataforma aún no está implementado",
    ))
}
