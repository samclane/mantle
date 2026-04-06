# Mantle

![Flujo de trabajo de ejemplo](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Logotipo de Mantle](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md)

Mantle es una aplicación de escritorio para controlar luces LIFX, nacida de las cenizas de [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Lanzamientos

Puedes descargar la última versión [aquí](https://github.com/samclane/mantle/releases).

**Nota:** Por el momento solo estoy generando compilaciones para Windows, pero el proyecto debería compilarse también en Linux y MacOS. Aún estoy configurando GitHub Actions para lograrlo.

## Capturas de pantalla

![Mantle](res/screenshot.png)

## Características

- Monitorizar el estado de las luces LIFX
- Controlar las luces LIFX
- Controlar múltiples luces
- Agrupar luces
- Controlar todas las luces
- Herramienta de cuentagotas
- Promedio de color de la pantalla en tiempo real para iluminación ambiental

## Compilación

Simplemente ejecuta `cargo build --release` para compilar el proyecto. Asegúrate de tener la carpeta `data`, que contiene el archivo `products.json`.

## Banderas de características (Feature Flags)

- `puffin` - Habilita el perfilador Puffin

## Comentarios y sugerencias

Únete al servidor de Discord [aquí](https://discord.gg/TwqSeTTYqX) para enviar tus comentarios, reportar errores o solicitar nuevas características.

## Agradecimientos

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Traducciones

| Idioma  | ¿Completa? | ¿Automática? |
|---------|------------|--------------|
| Inglés  | Sí         | No           |
| Español | Sí         | Sí           |
| Chino (simplificado) | Sí | Sí |
| Francés | Sí | Sí |