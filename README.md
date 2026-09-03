# OpenDraw

Editor de dibujo raster educativo escrito en Rust, sin dependencias externas.

Estado actual: documentos raster con capas, herramientas de pintura, selector visual de color, historial, archivos `.odraw`, importación PNG, exportación PNG/BMP, zoom y desplazamiento en Windows.

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

El panel derecho muestra las capas superpuestas de arriba abajo, con miniaturas del
contenido. Haz clic en una fila para activarla o en su ojo para mostrarla y ocultarla;
el campo `NAME` permite renombrarla y la rueda recorre listas largas. También permite
añadir, eliminar, ordenar y ajustar con un slider la opacidad de la capa activa.

`UNDO`/`REDO` o `Ctrl+Z`/`Ctrl+Y` restauran los cambios de pintura y capas.
Los botones de carpeta y disquete permiten abrir y guardar documentos `.odraw` mediante
los diálogos nativos de Windows; también pueden usarse `Ctrl+O` y `Ctrl+S`. El formato
versionado conserva dimensiones, capas, nombres, visibilidad, opacidad y píxeles.

El botón de exportación o `Ctrl+E` guarda la composición visible como PNG o BMP.
PNG conserva la transparencia; BMP aplana la imagen sobre un fondo blanco.
El botón de imagen o `Ctrl+I` importa un PNG como capa nueva conservando su transparencia.
La imagen se centra y el lienzo se amplía automáticamente si fuese necesario.
