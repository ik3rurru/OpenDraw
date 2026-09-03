# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: documentos raster con capas, pincel circular, borrador, zoom y desplazamiento en Windows.

## Ejecutar

En Windows, con Rust instalado:

```powershell
cargo run
```

El programa permite elegir dimensiones y fondo blanco o transparente antes de entrar
en el editor. Todo se dibuja directamente en búferes `Vec<u32>`.

Haz clic en el campo para escribir; `Tab` cambia el foco y `Enter` o espacio activa el botón.
En el lienzo, dibuja con el botón izquierdo, usa la rueda para el zoom y desplaza
la vista con el botón central.

El panel izquierdo permite elegir pincel o borrador y ajustar su tamaño y alpha.
El pincel también permite elegir color; el borrador elimina contenido de la capa activa.

El panel derecho permite añadir, eliminar, seleccionar, ocultar, ordenar y cambiar
la opacidad de las capas.
