# Mantle

![example workflow](https://github.com/samclane/mantle/actions/workflows/main.yml/badge.svg)

![Mantle Logo](./res/logo128.png)

Mantle is a desktop application for controlling LIFX lights, born from the ashes of [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel).

## Releases

You can download the latest release [here](https://github.com/samclane/mantle/releases).

**Note:** Right now I'm only building for Windows, but it should build on Linux and MacOS as well. Still getting GitHub Actions set up for that.

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

## MCP Server

Mantle includes an optional [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server that lets AI assistants control your LIFX lights.

### Building

```bash
cargo build --release --features mcp --bin mcp_server
```

### Configuration

Add the server to your MCP client config (Claude Desktop, Cursor, etc.):

```json
{
  "mcpServers": {
    "mantle": {
      "command": "path/to/target/release/mcp_server"
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `discover` | Find LIFX bulbs on the local network |
| `refresh` | Refresh cached bulb state (names, colors, power, groups) |
| `list_bulbs` | List all discovered bulbs with their current state |
| `list_groups` | List all bulb groups |
| `set_power` | Turn a bulb on or off by name |
| `set_color` | Set hue, saturation, brightness, and/or kelvin by name |
| `toggle_all_power` | Toggle power for every bulb on the network |

## Feature Flags

- `puffin` - Enables the Puffin profiler
- `mcp` - Enables the MCP server binary and dependencies

## Feedback

Join the Discord server [here](https://discord.gg/TwqSeTTYqX) to provide feedback, report bugs, or request features.

## Acknowledgements

- [`lifx_control_panel`](https://github.com/samclane/LIFX-Control-Panel)
- [`lifx-core`](https://github.com/eminence/lifx)
- [`lifxlan (Python)`](https://github.com/mclarkk/lifxlan)
- [`eframe_template`](https://github.com/emilk/eframe_template)
