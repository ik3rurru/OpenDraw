# OpenDraw v0.1.0 — candidata a pre-release

Primera versión preliminar para Windows x64. Editor de dibujo raster con capas,
hecho en Rust sin crates externos.

## Funciones

- Pincel y borrador con bordes suavizados, presión para tamaño y opacidad,
  posiciones fraccionarias e interpolación de trazos.
- Windows Pointer Input: cursor en proximidad, goma física temporal, controles
  utilizables con lápiz y diagnóstico de tableta con `F12`.
- Doce tamaños preestablecidos de lienzo, dimensiones editables y fondo blanco,
  de color o transparente en una capa independiente.
- Capas con nombre, orden, visibilidad, opacidad y miniaturas; deshacer y rehacer.
- Selector HSV/RGB, cuentagotas y relleno.
- Documentos `.odraw`, importación PNG y exportación PNG/BMP, con guardado
  atómico y aviso de cambios sin guardar.

## Comprobaciones realizadas

Validado el 15 de septiembre de 2026, con Rust 1.98.0 y destino
`x86_64-pc-windows-msvc`:

- Formato, Clippy con advertencias tratadas como errores y compilación de producción.
- 86 pruebas automáticas y 3 comprobaciones adicionales de imágenes y rendimiento.
- Las 89 comprobaciones también pasan en producción con el runtime de C integrado.
- Revisión visual de las imágenes generadas del suavizado y del formulario de
  creación de documentos, incluida su vista desplazada en una ventana pequeña.
- Comprobación de las DLL importadas: el ejecutable con runtime integrado sólo
  depende de componentes de Windows.

## Estado y limitaciones

El dibujo con presión ya se había comprobado con una Wacom física. Sigue
pendiente confirmar el nuevo suavizado con esa tableta: trazos finos, variación
de presión, goma y coincidencia del cursor. Por ello esta candidata se propone
como **pre-release**.

Wintab, curvas de presión configurables y asignación de acciones a los botones
del lápiz siguen pendientes. La inclinación se registra y aparece en el
diagnóstico, pero no modifica la forma del pincel. El zoom muestra directamente
los píxeles del documento.

## Paquete de Windows

`OpenDraw-v0.1.0-windows-x64.zip` contiene el ejecutable, instrucciones y licencia.
Extrae el ZIP y ejecuta `opendraw.exe`. El runtime de C está integrado; no hace
falta instalar Rust ni Visual C++ por separado. Para usar la presión con Wacom,
activa Windows Ink en el perfil de OpenDraw del controlador.

La preparación local del paquete no publica una release ni crea una etiqueta.
