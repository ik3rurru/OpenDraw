# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: documentos raster con capas, herramientas de pintura, selector visual de color, historial, zoom y desplazamiento en Windows.

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

El panel izquierdo permite elegir pincel, borrador, cuentagotas o relleno. Pincel
y borrador ofrecen sliders de grosor y opacidad, con una vista previa a escala. El
cuentagotas toma el color compuesto de las capas visibles; el relleno reemplaza una
región de color continuo en la capa activa. Debajo de la opacidad, el selector de
tono, saturación y luminosidad permanece visible y aplica cada cambio al instante,
junto a los sliders RGB y la representación hexadecimal.

El panel derecho permite añadir, eliminar, seleccionar, ocultar, ordenar y ajustar
con un slider la opacidad de las capas.

`UNDO`/`REDO` o `Ctrl+Z`/`Ctrl+Y` restauran los cambios de pintura y capas.
