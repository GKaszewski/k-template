use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use crate::job::Job;

pub struct JobRunner {
    jobs: Vec<(Arc<dyn Job>, Duration)>,
}

impl JobRunner {
    pub fn new() -> Self { Self { jobs: vec![] } }

    pub fn register(mut self, job: Arc<dyn Job>, interval: Duration) -> Self {
        self.jobs.push((job, interval));
        self
    }

    pub async fn run(self) {
        let handles: Vec<_> = self.jobs.into_iter().map(|(job, interval)| {
            tokio::spawn(async move {
                loop {
                    info!(job = job.name(), "running job");
                    if let Err(e) = job.run().await {
                        error!(job = job.name(), error = %e, "job failed");
                    }
                    tokio::time::sleep(interval).await;
                }
            })
        }).collect();
        for handle in handles { let _ = handle.await; }
    }
}

impl Default for JobRunner { fn default() -> Self { Self::new() } }
