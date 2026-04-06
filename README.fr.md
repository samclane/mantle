# Mantle

![Exemple de flux de travail](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Logo de Mantle](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

Mantle est une application de bureau permettant de contrôler les ampoules LIFX ; elle est née des cendres de [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Versions

Vous pouvez télécharger la dernière version [ici](https://github.com/samclane/mantle/releases).

**Note :** Pour l'instant, je ne génère des binaires que pour Windows, mais le projet devrait également pouvoir être compilé sous Linux et MacOS. Je suis encore en train de configurer GitHub Actions pour prendre en charge ces plateformes.

## Captures d'écran

![Mantle](res/screenshot.png)

## Fonctionnalités

- Surveillance de l'état des ampoules LIFX
- Contrôle des ampoules LIFX
- Contrôle de plusieurs ampoules simultanément
- Regroupement d'ampoules
- Contrôle de toutes les ampoules
- Outil pipette
- Calcul de la couleur moyenne de l'écran en temps réel pour un éclairage d'ambiance

## Compilation

Exécutez simplement la commande `cargo build --release` pour compiler le projet. Assurez-vous de disposer du dossier `data`, contenant le fichier `products.json`.

## Indicateurs de fonctionnalités (Feature Flags)

- `puffin` – Active le profileur Puffin

## Retours

Rejoignez le serveur Discord [ici](https://discord.gg/TwqSeTTYqX) pour faire part de vos commentaires, signaler des bugs ou suggérer de nouvelles fonctionnalités.

## Remerciements

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Traductions

| Langue | Complète | Est-ce une machine ? |
|----------|-------------|------------|
| Anglais  | Oui         | Non        |
| Espagnol | Oui         | Oui        |
| Chinois (simplifié) | Oui | Oui |
| Français | Oui | Oui |
| Allemand | Oui | Oui |