use std::borrow::Cow;
use std::collections::HashMap;

use lifx_core::HSBK;
use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters, model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::device_info::BulbInfo;
use crate::device_manager::LifxManager;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BulbNameParams {
    /// Name of the bulb (case-insensitive partial match)
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SetPowerParams {
    /// Name of the bulb (case-insensitive partial match)
    pub name: String,
    /// true to turn on, false to turn off
    pub on: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SetColorParams {
    /// Name of the bulb (case-insensitive partial match)
    pub name: String,
    /// Hue angle in degrees (0-360)
    pub hue: Option<f64>,
    /// Saturation percentage (0-100)
    pub saturation: Option<f64>,
    /// Brightness percentage (0-100)
    pub brightness: Option<f64>,
    /// Color temperature in Kelvin (1500-9000)
    pub kelvin: Option<u16>,
    /// Transition duration in milliseconds
    pub duration_ms: Option<u32>,
}

#[derive(Clone)]
pub struct MantleMcpServer {
    lifx_manager: LifxManager,
    tool_router: ToolRouter<Self>,
}

fn find_bulb_by_name<'a>(bulbs: &'a HashMap<u64, BulbInfo>, name: &str) -> Option<&'a BulbInfo> {
    let name_lower = name.to_lowercase();
    bulbs.values().find(|b| {
        b.name_label()
            .map(|n| n.to_lowercase().contains(&name_lower))
            .unwrap_or(false)
    })
}

fn mcp_err(message: impl Into<String>) -> McpError {
    McpError {
        code: ErrorCode::INTERNAL_ERROR,
        message: Cow::Owned(message.into()),
        data: None,
    }
}

fn mcp_invalid_params(message: impl Into<String>) -> McpError {
    McpError {
        code: ErrorCode::INVALID_PARAMS,
        message: Cow::Owned(message.into()),
        data: None,
    }
}

fn format_hsbk(color: &HSBK) -> String {
    format!(
        "H:{:.0}° S:{:.0}% B:{:.0}% K:{}",
        color.hue as f64 / 65535.0 * 360.0,
        color.saturation as f64 / 65535.0 * 100.0,
        color.brightness as f64 / 65535.0 * 100.0,
        color.kelvin,
    )
}

#[tool_router]
impl MantleMcpServer {
    pub fn new(lifx_manager: LifxManager) -> Self {
        Self {
            lifx_manager,
            tool_router: Self::tool_router(),
        }
    }

