use async_trait::async_trait;

#[async_trait]
pub trait Job: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self) -> anyhow::Result<()>;
}
