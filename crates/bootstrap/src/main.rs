use std::net::SocketAddr;
use tracing::info;

mod config;
mod factory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bootstrap=info".parse()?)
                .add_directive("tower_http=debug".parse()?),
        )
        .init();

    let config = config::Config::from_env();
    let app = factory::build_app(&config).await?;

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("🚀 Server running at http://{addr}");
    info!("📖 Scalar docs at http://{addr}/scalar");

    axum::serve(listener, app).await?;
    Ok(())
}
