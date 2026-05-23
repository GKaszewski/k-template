use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{self, BoxStream, StreamExt};
use crate::errors::DomainError;

pub type DataStream = BoxStream<'static, Result<Bytes, DomainError>>;

/// Read operations on object storage. Keys are full paths relative to the adapter root.
#[async_trait]
pub trait StorageReader: Send + Sync {
    /// Returns the content of `key` as a stream. Returns `DomainError::NotFound` if absent.
    async fn get(&self, key: &str) -> Result<DataStream, DomainError>;

    /// Lists all keys whose path begins with `prefix`, or all keys when `prefix` is `None`.
    /// Returned keys are **full paths from the adapter root**, not relative to `prefix`.
    /// Example: `list(Some("docs"))` returns `["docs/readme.txt"]`, not `["readme.txt"]`.
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, DomainError>;

    /// Convenience: reads the entire content of `key` into memory. Wraps `get`.
    async fn get_bytes(&self, key: &str) -> Result<Bytes, DomainError> {
        let mut stream = self.get(key).await?;
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = stream.next().await {
            buf.extend_from_slice(&chunk?);
        }
        Ok(Bytes::from(buf))
    }
}

/// Write operations on object storage.
#[async_trait]
pub trait StorageWriter: Send + Sync {
    /// Stores `data` at `key`. Overwrites any existing content at that key silently.
    async fn put(&self, key: &str, data: DataStream) -> Result<(), DomainError>;

    /// Deletes `key`. Returns `Ok(())` even if the key does not exist (idempotent).
    async fn delete(&self, key: &str) -> Result<(), DomainError>;

    /// Convenience: stores an in-memory buffer at `key`. Wraps `put`.
    async fn put_bytes(&self, key: &str, data: Bytes) -> Result<(), DomainError> {
        self.put(key, Box::pin(stream::once(async move { Ok(data) }))).await
    }
}

/// Combined read + write storage interface.
///
/// **Usage note:** `Arc<dyn StoragePort>` is the intended DI type everywhere.
/// `StorageReader` and `StorageWriter` exist for implementation clarity, but Rust does not
/// support narrowing `Arc<dyn StoragePort>` to `Arc<dyn StorageReader>` at runtime.
/// Inject `Arc<dyn StoragePort>` into constructors and pass `.clone()` from the factory.
pub trait StoragePort: StorageReader + StorageWriter {}
impl<T: StorageReader + StorageWriter> StoragePort for T {}
