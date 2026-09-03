# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: documentos raster reales con una capa, lienzo, zoom y desplazamiento en Windows.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa permite elegir dimensiones y fondo blanco o transparente antes de entrar
en el editor. Todo se dibuja directamente en búferes `Vec<u32>`.

Haz clic en el campo para escribir; `Tab` cambia el foco y `Enter` o espacio activa el botón.
En el lienzo, la rueda controla el zoom y el botón central permite desplazar la vista.
