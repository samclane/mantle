# Mantle

![example workflow](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle Logo](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

Mantle is a cross-platform desktop application for discovering and controlling [LIFX](https://www.lifx.com/) smart lights over your local network. Built with Rust and [egui](https://github.com/emilk/egui), it offers real-time light management alongside unique ambient features like screen-color sync, audio-reactive lighting, saved scenes with scheduling, global keyboard shortcuts, and a system tray for quick access. Born from the ashes of [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Releases

You can download the latest release [here](https://github.com/samclane/mantle/releases).

Builds are available for **Windows** (x86_64), **Linux** (x86_64), and **macOS** (Apple Silicon / aarch64).

## Screenshots

![Mantle](res/screenshot.png)

## Features

### Light Discovery and Control

- Automatically discover LIFX bulbs on the local network
- Toggle power, adjust hue/saturation/brightness/kelvin with real-time sliders
- Set transition duration for smooth color changes
- Multizone support for light strips with per-zone and gradient controls

### Grouping

- Organize lights into groups
- Control all lights at once or filter/search by name

### Eyedropper and Screen Sync

- Pick any color from your screen with the eyedropper tool
- Average a screen region, window, or full monitor to drive ambient lighting in real time

### Audio-Reactive Lighting

- Drive light colors from microphone input using FFT analysis
- Optional waveform debug window for visualizing the audio spectrum

### Scenes and Scheduling

- Save and load named scenes (color presets across multiple lights)
- Schedule scenes to activate automatically at specific times of day

### Keyboard Shortcuts

- Bind global hotkeys to lighting actions for hands-free control

### System Tray

- Minimize to system tray
- Quick-toggle power and quit from the tray menu

### Localization

- Available in 6 languages: English, Spanish, Simplified Chinese, French, German, and Portuguese (Brazil)

## Built With

| Crate | Purpose |
|-------|---------|
| [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) / [egui](https://github.com/emilk/egui) | GUI framework |
| [lifx-core](https://github.com/eminence/lifx) | LIFX LAN protocol |
| [cpal](https://github.com/RustAudio/cpal) + [rustfft](https://github.com/ejmahler/RustFFT) | Audio capture and FFT |
| [xcap](https://github.com/niceChenGitH/xcap) | Screen capture |
| [rdev](https://github.com/Narsil/rdev) | Global keyboard/mouse input |
| [tray-icon](https://github.com/niceChenGitH/tray-icon) | System tray |
| [rust-i18n](https://github.com/longbridge/rust-i18n) | Localization |

## Building

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) toolchain (stable)
- The `data/` folder containing `products.json` (included in the repo; embedded at compile time)

**Linux only** -- install the following system libraries:

```bash
sudo apt install libasound2-dev libudev-dev libxtst-dev libevdev-dev libgtk-3-dev libxdo-dev
```

### Compile

```bash
cargo build --release
```

### Run

```bash
cargo run --release
```

Logs are written to `log/output.log`.

## Feature Flags

- `puffin` -- Enables the [Puffin](https://github.com/EmbarkStudios/puffin) profiler for performance analysis

```bash
cargo run --release --features puffin
```

## Contributing

The repository includes a pre-commit hook that runs `cargo fmt --check`, `cargo clippy`, and `cargo test`. To enable it:

```bash
git config core.hooksPath .githooks
```

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
