# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: creación validada de documentos y transición al editor en Windows.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa permite elegir dimensiones y fondo blanco o transparente antes de entrar
en el editor provisional. Todo se dibuja directamente en un `Vec<u32>`.

Haz clic en el campo para escribir; `Tab` cambia el foco y `Enter` o espacio activa el botón.
