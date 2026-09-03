mod graphics;
mod platform;

fn main() {
    if let Err(error) = platform::run() {
        eprintln!("OpenDraw: {error}");
        std::process::exit(1);
    }
}