    /// List all discovered LIFX bulbs on the network with their names, power
    /// states, colors, and group membership.
    #[tool(
        description = "List all discovered LIFX bulbs on the network with their names, power states, colors, and group membership"
    )]
    fn list_bulbs(&self) -> Result<CallToolResult, McpError> {
        let bulbs = self
            .lifx_manager
            .bulbs
            .lock()
            .map_err(|e| mcp_err(format!("Failed to lock bulbs: {e}")))?;

        if bulbs.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No bulbs discovered. Try running the 'discover' tool first.",
            )]));
        }

        let mut lines = Vec::new();
        for bulb in bulbs.values() {
            let name = bulb
                .name_label()
                .unwrap_or_else(|| format!("Unknown (ID: {})", bulb.target));
            let power = bulb
                .power_level
                .data
                .map(|p| if p > 0 { "On" } else { "Off" })
                .unwrap_or("Unknown");
            let color_str = bulb
                .get_color()
                .map(format_hsbk)
                .unwrap_or_else(|| "Unknown".to_string());
            let group = bulb.group_label().unwrap_or_else(|| "None".to_string());

            lines.push(format!(
                "- {name} | Power: {power} | Color: {color_str} | Group: {group}"
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    /// List all LIFX bulb groups on the network.
    #[tool(description = "List all LIFX bulb groups on the network")]
    fn list_groups(&self) -> Result<CallToolResult, McpError> {
        let groups = self.lifx_manager.get_groups();

        if groups.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No groups found. Try running 'discover' and 'refresh' first.",
            )]));
        }

        let lines: Vec<String> = groups.iter().map(|g| format!("- {}", g.label)).collect();
        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    /// Set the power state (on/off) of a bulb by name.
    #[tool(description = "Set the power state (on/off) of a bulb by name")]
    fn set_power(
        &self,
        Parameters(params): Parameters<SetPowerParams>,
    ) -> Result<CallToolResult, McpError> {
        let level = if params.on { u16::MAX } else { 0 };
        let bulbs = self
            .lifx_manager
            .bulbs
            .lock()
            .map_err(|e| mcp_err(format!("Failed to lock bulbs: {e}")))?;

        let bulb = find_bulb_by_name(&bulbs, &params.name).ok_or_else(|| {
            mcp_invalid_params(format!("No bulb found matching '{}'", params.name))
        })?;

        self.lifx_manager
            .set_power(&bulb, level)
            .map_err(|e| mcp_err(format!("Failed to set power: {e}")))?;

        let name = bulb.name_label().unwrap_or_else(|| "Unknown".to_string());
        let state = if params.on { "on" } else { "off" };
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Set '{name}' power to {state}"
        ))]))
    }

    /// Set the color of a bulb by name. All color fields are optional; only
    /// specified fields are changed while the rest keep their current values.
    /// Hue: 0-360 degrees, Saturation: 0-100%, Brightness: 0-100%, Kelvin: 1500-9000.
    #[tool(
        description = "Set the color of a bulb. Hue: 0-360 degrees, Saturation: 0-100%, Brightness: 0-100%, Kelvin: 1500-9000. Only specified fields are changed."
    )]
    fn set_color(
        &self,
        Parameters(params): Parameters<SetColorParams>,
    ) -> Result<CallToolResult, McpError> {
        let bulbs = self
            .lifx_manager
            .bulbs
            .lock()
            .map_err(|e| mcp_err(format!("Failed to lock bulbs: {e}")))?;

        let bulb = find_bulb_by_name(&bulbs, &params.name).ok_or_else(|| {
            mcp_invalid_params(format!("No bulb found matching '{}'", params.name))
        })?;

        let current = bulb.get_color().copied().unwrap_or(HSBK {
            hue: 0,
            saturation: 0,
            brightness: u16::MAX,
            kelvin: 3500,
        });

        let color = HSBK {
            hue: params
                .hue
                .map(|h| (h.clamp(0.0, 360.0) / 360.0 * 65535.0) as u16)
                .unwrap_or(current.hue),
            saturation: params
                .saturation
                .map(|s| (s.clamp(0.0, 100.0) / 100.0 * 65535.0) as u16)
                .unwrap_or(current.saturation),
            brightness: params
                .brightness
                .map(|b| (b.clamp(0.0, 100.0) / 100.0 * 65535.0) as u16)
                .unwrap_or(current.brightness),
            kelvin: params
                .kelvin
                .map(|k| k.clamp(1500, 9000))
                .unwrap_or(current.kelvin),
        };

        self.lifx_manager
            .set_color(&bulb, color, params.duration_ms)
            .map_err(|e| mcp_err(format!("Failed to set color: {e}")))?;

        let name = bulb.name_label().unwrap_or_else(|| "Unknown".to_string());
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Set '{name}' color to {}",
            format_hsbk(&color),
        ))]))
    }

    /// Toggle the power state of all bulbs on the network.
    #[tool(description = "Toggle the power state of all bulbs on the network")]
    fn toggle_all_power(&self) -> Result<CallToolResult, McpError> {
        self.lifx_manager
            .toggle_power()
            .map_err(|e| mcp_err(format!("Failed to toggle power: {e}")))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Toggled power for all bulbs",
        )]))
    }

    /// Discover LIFX bulbs on the local network via UDP broadcast. Run this
    /// first to populate the bulb list.
    #[tool(
        description = "Discover LIFX bulbs on the local network. Run this first to find available bulbs."
    )]
    async fn discover(&self) -> Result<CallToolResult, McpError> {
        let mut mgr = self.lifx_manager.clone();
        mgr.discover()
            .map_err(|e| mcp_err(format!("Discovery failed: {e}")))?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let count = self.lifx_manager.bulbs.lock().map(|b| b.len()).unwrap_or(0);

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Discovery complete. Found {count} bulb(s)."
        ))]))
    }

    /// Refresh the cached state of all known bulbs (names, colors, power
    /// levels, groups).
    #[tool(
        description = "Refresh the state of all known bulbs (updates names, colors, power levels, groups)"
    )]
    async fn refresh(&self) -> Result<CallToolResult, McpError> {
        self.lifx_manager
            .refresh()
            .map_err(|e| mcp_err(format!("Refresh failed: {e}")))?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(CallToolResult::success(vec![Content::text(
            "Bulb states refreshed successfully",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for MantleMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "Mantle LIFX smart bulb controller. Use 'discover' to find bulbs on \
             the network, then 'refresh' to fetch their state. After that you can \
             list_bulbs, set_power, set_color, and toggle_all_power."
                    .to_string(),
            )
    }
}
