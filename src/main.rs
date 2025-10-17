use jetkvm_mcp_server::server::JetKvmServer;
use rmcp::{
    service::ServiceExt,
    transport::stdio,
};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let host = env::var("JETKVM_HOST").expect("JETKVM_HOST environment variable not set");
    let password = env::var("JETKVM_PASSWORD").expect("JETKVM_PASSWORD environment variable not set");

    let server = JetKvmServer::new(host, password);
    
    server.connect().await.map_err(|e| anyhow::anyhow!(e))?;
    
    tracing::info!("Connected to JetKVM device");

    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
