# Mantle

![example workflow](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle Logo](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

Mantle is a desktop application for controlling LIFX lights, born from the ashes of [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Releases

You can download the latest release [here](https://github.com/samclane/mantle/releases).

Builds are available for **Windows** (x86_64), **Linux** (x86_64), and **macOS** (Apple Silicon / aarch64).

## Screenshots

![Mantle](res/screenshot.png)

## Features

- Monitor LIFX light status
- Control LIFX lights
- Control multiple lights
  - Group lights
  - Control all lights
- Eyedropper tool
- Real Time Screen averaging for ambient lighting

## Building

Simply run `cargo build --release` to build the project. Ensure you have the `data` folder, containing `products.json`.

## Feature Flags

- `puffin` - Enables the Puffin profiler

## Feedback

Join the Discord server [here](https://discord.gg/TwqSeTTYqX) to provide feedback, report bugs, or request features.

## Acknowledgements

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Translations

| Language | Is Complete | Is Machine |
|----------|-------------|------------|
| English  | Yes         | No         |
| Spanish  | Yes         | Yes        |
| Chinese (Simplified) | Yes | Yes | 
| French | Yes | Yes |
| German | Yes | Yes |
| Portuguese (Brazil) | Yes | Yes |