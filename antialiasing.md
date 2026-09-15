# Antialiasing del pincel y del borrador

Plan guardado el 14 de septiembre de 2026 e implementado el 15 de septiembre.
Estado: implementación y validación automática completadas; pendiente de
confirmación visual con la Wacom física después de este cambio.

## Punto de partida (14 de septiembre)

El usuario ha confirmado con una Wacom física que el dibujo con presión ya
funciona. La captura de la sesión muestra trazos continuos de grosor variable,
con bordes muy pixelados. La siguiente mejora acordada es suavizar esos bordes.

El soporte actual incluye presión para tamaño y opacidad, interpolación de
muestras, goma temporal, uso del lápiz en la UI y diagnóstico con F12.
Ratón y lápiz comparten el motor. Pasan 65 pruebas, Clippy y las compilaciones
de desarrollo y producción.

Se corrigió un fallo de `POINTER_INFO`: faltaban campos nativos y se reservaban
88 bytes por muestra de lápiz en x64, mientras Windows escribe 120. Mantener las
guardas de compilación y las pruebas con paquetes generados desde el SDK de
Windows en `tests/fixtures/`. No sustituir esa referencia independiente por
muestras creadas únicamente con los tipos Rust.

## Objetivo y alcance

Añadir antialiasing a las marcas circulares del pincel y del borrador, con
posiciones y tamaños fraccionarios. La mejora debe quedar en los píxeles de la
capa y conservarse al guardar o exportar, además de verse en el editor.

Mantener Rust sin crates externos, separación entre plataforma y herramientas,
presión, interpolación por distancia, deshacer/rehacer y recorte al lienzo.

La primera versión se limita a los pinceles circulares actuales. El campo
PPP/PPI para impresión, las curvas de presión, el estabilizador de trazos, los
pinceles con textura y el filtrado de la vista al hacer zoom quedan fuera de
esta tarea. Tampoco se necesita renderizar todo el documento a una resolución
multiplicada para después reducirlo.

## Por qué el campo de resolución no resuelve estos bordes

Con ancho y alto fijos en píxeles, cambiar PPP modifica el tamaño de impresión,
sin añadir detalle ni suavizar el dibujo. Si las dimensiones se indican en
centímetros, los PPP sí determinan cuántos píxeles se crean.

El antialiasing añade cobertura parcial en el contorno de cada marca. El lienzo
seguirá siendo raster y sus píxeles seguirán siendo visibles a mucho aumento.
El zoom actual muestra los píxeles directamente; mejorar su interpolación es
una tarea diferente y no sustituye al antialiasing en la capa.

## Código implementado

| Archivo | Punto relevante |
| --- | --- |
| `src/document/mod.rs` | Cobertura circular compartida, recorte y composición del pincel y del borrador. |
| `src/tools/brush.rs`, `src/tools/eraser.rs` | Estampado con centros y radios fraccionarios, presión y opacidad. |
| `src/tools/dynamics.rs` | Radio geométrico continuo en `f32`, conservando el diámetro del control. |
| `src/tools/stroke.rs` | Muestreo por distancia, deduplicación de posiciones y cierre del segmento. |
| `src/app.rs`, `src/graphics/framebuffer.rs` | Cursor circular continuo, suavizado y recortado al área del editor. |
| `src/document/canvas_view.rs` | Convención explícita de coordenadas, sin cambiar el filtrado del zoom. |
| `src/document/antialiasing_tests.rs` | Cobertura, referencia geométrica independiente, transparencia y recorte. |
| `src/antialiasing_validation.rs` | Comparación visual reproducible y mediciones de rendimiento. |

## Propuesta original

1. **Definir coordenadas y tamaño continuo.**
   Conservar `f32` hasta rasterizar. Acordar una convención explícita para el
   centro del píxel, evitando desplazar el dibujo medio píxel respecto al
   cursor. Los ajustes actuales expresan un diámetro de `2 * radius + 1`;
   mantener ese significado, incluido el pincel de un píxel y el mínimo del
   10% con presión. El radio geométrico continuo y el radio entero actual no
   son intercambiables sin esta conversión.

