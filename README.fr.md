# Mantle

![Exemple de flux de travail](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Logo Mantle](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

Mantle est une application de bureau multiplateforme permettant de découvrir et de contrôler les ampoules connectées [LIFX](https://www.lifx.com/) via votre réseau local. Développée en Rust et [egui](https://github.com/emilk/egui), elle offre une gestion de l'éclairage en temps réel ainsi que des fonctionnalités d'ambiance uniques, telles que la synchronisation des couleurs avec l'écran, un éclairage réactif au son, des scènes enregistrées avec planification, des raccourcis clavier globaux et une icône dans la zone de notification pour un accès rapide. Née des cendres de [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Versions

Vous pouvez télécharger la dernière version [ici](https://github.com/samclane/mantle/releases).

Des versions compilées sont disponibles pour **Windows** (x86_64), **Linux** (x86_64) et **macOS** (Apple Silicon / aarch64). ## Captures d'écran

![Mantle](res/screenshot.png)

## Fonctionnalités

### Découverte et contrôle des lumières

- Découverte automatique des ampoules LIFX sur le réseau local
- Activation/désactivation, réglage de la teinte, de la saturation, de la luminosité et de la température de couleur (Kelvin) via des curseurs en temps réel
- Définition de la durée de transition pour des changements de couleur fluides
- Prise en charge multizone pour les rubans lumineux, avec contrôles par zone et gestion des dégradés

### Groupement

- Organisation des lumières en groupes
- Contrôle simultané de toutes les lumières ou filtrage/recherche par nom

### Pipette et synchronisation d'écran

- Sélection de n'importe quelle couleur à l'écran grâce à l'outil pipette
- Calcul de la couleur moyenne d'une zone spécifique, d'une fenêtre ou de l'écran entier pour piloter l'éclairage d'ambiance en temps réel

### Éclairage réactif à l'audio

- Pilotage des couleurs des lumières à partir de l'entrée microphone via une analyse FFT
- Fenêtre de débogage optionnelle pour visualiser la forme d'onde et le spectre audio

### Scènes et planification

- Enregistrement et chargement de scènes nommées (préréglages de couleurs appliqués à plusieurs lumières)
- Planification de scènes pour une activation automatique à des moments précis de la journée

### Raccourcis clavier

- Attribution de raccourcis clavier globaux pour piloter l'éclairage sans les mains

### Zone de notification

- Réduction de l'application dans la zone de notification (systray)
- Activation/désactivation rapide et fermeture de l'application via le menu de la zone de notification

### Localisation

- Disponible en 6 langues : anglais, espagnol, chinois simplifié, français, allemand et portugais (Brésil)

## Technologies utilisées

| Crate | Objectif |
|-------|---------|
| [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) / [egui](https://github.com/emilk/egui) | Framework d'interface graphique (GUI) |
| [lifx-core](https://github.com/eminence/lifx) | Protocole LAN LIFX |
| [cpal](https://github.com/RustAudio/cpal) + [rustfft](https://github.com/ejmahler/RustFFT) | Capture audio et FFT |
| [xcap](https://github.com/niceChenGitH/xcap) | Capture d'écran |
| [rdev](https://github.com/Narsil/rdev) | Gestion globale des entrées clavier/souris |
| [tray-icon](https://github.com/niceChenGitH/tray-icon) | Zone de notification |
| [rust-i18n](https://github.com/longbridge/rust-i18n) | Localisation |

## Compilation

### Prérequis

- Chaîne d'outils [Rust](https://www.rust-lang.org/tools/install) (stable)
- Le dossier `data/` contenant `products.json` (inclus dans le dépôt ; intégré au moment de la compilation)

**Linux uniquement** — installez les bibliothèques système suivantes :

```bash
sudo apt install libasound2-dev libudev-dev libxtst-dev libevdev-dev libgtk-3-dev libxdo-dev
```

### Compiler

```bash
cargo build --release
```

### Exécuter

```bash
cargo run --release
```

Les journaux sont écrits dans `log/output.log`.

## Indicateurs de fonctionnalités (Feature Flags)

- `puffin` — Active le profileur [Puffin](https://github.com/EmbarkStudios/puffin) pour l'analyse des performances.

```bash
cargo run --release --features puffin
```

## Contribuer

Le dépôt inclut un hook de pré-validation (`pre-commit`) qui exécute `cargo fmt --check`, `cargo clippy` et `cargo test`. Pour l'activer :

```bash
git config core.hooksPath .githooks
```

## Retours

Rejoignez le serveur Discord [ici](https://discord.gg/TwqSeTTYqX) pour faire part de vos retours, signaler des bugs ou demander de nouvelles fonctionnalités.

## Remerciements

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Traductions

| Langue | Complète | Automatique |
|----------|-------------|------------| | Anglais | Oui | Non |
| Espagnol | Oui | Oui |
| Chinois (simplifié) | Oui | Oui |
| Français | Oui | Oui |
| Allemand | Oui | Oui |
| Portugais (Brésil) | Oui | Oui |