# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: Milestone 003 en Windows — framebuffer por software y primitivas 2D.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa abre una ventana redimensionable y presenta una escena de prueba con
líneas, rectángulos, círculos y transparencia generada directamente en un `Vec<u32>`.
