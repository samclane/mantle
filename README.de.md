# Mantle

![Beispiel-Workflow](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle-Logo](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

Mantle ist eine Desktop-Anwendung zur Steuerung von LIFX-Lampen, die aus der Asche des Projekts [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel) auferstanden ist.

## Releases

Sie können das neueste Release [hier](https://github.com/samclane/mantle/releases) herunterladen.

**Hinweis:** Derzeit erstelle ich Builds nur für Windows; die Anwendung sollte sich jedoch auch unter Linux und macOS kompilieren lassen. Ich bin noch dabei, die entsprechenden GitHub Actions dafür einzurichten.

## Screenshots

![Mantle](res/screenshot.png)

## Funktionen

- Überwachung des Status von LIFX-Lampen
- Steuerung von LIFX-Lampen
- Steuerung mehrerer Lampen
- Gruppierung von Lampen
- Steuerung aller Lampen gleichzeitig
- Pipetten-Werkzeug (Eyedropper)
- Echtzeit-Bildschirm-Mittelwertbildung für Umgebungsbeleuchtung (Ambient Lighting)

## Kompilierung

Führen Sie einfach `cargo build --release` aus, um das Projekt zu kompilieren. Stellen Sie sicher, dass der Ordner `data` vorhanden ist und die Datei `products.json` enthält.

## Feature-Flags

- `puffin` – Aktiviert den Puffin-Profiler

## Feedback

Treten Sie dem Discord-Server [hier](https://discord.gg/TwqSeTTYqX) bei, um Feedback zu geben, Fehler zu melden oder neue Funktionen vorzuschlagen. ## Danksagungen

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Übersetzungen

| Sprache | Vollständig | Maschinell |
|----------|-------------|------------|
| Englisch | Ja          | Nein       |
| Spanisch | Ja          | Ja         |
| Chinesisch (vereinfacht) | Ja | Ja |
| Französisch | Ja | Ja |
| Deutsch | Ja | Ja |
| Portugiesisch (Brasilien) | Ja | Ja |