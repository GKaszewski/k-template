use std::sync::Arc;
use anyhow::{Context, Result};
use object_store::ObjectStore;
use object_store::local::LocalFileSystem;

/// All storage configuration. Populate once via `from_env()` and pass
/// explicitly to `build_store` and `ObjectStorageAdapter::new`.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend: String,
    pub prefix: String,
    // local backend:
    pub local_path: Option<String>,
    // s3/minio backend:
    pub s3_endpoint: Option<String>,
    pub s3_access_key_id: Option<String>,
    pub s3_secret_access_key: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    // gcs backend:
    pub gcs_bucket: Option<String>,
}

impl StorageConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            backend: std::env::var("STORAGE_BACKEND")
                .context("STORAGE_BACKEND must be set (local, s3, gcs)")?,
            prefix: std::env::var("STORAGE_PREFIX").unwrap_or_default(),
            local_path: std::env::var("STORAGE_PATH").ok(),
            s3_endpoint: std::env::var("S3_ENDPOINT").ok(),
            s3_access_key_id: std::env::var("S3_ACCESS_KEY_ID").ok(),
            s3_secret_access_key: std::env::var("S3_SECRET_ACCESS_KEY").ok(),
            s3_bucket: std::env::var("S3_BUCKET").ok(),
            s3_region: std::env::var("S3_REGION").ok(),
            gcs_bucket: std::env::var("GCS_BUCKET").ok(),
        })
    }
}

pub fn build_store(config: &StorageConfig) -> Result<Arc<dyn ObjectStore>> {
    match config.backend.as_str() {
        "local" => {
            let path = config.local_path.as_deref()
                .context("STORAGE_PATH must be set when STORAGE_BACKEND=local")?;
            std::fs::create_dir_all(path)
                .with_context(|| format!("failed to create storage dir: {path}"))?;
            let store = LocalFileSystem::new_with_prefix(path)?;
            Ok(Arc::new(store))
        }
        #[cfg(feature = "s3")]
        "s3" => {
            use object_store::aws::AmazonS3Builder;
            let store = AmazonS3Builder::new()
                .with_endpoint(
                    config.s3_endpoint.as_deref().context("S3_ENDPOINT must be set")?,
                )
                .with_access_key_id(
                    config.s3_access_key_id.as_deref()
                        .context("S3_ACCESS_KEY_ID must be set")?,
                )
                .with_secret_access_key(
                    config.s3_secret_access_key.as_deref()
                        .context("S3_SECRET_ACCESS_KEY must be set")?,
                )
                .with_bucket_name(
                    config.s3_bucket.as_deref().context("S3_BUCKET must be set")?,
                )
                .with_region(config.s3_region.as_deref().unwrap_or("us-east-1"))
                .with_allow_http(true)
                .build()?;
            Ok(Arc::new(store))
        }
        #[cfg(feature = "gcs")]
        "gcs" => {
            use object_store::gcp::GoogleCloudStorageBuilder;
            let store = GoogleCloudStorageBuilder::new()
                .with_bucket_name(
                    config.gcs_bucket.as_deref().context("GCS_BUCKET must be set")?,
                )
                .build()?;
            Ok(Arc::new(store))
        }
        other => anyhow::bail!(
            "unknown STORAGE_BACKEND={other:?}; compiled features: local{}{}",
            if cfg!(feature = "s3") { ", s3" } else { "" },
            if cfg!(feature = "gcs") { ", gcs" } else { "" },
        ),
    }
}