2. **Compartir el cálculo de cobertura circular.**
   Recorrer sólo el rectángulo de la marca, ampliado para incluir el borde y
   recortado a la capa. Devolver una cobertura entre 0 y 1 por píxel. Evaluar
   una transición aproximada de un píxel según la distancia al borde; no
   confundir esa aproximación con el área exacta de intersección. Comprobar
   especialmente círculos pequeños y centros fraccionarios. El interior
   completamente cubierto puede conservar una ruta rápida.

3. **Componer pincel y borrador con esa cobertura.**
   Para pintar, multiplicar la opacidad efectiva, incluida la presión, por la
   cobertura antes de componer el color. Para borrar, usar la cobertura para
   modular cuánto alfa se elimina, conservando el comportamiento del color
   existente. Probar tanto fondos opacos como transparentes.

4. **Conservar el muestreo por distancia.**
   Revisar la deduplicación por píxel entero: con antialiasing, dos centros
   fraccionarios dentro de una misma celda pueden producir marcas diferentes.
   Seguir descartando muestras duplicadas y evitar estampar una vez por cada
   evento del dispositivo. La densidad y opacidad del trazo no deben depender
   de cuántos paquetes se reciban. Revisar el espaciado mínimo y el cierre del
   último segmento junto con este cambio.

5. **Ajustar la vista previa del cursor.**
   Representar el tamaño efectivo continuo para que el contorno mostrado sea
   coherente con el pincel, la presión y el zoom.

6. **Revisar calidad y rendimiento.**
   Comparar trazos pequeños, diagonales, curvas y cambios de presión. Prestar
   atención a halos, costuras y oscurecimiento del borde por marcas
   superpuestas. Conservar la acumulación de opacidad actual salvo ajustes
   necesarios para la cobertura; no rediseñar aquí todo el modelo de flujo.
   Medir pinceles grandes y movimientos rápidos, evitando asignaciones por
   marca y cálculos costosos en píxeles completamente cubiertos.

## Decisiones de implementación

- **Coordenadas:** el documento empieza en `(0, 0)` y el centro del píxel
  `(x, y)` está en `(x + 0.5, y + 0.5)`. La posición del puntero pasa sin
  redondear desde la vista hasta la marca. `BrushSample::pixel` sólo se usa
  para las herramientas que seleccionan una celda: relleno y cuentagotas.
- **Tamaño:** el control sigue expresando un diámetro de `2 * radius + 1`.
  El radio geométrico es la mitad del diámetro efectivo después de aplicar
  presión, con un mínimo de `0.5` píxeles. Por ejemplo, tamaño 41 y presión
  50% producen diámetro 22.55 y radio 11.275, sin redondear a un tamaño impar.
- **Cobertura:** transición lineal de un píxel alrededor del radio geométrico:
  `clamp(radio + 0.5 - distancia_al_centro, 0, 1)`. Es una aproximación de
  cobertura, no un cálculo exacto del área. Se compara con una referencia
  geométrica independiente de 64 × 64 puntos por píxel para radios pequeños
  y centros fraccionarios. El pincel de un píxel conserva cobertura completa
  cuando está centrado en una celda y reparte alfa al desplazarse.
- **Composición:** la cobertura multiplica el alfa efectivo del pincel o la
  cantidad que borra la goma. Se conserva el RGB al borrar y se usa la
  composición existente con alfa no premultiplicado. El recorte se realiza
  antes de recorrer los píxeles; no hay asignaciones por marca ni raíces
  cuadradas para el interior completamente cubierto.
- **Muestreo:** separación de un cuarto del diámetro efectivo, con mínimo de
  medio píxel. Se acumula la distancia sobrante en `f64` entre paquetes y
  se conservan las posiciones de las marcas en `f32`. Dos centros distintos
  dentro de una celda pueden pintar; los paquetes estacionarios no acumulan
  alfa. El último contacto se estampa una sola vez, usando una tolerancia de
  `0.0001` píxeles para el error numérico en la posición. La presión que cambia
  sin mover el lápiz se aplica en la siguiente marca, como antes.
- **Cursor:** centro y radio continuos, escalados por el zoom, con dos
  contornos suavizados para el contraste. En hover muestra el tamaño máximo;
  durante el contacto muestra el tamaño efectivo según la presión y la goma.
- **Acumulación:** las marcas superpuestas siguen acumulando opacidad. Se
  mantiene la respuesta de flujo existente; el borde puede oscurecerse con
  las superposiciones. El zoom conserva la visualización directa de píxeles.

