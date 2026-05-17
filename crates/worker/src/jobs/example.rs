use async_trait::async_trait;
use tracing::info;
use crate::job::Job;

pub struct ExampleJob;

#[async_trait]
impl Job for ExampleJob {
    fn name(&self) -> &str { "example" }
    async fn run(&self) -> anyhow::Result<()> {
        info!("example job ran — replace with real work");
        Ok(())
    }
}
