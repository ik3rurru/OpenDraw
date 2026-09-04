# OpenDraw

Un editor de dibujo raster para Windows, hecho desde cero en Rust y sin dependencias externas.

![OpenDraw editando una ilustración](assets/opendraw-editor.png)

## Características

- Pincel, borrador, cuentagotas y relleno, con control de grosor y opacidad.
- Capas reordenables con nombre, visibilidad, opacidad y miniaturas.
- Selector de color HSL, RGB y hexadecimal.
- Historial de deshacer y rehacer para pintura y capas.
- Documentos `.odraw`, importación PNG y exportación PNG/BMP.
- Zoom con la rueda y desplazamiento del lienzo con el botón central.

## Ejecutar

Necesitas Windows y una instalación reciente de Rust:

```powershell
cargo run
```

Al crear un documento puedes elegir sus dimensiones y un fondo blanco o transparente.

## Atajos

| Acción | Atajo |
| --- | --- |
| Deshacer / rehacer | `Ctrl+Z` / `Ctrl+Y` |
| Abrir / guardar | `Ctrl+O` / `Ctrl+S` |
| Importar PNG | `Ctrl+I` |
| Exportar imagen | `Ctrl+E` |

El formato `.odraw` conserva las dimensiones del documento y todas sus capas, incluyendo nombres, visibilidad, opacidad y píxeles. Al exportar, PNG mantiene la transparencia y BMP aplana la imagen sobre fondo blanco.

## Estado

OpenDraw está en desarrollo y actualmente funciona en Windows.

## Licencia

[MIT](LICENSE)
