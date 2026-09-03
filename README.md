# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: primera UI propia sobre el renderer 2D por software en Windows.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa abre una ventana redimensionable y dibuja una pantalla interactiva con
fuente bitmap, panel, botón y campo de texto directamente en un `Vec<u32>`.

Haz clic en el campo para escribir; `Tab` cambia el foco y `Enter` o espacio activa el botón.
