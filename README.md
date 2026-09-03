# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: eventos internos en Windows sobre el renderer 2D por software.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa abre una ventana redimensionable y presenta una escena de prueba con
líneas, rectángulos, círculos y transparencia generada directamente en un `Vec<u32>`.
El marcador del ratón responde a botones y rueda; el teclado modifica el color de la escena.
