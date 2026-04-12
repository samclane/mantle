# Mantle

![exemplo de fluxo de trabalho](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Logo do Mantle](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

O Mantle é um aplicativo de desktop multiplataforma para descobrir e controlar lâmpadas inteligentes [LIFX](https://www.lifx.com/) através da sua rede local. Desenvolvido com Rust e [egui](https://github.com/emilk/egui), ele oferece gerenciamento de iluminação em tempo real, juntamente com recursos ambientais exclusivos, como sincronização de cores com a tela, iluminação reativa ao áudio, cenas salvas com agendamento, atalhos de teclado globais e um ícone na bandeja do sistema para acesso rápido. Renascido das cinzas do [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Lançamentos

Você pode baixar a versão mais recente [aqui](https://github.com/samclane/mantle/releases).

Compilações estão disponíveis para **Windows** (x86_64), **Linux** (x86_64) e **macOS** (Apple Silicon / aarch64). ## Capturas de Tela

![Mantle](res/screenshot.png)

## Recursos

### Descoberta e Controle de Luzes

- Descubra automaticamente lâmpadas LIFX na rede local
- Alternar ligar/desligar; ajustar matiz, saturação, brilho e temperatura (Kelvin) com controles deslizantes em tempo real
- Definir a duração da transição para mudanças de cor suaves
- Suporte a múltiplas zonas para fitas de LED, com controles por zona e de gradiente

### Agrupamento

- Organize as luzes em grupos
- Controle todas as luzes simultaneamente ou filtre/pesquise por nome

### Conta-gotas e Sincronização de Tela

- Selecione qualquer cor da sua tela usando a ferramenta de conta-gotas
- Calcule a média de cores de uma região da tela, de uma janela ou do monitor inteiro para controlar a iluminação ambiente em tempo real

### Iluminação Reativa ao Áudio

- Controle as cores das luzes a partir da entrada do microfone, utilizando análise FFT
- Janela opcional de depuração de forma de onda para visualizar o espectro de áudio

### Cenas e Agendamento

- Salve e carregue cenas nomeadas (predefinições de cores aplicadas a múltiplas luzes)
- Agende cenas para serem ativadas automaticamente em horários específicos do dia

### Atalhos de Teclado

- Atribua atalhos de teclado globais a ações de iluminação para um controle sem o uso das mãos

### Bandeja do Sistema

- Minimize o aplicativo para a bandeja do sistema (system tray)
- Alternar rapidamente o estado de ligar/desligar e sair do programa através do menu da bandeja

### Localização

- Disponível em 6 idiomas: Inglês, Espanhol, Chinês Simplificado, Francês, Alemão e Português (Brasil)

## Desenvolvido com

| Crate | Finalidade |
|-------|---------|
| [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) / [egui](https://github.com/emilk/egui) | Framework de GUI |
| [lifx-core](https://github.com/eminence/lifx) | Protocolo LIFX LAN |
| [cpal](https://github.com/RustAudio/cpal) + [rustfft](https://github.com/ejmahler/RustFFT) | Captura de áudio e FFT |
| [xcap](https://github.com/niceChenGitH/xcap) | Captura de tela |
| [rdev](https://github.com/Narsil/rdev) | Entrada global de teclado/mouse |
| [tray-icon](https://github.com/niceChenGitH/tray-icon) | Bandeja do sistema |
| [rust-i18n](https://github.com/longbridge/rust-i18n) | Localização |

## Compilação

### Pré-requisitos

- Toolchain do [Rust](https://www.rust-lang.org/tools/install) (estável)
- A pasta `data/` contendo o arquivo `products.json` (incluído no repositório; incorporado em tempo de compilação)

**Apenas Linux** — instale as seguintes bibliotecas de sistema:

```bash
sudo apt install libasound2-dev libudev-dev libxtst-dev libevdev-dev libgtk-3-dev libxdo-dev
```

### Compilar

```bash
cargo build --release
```

### Executar

```bash
cargo run --release
```

Os logs são gravados em `log/output.log`.

## Feature Flags

- `puffin` — Habilita o profiler [Puffin](https://github.com/EmbarkStudios/puffin) para análise de desempenho

```bash
cargo run --release --features puffin
```

## Contribuições

O repositório inclui um hook de pré-commit que executa `cargo fmt --check`, `cargo clippy` e `cargo test`. Para habilitá-lo:

```bash
git config core.hooksPath .githooks
```

## Feedback

Entre no servidor do Discord [aqui](https://discord.gg/TwqSeTTYqX) para enviar feedback, relatar bugs ou solicitar novos recursos.

## Agradecimentos

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Traduções

| Idioma | Completa | Automática |
|----------|-------------|------------| | Inglês  | Sim         | Não         |
| Espanhol  | Sim         | Sim        |
| Chinês (Simplificado) | Sim | Sim |
| Francês | Sim | Sim |
| Alemão | Sim | Sim |
| Português (Brasil) | Sim | Sim |