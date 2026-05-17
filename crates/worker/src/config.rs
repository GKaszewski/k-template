#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub example_job_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            example_job_interval_secs: std::env::var("EXAMPLE_JOB_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }
}
