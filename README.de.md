# Mantle

![Beispiel-Workflow](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle-Logo](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

Mantle ist eine plattformübergreifende Desktop-Anwendung zur Erkennung und Steuerung von intelligenten [LIFX](https://www.lifx.com/)-Leuchten über Ihr lokales Netzwerk. Entwickelt mit Rust und [egui](https://github.com/emilk/egui), bietet die Anwendung eine Echtzeit-Lichtverwaltung sowie einzigartige Ambient-Funktionen – darunter Bildschirmfarben-Synchronisation, audioreaktive Beleuchtung, gespeicherte Szenen mit Zeitplanung, globale Tastenkürzel und ein System-Tray für den schnellen Zugriff. Entstanden aus der Asche des Projekts [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Releases

Die neueste Version können Sie [hier](https://github.com/samclane/mantle/releases) herunterladen.

Es sind Builds für **Windows** (x86_64), **Linux** (x86_64) und **macOS** (Apple Silicon / aarch64) verfügbar. ## Screenshots

![Mantle](res/screenshot.png)

## Funktionen

### Erkennung und Steuerung von Lichtern

- Automatische Erkennung von LIFX-Lampen im lokalen Netzwerk
- Ein-/Ausschalten sowie Anpassen von Farbton, Sättigung, Helligkeit und Farbtemperatur (Kelvin) über Echtzeit-Schieberegler
- Festlegen der Übergangsdauer für sanfte Farbwechsel
- Multizonen-Unterstützung für Lichtstreifen, mit Steuerungsmöglichkeiten für einzelne Zonen und Farbverläufe

### Gruppierung

- Organisation von Lichtern in Gruppen
- Gleichzeitige Steuerung aller Lichter oder Filtern/Suchen nach Namen

### Pipette und Bildschirmsynchronisation

- Auswahl beliebiger Farben direkt vom Bildschirm mithilfe des Pipetten-Werkzeugs
- Mittelwertbildung eines Bildschirmbereichs, eines Fensters oder des gesamten Monitors zur Echtzeit-Steuerung der Umgebungsbeleuchtung

### Audio-reaktive Beleuchtung

- Steuerung der Lichtfarben basierend auf Mikrofoneingaben mittels FFT-Analyse
- Optionales Debug-Fenster zur Visualisierung des Audiospektrums (Wellenform)

### Szenen und Zeitplanung

- Speichern und Laden benannter Szenen (Farbvoreinstellungen für mehrere Lichter gleichzeitig)
- Zeitplanung von Szenen, die sich zu bestimmten Tageszeiten automatisch aktivieren

### Tastenkürzel

- Zuweisung globaler Hotkeys zu Beleuchtungsaktionen für eine freihändige Steuerung

### System-Tray

- Minimierung in den System-Tray (Infobereich)
- Schnelles Ein-/Ausschalten und Beenden der Anwendung über das Tray-Menü

### Lokalisierung

- Verfügbar in 6 Sprachen: Englisch, Spanisch, vereinfachtes Chinesisch, Französisch, Deutsch und Portugiesisch (Brasilien)

## Entwickelt mit

| Crate | Zweck |
|-------|---------|
| [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) / [egui](https://github.com/emilk/egui) | GUI-Framework |
| [lifx-core](https://github.com/eminence/lifx) | LIFX-LAN-Protokoll |
| [cpal](https://github.com/RustAudio/cpal) + [rustfft](https://github.com/ejmahler/RustFFT) | Audioaufnahme und FFT |
| [xcap](https://github.com/niceChenGitH/xcap) | Bildschirmerfassung |
| [rdev](https://github.com/Narsil/rdev) | Globale Tastatur-/Mauseingabe |
| [tray-icon](https://github.com/niceChenGitH/tray-icon) | System-Tray |
| [rust-i18n](https://github.com/longbridge/rust-i18n) | Lokalisierung |

## Kompilieren

### Voraussetzungen

- [Rust](https://www.rust-lang.org/tools/install)-Toolchain (stabil)
- Der Ordner `data/`, der die Datei `products.json` enthält (im Repository enthalten; wird zur Kompilierzeit eingebettet)

**Nur Linux** – Installieren Sie die folgenden Systembibliotheken:

```bash
sudo apt install libasound2-dev libudev-dev libxtst-dev libevdev-dev libgtk-3-dev libxdo-dev
```

### Kompilieren

```bash
cargo build --release
```

### Ausführen

```bash
cargo run --release
```

Protokolle werden in die Datei `log/output.log` geschrieben.

## Feature-Flags

- `puffin` – Aktiviert den [Puffin](https://github.com/EmbarkStudios/puffin)-Profiler zur Leistungsanalyse

```bash
cargo run --release --features puffin
```

## Mitwirken

Das Repository enthält einen Pre-Commit-Hook, der `cargo fmt --check`, `cargo clippy` und `cargo test` ausführt. Um diesen zu aktivieren:

```bash
git config core.hooksPath .githooks
```

## Feedback

Treten Sie dem Discord-Server [hier](https://discord.gg/TwqSeTTYqX) bei, um Feedback zu geben, Fehler zu melden oder Funktionswünsche einzureichen.

## Danksagungen

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Übersetzungen

| Sprache | Vollständig | Maschinell erstellt |
|----------|-------------|---------------------| | Englisch | Ja | Nein |
| Spanisch | Ja | Ja |
| Chinesisch (vereinfacht) | Ja | Ja |
| Französisch | Ja | Ja |
| Deutsch | Ja | Ja |
| Portugiesisch (Brasilien) | Ja | Ja |