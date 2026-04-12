# Mantle

![示例工作流](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle Logo](./res/logo128.png)

🌐 [English](README.md) | [Español](README.es.md) | [简体中文](README.zh-CN.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Português](README.pt-BR.md)

Mantle 是一款跨平台的桌面应用程序，用于通过本地网络发现并控制 [LIFX](https://www.lifx.com/) 智能灯具。该应用基于 Rust 和 [egui](https://github.com/emilk/egui) 构建，不仅提供实时的灯光管理功能，还包含一系列独特的氛围特效，例如屏幕色彩同步、音频律动灯光、支持定时调度的预设场景、全局键盘快捷键，以及用于快速访问的系统托盘图标。Mantle 脱胎于 [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel) 项目，可谓浴火重生之作。

## 发布版本

您可在此处下载最新发布版本：[点击下载](https://github.com/samclane/mantle/releases)。

目前已提供适用于 **Windows** (x86_64)、**Linux** (x86_64) 和 **macOS** (Apple Silicon / aarch64) 平台的构建版本。 ## 截图

![Mantle](res/screenshot.png)

## 功能特性

### 灯光发现与控制

- 自动发现本地网络中的 LIFX 灯泡
- 切换电源、实时调节色相/饱和度/亮度/色温（开尔文值）
- 设置过渡时长，实现平滑的色彩变化效果
- 支持多区域控制（Multizone），专为灯带设计，可进行逐区调节及渐变控制

### 分组管理

- 将灯光归类至不同组别
- 一键控制所有灯光，或按名称进行筛选与搜索

### 吸管工具与屏幕同步

- 利用吸管工具，从屏幕任意位置拾取颜色
- 对屏幕特定区域、窗口或整个显示器进行色彩平均采样，实时驱动环境灯光效果

### 音频响应灯光

- 基于麦克风输入及 FFT（快速傅里叶变换）分析，实时驱动灯光色彩变化
- 提供可选的波形调试窗口，用于可视化音频频谱

### 场景与日程安排

- 保存并加载命名场景（跨多个灯光的色彩预设）
- 设置日程计划，让指定场景在特定时间自动激活

### 键盘快捷键

- 绑定全局热键，通过键盘操作即可免手控地执行灯光指令

### 系统托盘

- 支持最小化至系统托盘区域
- 通过托盘菜单快速切换电源状态或退出程序

### 本地化支持

- 提供 6 种语言版本：英语、西班牙语、简体中文、法语、德语及葡萄牙语（巴西）

## 技术栈

| Crate（库） | 用途 |
|-------|---------|
| [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) / [egui](https://github.com/emilk/egui) | GUI 用户界面框架 |
| [lifx-core](https://github.com/eminence/lifx) | LIFX 局域网通信协议 |
| [cpal](https://github.com/RustAudio/cpal) + [rustfft](https://github.com/ejmahler/RustFFT) | 音频捕获与 FFT 分析 |
| [xcap](https://github.com/niceChenGitH/xcap) | 屏幕截图与捕获 |
| [rdev](https://github.com/Narsil/rdev) | 全局键盘/鼠标输入监听 |
| [tray-icon](https://github.com/niceChenGitH/tray-icon) | 系统托盘 |
| [rust-i18n](https://github.com/longbridge/rust-i18n) | 本地化 |

## 构建

### 前置条件

- [Rust](https://www.rust-lang.org/tools/install) 工具链（稳定版）
- 包含 `products.json` 文件的 `data/` 目录（已包含在代码仓库中；会在编译时嵌入）

**仅限 Linux** —— 请安装以下系统库：

```bash
sudo apt install libasound2-dev libudev-dev libxtst-dev libevdev-dev libgtk-3-dev libxdo-dev
```

### 编译

```bash
cargo build --release
```

### 运行

```bash
cargo run --release
```

日志将写入 `log/output.log` 文件中。

## 功能标志（Feature Flags）

- `puffin` —— 启用 [Puffin](https://github.com/EmbarkStudios/puffin) 性能分析器，用于性能分析

```bash
cargo run --release --features puffin
```

## 贡献

本代码仓库包含一个 `pre-commit` 钩子（hook），用于自动运行 `cargo fmt --check`、`cargo clippy` 和 `cargo test` 命令。若要启用该钩子，请执行：

```bash
git config core.hooksPath .githooks
```

## 反馈

欢迎加入我们的 Discord 服务器 [（点击此处）](https://discord.gg/TwqSeTTYqX)，以便提供反馈、报告 Bug 或提出功能建议。

## 致谢

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
- [`tabler icons`](https://tabler.io/icons)

## 翻译

| 语言 | 是否完成 | 是否为机翻 |
|----------|-------------|------------| | 英语 | 是 | 否 |
| 西班牙语 | 是 | 是 |
| 中文（简体） | 是 | 是 |
| 法语 | 是 | 是 |
| 德语 | 是 | 是 |
| 葡萄牙语（巴西） | 是 | 是 |