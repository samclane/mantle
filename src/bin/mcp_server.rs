use mantle::mcp::MantleMcpServer;
use mantle::LifxManager;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("Starting Mantle MCP Server");

    let lifx_manager = tokio::task::spawn_blocking(|| {
        LifxManager::new().or_else(|_| {
            log::info!("Port 56700 in use, binding to an ephemeral port");
            LifxManager::with_port(0)
        }).expect("Failed to initialize LIFX manager")
    })
    .await?;

    let server = MantleMcpServer::new(lifx_manager);

    let service = server.serve(stdio()).await?;
    log::info!("MCP server running on stdio");
    service.waiting().await?;

    Ok(())
}