Las pruebas anteriores que usaban una marca para inicializar un píxel ahora
la sitúan en el centro geométrico de esa celda. Las pruebas de contacto y
goma de un píxel usan esa misma referencia. La prueba de coordenadas
fraccionarias comprueba que el alfa se reparte hacia los vecinos correctos.

## Validación y criterios de aceptación

- [x] Bordes con cobertura parcial, interior correcto y exterior intacto.
- [x] Desplazamientos y tamaños fraccionarios producen cambios graduales.
- [x] El pincel mínimo sigue siendo visible con presión efectiva suficiente.
- [x] El borrador reduce el alfa de forma gradual en el contorno.
- [x] No aparecen halos de color sobre fondos transparentes.
- [x] Muestras duplicadas no añaden opacidad; cambiar la frecuencia de
      muestreo mantiene el resultado de un mismo recorrido y presión.
- [x] Recorte correcto en los cuatro bordes, esquinas y documentos pequeños.
- [x] Hover no pinta; levantar el lápiz, cancelar el contacto o perder foco
      termina el trazo. La goma y el ratón conservan su comportamiento.
- [x] Deshacer/rehacer conserva un trazo como una sola operación.
- [x] PNG y documentos guardados conservan los bordes suavizados.
- [x] Comparación visual por software a 100% y 800%.
- [ ] Confirmación del nuevo antialiasing con la Wacom física.
- [x] Rendimiento medido con pinceles grandes y trazos rápidos.

Se añadieron 15 pruebas automáticas y dos comprobaciones manuales reproducibles
marcadas como `ignored`: una exporta imágenes y otra mide tiempos. La comparación
de frecuencia de entrada cubre hasta 1.024 paquetes, duplicados, diagonales y
presión variable. La exportación PNG se vuelve a decodificar con el lector
nativo de Windows y se compara píxel a píxel con la capa y con el `.odraw`.

La revisión de `target/antialiasing/quality.png` muestra diagonales finas,
presión variable, curvas semitransparentes y borrado gradual a 100% y 800%.
Los cuatro documentos de ejemplo se guardan como `stroke-0` a `stroke-3`, en
PNG y `.odraw`, en el mismo directorio. Se generan con:

```powershell
cargo test --release antialiasing_validation -- --ignored --nocapture --test-threads=1
```

### Rendimiento observado

Windows x64, `rustc 1.98.0`, perfil `release`. Marcas con centros fraccionarios,
lienzo de 2048 × 512 y opacidad 90/255. Mediana de cinco tandas de 512 marcas,
excluyendo creación y limpieza del búfer:

| Diámetro | Pincel, µs/marca | Borrador, µs/marca |
| --- | ---: | ---: |
| 9 px | 0.5 | 0.2 |
| 65 px | 14.0 | 6.1 |
| 255 px | 203.4 | 88.2 |

Con opacidad 255/255, el pincel de 255 px baja a 90.5 µs/marca gracias a la
ruta rápida del interior. Un movimiento de 1.792 píxeles horizontales y 150
verticales, recibido en un solo paquete y con presión de 10% a 100%, tarda
**3.712 ms con pincel** de diámetro máximo 255 y **1.710 ms con borrador**
(mediana de 31 trazos). Son tiempos del motor; no incluyen repintar la ventana,
el controlador de la tableta ni su latencia.

### Comprobación física pendiente

Usar la Wacom a 100% y con aumento: puntos de 1 y 3 px, diagonales, curvas,
variaciones de presión, goma y bordes del lienzo. Verificar la coincidencia
del cursor, levantar y volver a apoyar, perder foco, deshacer/rehacer y abrir
el PNG exportado. El soporte de presión anterior sí se había probado con
hardware; la validación física de estos bordes sigue pendiente.

### Comandos de cierre

Ejecutados correctamente el 15 de septiembre de 2026:

| Comando | Resultado |
| --- | --- |
| `cargo fmt --check` | Correcto |
| `cargo test` | 80 pruebas pasan; 2 comprobaciones manuales omitidas por defecto |
| `cargo clippy --all-targets -- -D warnings` | Correcto |
| `cargo build` | Correcto |
| `cargo build --release` | Correcto |
| Comprobaciones manuales con el comando anterior de `release` | 2 pasan: imágenes exportadas y tiempos medidos |

Ejecutable generado: `target/release/opendraw.exe`.
