use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use object_store::{ObjectStore, path::Path, Error as OsError};
use domain::errors::DomainError;
use domain::ports::{DataStream, StorageReader, StorageWriter};

pub struct ObjectStorageAdapter {
    store: Arc<dyn ObjectStore>,
    prefix: String,
}

impl ObjectStorageAdapter {
    pub fn new(store: Arc<dyn ObjectStore>, prefix: impl Into<String>) -> Result<Self, DomainError> {
        let prefix = prefix.into();
        if !prefix.is_empty() {
            validate_key(&prefix)?;
        }
        Ok(Self { store, prefix })
    }

    fn path(&self, key: &str) -> Path {
        if self.prefix.is_empty() {
            Path::from(key)
        } else {
            Path::from(format!("{}/{key}", self.prefix))
        }
    }
}

fn map_err(e: OsError, key: &str) -> DomainError {
    match e {
        OsError::NotFound { .. } => DomainError::NotFound(key.to_string()),
        e => DomainError::Internal(e.to_string()),
    }
}

fn validate_key(key: &str) -> Result<(), DomainError> {
    if key.is_empty() {
        return Err(DomainError::Validation("storage key must not be empty".into()));
    }
    if key.starts_with('/') {
        return Err(DomainError::Validation(
            format!("storage key must not start with '/': {key}"),
        ));
    }
    if key.split('/').any(|seg| seg == ".." || seg == ".") {
        return Err(DomainError::Validation(
            format!("storage key contains invalid path segment: {key}"),
        ));
    }
    Ok(())
}

#[async_trait]
impl StorageWriter for ObjectStorageAdapter {
    async fn put(&self, key: &str, data: DataStream) -> Result<(), DomainError> {
        validate_key(key)?;
        let path = self.path(key);
        let mut upload = self
            .store
            .put_multipart(&path)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let mut stream = data;
        while let Some(result) = stream.next().await {
            match result {
                Ok(bytes) => {
                    if let Err(e) = upload.put_part(bytes.into()).await {
                        let _ = upload.abort().await;
                        return Err(DomainError::Internal(e.to_string()));
                    }
                }
                Err(e) => {
                    let _ = upload.abort().await;
                    return Err(e);
                }
            }
        }
        upload.complete().await.map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), DomainError> {
        validate_key(key)?;
        let path = self.path(key);
        match self.store.delete(&path).await {
            Ok(()) => Ok(()),
            Err(OsError::NotFound { .. }) => Ok(()),
            Err(e) => Err(DomainError::Internal(e.to_string())),
        }
    }
}

