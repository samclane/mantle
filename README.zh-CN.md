# Mantle

![示例工作流](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle Logo](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md)

Mantle 是一款用于控制 LIFX 灯具的桌面应用程序，它脱胎于 [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel) 项目。

## 发布版本

您可以在[此处](https://github.com/samclane/mantle/releases)下载最新发布版本。

**注意：** 目前我仅针对 Windows 平台构建发布包，但该项目理论上也能在 Linux 和 MacOS 上成功构建。我仍在配置 GitHub Actions 以支持这些平台。

## 截图

![Mantle](res/screenshot.png)

## 功能特性

- 监控 LIFX 灯具状态
- 控制 LIFX 灯具
- 控制多盏灯具
- 灯具分组控制
- 控制所有灯具
- 吸管工具（取色器）
- 实时屏幕色彩平均采样，实现环境氛围照明效果

## 构建项目

只需运行 `cargo build --release` 即可构建该项目。请确保您拥有 `data` 文件夹，且其中包含 `products.json` 文件。

## 功能标志（Feature Flags）

- `puffin` - 启用 Puffin 性能分析器

## 反馈与建议

欢迎加入我们的 Discord 服务器（点击[此处](https://discord.gg/TwqSeTTYqX)加入），以便提供反馈、报告 Bug 或提出功能建议。

## 致谢

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## 语言翻译

| 语言       | 是否完成 | 是否为机翻 |
|------------|----------|------------|
| 英语 | 是 | 否 |
| 西班牙语 | 是 | 是 |
| 中文（简体） | 是 | 是 |
| 法语 | 是 | 是 |