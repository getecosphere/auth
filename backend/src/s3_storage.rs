//! Thin wrapper around aws-sdk-s3, pointed at whatever S3-compatible
//! endpoint this estate's storage.minio config declares (MinIO locally,
//! or any real S3-compatible service in remote mode -- see eco's
//! `eco install minio` / configure.sh's resolve_minio_s3_config). Only
//! used when config.storage_backend is S3; storage.rs is the caller.

use aws_sdk_s3::{
    config::{BehaviorVersion, Builder as S3ConfigBuilder, Credentials, Region},
    primitives::ByteStream,
    Client,
};

use crate::config::AppConfig;

/// force_path_style is required for MinIO -- it doesn't support
/// virtual-hosted-style bucket addressing (`bucket.endpoint/key`) the way
/// real AWS S3 does, only path-style (`endpoint/bucket/key`).
pub fn build_client(config: &AppConfig) -> Client {
    let credentials = Credentials::new(
        &config.s3_access_key,
        &config.s3_secret_key,
        None,
        None,
        "eco-configured",
    );
    let s3_config = S3ConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(config.s3_region.clone()))
        .endpoint_url(&config.s3_endpoint)
        .credentials_provider(credentials)
        .force_path_style(true)
        .build();
    Client::from_conf(s3_config)
}

/// S3 (unlike local disk) doesn't auto-create a bucket on first write --
/// called once at startup so the first real upload doesn't fail with
/// NoSuchBucket. Idempotent: a bucket that already exists is left as-is.
pub async fn ensure_bucket(client: &Client, bucket: &str) -> anyhow::Result<()> {
    if client.head_bucket().bucket(bucket).send().await.is_ok() {
        return Ok(());
    }
    client
        .create_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to create S3 bucket '{bucket}': {e}"))?;
    tracing::info!(bucket, "created S3 bucket");
    Ok(())
}

pub async fn put_object(
    client: &Client,
    bucket: &str,
    key: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> anyhow::Result<()> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(bytes))
        .content_type(content_type)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("S3 put_object failed for '{key}': {e}"))?;
    Ok(())
}

pub async fn get_object(client: &Client, bucket: &str, key: &str) -> anyhow::Result<Vec<u8>> {
    let output = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("S3 get_object failed for '{key}': {e}"))?;
    let bytes = output
        .body
        .collect()
        .await
        .map_err(|e| anyhow::anyhow!("failed to read S3 object body for '{key}': {e}"))?
        .into_bytes();
    Ok(bytes.to_vec())
}

pub async fn delete_object(client: &Client, bucket: &str, key: &str) -> anyhow::Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("S3 delete_object failed for '{key}': {e}"))?;
    Ok(())
}