#[async_trait]
impl StorageReader for ObjectStorageAdapter {
    async fn get(&self, key: &str) -> Result<DataStream, DomainError> {
        validate_key(key)?;
        let path = self.path(key);
        let result = self
            .store
            .get(&path)
            .await
            .map_err(|e| map_err(e, key))?;
        let s = result
            .into_stream()
            .map(|r| r.map_err(|e| DomainError::Internal(e.to_string())));
        Ok(Box::pin(s))
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, DomainError> {
        if let Some(p) = prefix {
            validate_key(p)?;
        }
        let list_prefix = match (prefix, self.prefix.is_empty()) {
            (Some(p), false) => Some(Path::from(format!("{}/{p}", self.prefix))),
            (Some(p), true) => Some(Path::from(p)),
            (None, false) => Some(Path::from(self.prefix.as_str())),
            (None, true) => None,
        };

        let mut result = Vec::new();
        let mut stream = self.store.list(list_prefix.as_ref());
        while let Some(meta) = stream.next().await {
            let meta = meta.map_err(|e| DomainError::Internal(e.to_string()))?;
            let key = meta.location.to_string();
            let stripped = if !self.prefix.is_empty() {
                key.strip_prefix(&format!("{}/", self.prefix))
                    .ok_or_else(|| DomainError::Internal(format!(
                        "listed key '{key}' does not start with expected prefix '{}'",
                        self.prefix
                    )))?
                    .to_string()
            } else {
                key
            };
            result.push(stripped);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::ports::{StorageReader, StorageWriter};
    use futures::stream;
    use object_store::memory::InMemory;

    fn make_adapter() -> ObjectStorageAdapter {
        ObjectStorageAdapter::new(Arc::new(InMemory::new()), "test").unwrap()
    }

    fn one_shot(data: &'static [u8]) -> DataStream {
        Box::pin(stream::once(async move { Ok(Bytes::from(data)) }))
    }

    #[tokio::test]
    async fn put_get_roundtrip() {
        let a = make_adapter();
        a.put("hello.txt", one_shot(b"world")).await.unwrap();
        let mut s = a.get("hello.txt").await.unwrap();
        let mut out = Vec::new();
        while let Some(chunk) = s.next().await {
            out.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(out, b"world");
    }

    #[tokio::test]
    async fn get_missing_is_not_found() {
        let a = make_adapter();
        assert!(matches!(a.get("nope.txt").await, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let a = make_adapter();
        a.delete("nope.txt").await.unwrap();
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let a = make_adapter();
        a.put("file.txt", one_shot(b"data")).await.unwrap();
        a.delete("file.txt").await.unwrap();
        assert!(matches!(a.get("file.txt").await, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_returns_keys_under_prefix() {
        let a = make_adapter();
        a.put("docs/readme.txt", one_shot(b"x")).await.unwrap();
        a.put("docs/guide.txt", one_shot(b"y")).await.unwrap();
        a.put("other/file.txt", one_shot(b"z")).await.unwrap();
        let keys = a.list(Some("docs")).await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().any(|k| k.ends_with("readme.txt")));
        assert!(keys.iter().any(|k| k.ends_with("guide.txt")));
    }

    #[tokio::test]
    async fn list_none_returns_all() {
        let a = make_adapter();
        a.put("a.txt", one_shot(b"1")).await.unwrap();
        a.put("b.txt", one_shot(b"2")).await.unwrap();
        let keys = a.list(None).await.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[tokio::test]
    async fn rejects_empty_key() {
        let a = make_adapter();
        assert!(matches!(a.put("", one_shot(b"x")).await, Err(DomainError::Validation(_))));
        assert!(matches!(a.get("").await, Err(DomainError::Validation(_))));
        assert!(matches!(a.delete("").await, Err(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn rejects_absolute_key() {
        let a = make_adapter();
        assert!(matches!(
            a.put("/etc/passwd", one_shot(b"x")).await,
            Err(DomainError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let a = make_adapter();
        assert!(matches!(a.get("../escape").await, Err(DomainError::Validation(_))));
        assert!(matches!(a.get("a/../../../etc").await, Err(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn rejects_dot_segment() {
        let a = make_adapter();
        assert!(matches!(
            a.put("./file.txt", one_shot(b"x")).await,
            Err(DomainError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn rejects_invalid_list_prefix() {
        let a = make_adapter();
        assert!(matches!(a.list(Some("")).await, Err(DomainError::Validation(_))));
        assert!(matches!(a.list(Some("../escape")).await, Err(DomainError::Validation(_))));
    }

    #[tokio::test]
    async fn put_overwrites_existing() {
        let a = make_adapter();
        a.put("file.txt", one_shot(b"version1")).await.unwrap();
        a.put("file.txt", one_shot(b"version2")).await.unwrap();
        let mut s = a.get("file.txt").await.unwrap();
        let mut out = Vec::new();
        while let Some(chunk) = s.next().await {
            out.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(out, b"version2");
    }

    #[tokio::test]
    async fn list_returns_exact_key_paths() {
        let a = make_adapter();
        a.put("docs/readme.txt", one_shot(b"x")).await.unwrap();
        let mut keys = a.list(Some("docs")).await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["docs/readme.txt"]);
    }

    #[tokio::test]
    async fn put_bytes_get_bytes_roundtrip() {
        let a = make_adapter();
        a.put_bytes("data.bin", Bytes::from("hello bytes")).await.unwrap();
        let got = a.get_bytes("data.bin").await.unwrap();
        assert_eq!(got.as_ref(), b"hello bytes");
    }

    #[tokio::test]
    async fn get_bytes_missing_is_not_found() {
        let a = make_adapter();
        assert!(matches!(a.get_bytes("nope.bin").await, Err(DomainError::NotFound(_))));
    }

    #[test]
    fn new_rejects_traversal_prefix() {
        let result = ObjectStorageAdapter::new(Arc::new(InMemory::new()), "../evil");
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn new_rejects_absolute_prefix() {
        let result = ObjectStorageAdapter::new(Arc::new(InMemory::new()), "/root");
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }

    #[test]
    fn new_accepts_empty_prefix() {
        assert!(ObjectStorageAdapter::new(Arc::new(InMemory::new()), "").is_ok());
    }

    #[test]
    fn new_accepts_valid_prefix() {
        assert!(ObjectStorageAdapter::new(Arc::new(InMemory::new()), "my-bucket/data").is_ok());
    }
}
