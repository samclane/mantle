# Mantle

![exemplo de fluxo de trabalho](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Logo do Mantle](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

O Mantle é um aplicativo de desktop para controlar lâmpadas LIFX, que renasceu das cinzas do [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Lançamentos

Você pode baixar o lançamento mais recente [aqui](https://github.com/samclane/mantle/releases).

**Nota:** No momento, estou gerando builds apenas para Windows, mas o projeto também deve compilar no Linux e no MacOS. Ainda estou configurando o GitHub Actions para isso.

## Capturas de Tela

![Mantle](res/screenshot.png)

## Recursos

- Monitorar o status das lâmpadas LIFX
- Controlar lâmpadas LIFX
- Controlar múltiplas lâmpadas
- Agrupar lâmpadas
- Controlar todas as lâmpadas
- Ferramenta conta-gotas
- Média da tela em tempo real para iluminação ambiente

## Compilação

Basta executar `cargo build --release` para compilar o projeto. Certifique-se de ter a pasta `data`, contendo o arquivo `products.json`.

## Feature Flags

- `puffin` - Habilita o profiler Puffin

## Feedback

Entre no servidor do Discord [aqui](https://discord.gg/TwqSeTTYqX) para enviar feedback, relatar bugs ou solicitar novos recursos. ## Agradecimentos

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## Traduções

| Idioma | Completa | Automática |
|----------|-------------|------------|
| Inglês  | Sim         | Não        |
| Espanhol  | Sim         | Sim        |
| Chinês (Simplificado) | Sim | Sim |
| Francês | Sim | Sim |
| Alemão | Sim | Sim |
| Português (Brasil) | Sim | Sim |