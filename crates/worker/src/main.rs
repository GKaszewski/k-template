use std::sync::Arc;
use std::time::Duration;
use tracing::info;

mod config;
mod job;
mod jobs;
mod runner;

use jobs::ExampleJob;
use runner::JobRunner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("worker=info".parse()?),
        )
        .init();

    let config = config::WorkerConfig::from_env();
    info!("Worker starting");

    let _pool = adapters_sqlite::connect(&config.database_url).await?;
    adapters_sqlite::run_migrations(&_pool).await?;

    let interval = Duration::from_secs(config.example_job_interval_secs);
    let runner = JobRunner::new().register(Arc::new(ExampleJob), interval);

    info!("Worker running");
    runner.run().await;
    Ok(())
}
