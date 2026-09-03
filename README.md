# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: Milestone 001 — ventana nativa Win32 y framebuffer por software.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa abre una ventana redimensionable y presenta un checkerboard generado
directamente en un `Vec<u32>`.
