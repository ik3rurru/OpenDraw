# Paquetes de lápiz del SDK de Windows

`windows-pen-x64.bin` y `windows-pen-x86.bin` contienen dos muestras de lápiz,
primero la más reciente, generadas por `windows_pen_fixture.c` con las
definiciones nativas de `WinUser.h`. No se generan desde los tipos Rust.

La referencia se comprobó con MSVC y Windows SDK 10.0.26100.0:

| Arquitectura | `sizeof(POINTER_INFO)` | `sizeof(POINTER_PEN_INFO)` | Alineamiento |
| --- | --- | --- | --- |
| x64 | 96 | 120 | 8 |
| x86 | 88 | 112 | 8 |

Los identificadores de dispositivo y ventana son valores ficticios que las
pruebas nunca desreferencian. Las muestras incluyen contacto, presión, posición,
inclinación y tiempo distintos de los datos de los campos adyacentes para detectar
desplazamientos incorrectos al leerlas.

`cargo test windows_sdk` comprueba los offsets de todos los campos y procesa los
paquetes hasta obtener un trazo visible. También verifica el tamaño de cada
registro antes de leer los bytes. La aplicación comprueba tamaño y alineamiento
al compilar, incluidos los builds de producción.

Para regenerar, abre una consola **x64 Native Tools** de Visual Studio en la raíz
del repositorio:

```bat
cl /nologo /W4 /WX tests\fixtures\windows_pen_fixture.c /Fe:target\pen-fixture.exe /Fo:target\pen-fixture.obj
target\pen-fixture.exe tests\fixtures\windows-pen-x64.bin
```

Repite desde una consola **x86 Native Tools**, usando `windows-pen-x86.bin` como
destino. El generador imprime los tamaños, alineamientos y offsets del SDK para
compararlos con las aserciones de Rust. El compilador C sólo es necesario para
regenerar las muestras; las pruebas habituales usan los archivos incluidos.
