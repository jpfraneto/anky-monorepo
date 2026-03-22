use crate::config::Config;
use anyhow::Result;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::time::Duration;

/// Build an S3 client pointed at Cloudflare R2.
fn r2_client(config: &Config) -> Client {
    let endpoint = format!("https://{}.r2.cloudflarestorage.com", config.r2_account_id);
    let credentials = Credentials::new(
        &config.r2_access_key_id,
        &config.r2_secret_access_key,
        None,
        None,
        "r2-env",
    );
    let sdk_config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(&endpoint)
        .region(Region::new("auto"))
        .credentials_provider(credentials)
        .force_path_style(true)
        .build();
    Client::from_conf(sdk_config)
}

/// Check if R2 is configured (account_id + keys present).
pub fn is_configured(config: &Config) -> bool {
    !config.r2_account_id.is_empty()
        && !config.r2_access_key_id.is_empty()
        && !config.r2_secret_access_key.is_empty()
}

/// Generate a presigned PUT URL for uploading a file to R2.
pub async fn presigned_put_url(config: &Config, key: &str) -> Result<String> {
    let client = r2_client(config);
    let presign_config = PresigningConfig::builder()
        .expires_in(Duration::from_secs(3600))
        .build()?;
    let url = client
        .put_object()
        .bucket(&config.r2_bucket_name)
        .key(key)
        .content_type("audio/mp4")
        .presigned(presign_config)
        .await?;
    Ok(url.uri().to_string())
}

/// Build the public URL for an approved recording.
pub fn public_url(config: &Config, key: &str) -> String {
    let base = config.r2_public_url.trim_end_matches('/');
    format!("{}/{}", base, key)
}

/// Upload bytes directly to R2 (server-side upload).
pub async fn upload_bytes(
    config: &Config,
    key: &str,
    bytes: &[u8],
    content_type: &str,
) -> Result<()> {
    let client = r2_client(config);
    let body = aws_sdk_s3::primitives::ByteStream::from(bytes.to_vec());
    client
        .put_object()
        .bucket(&config.r2_bucket_name)
        .key(key)
        .body(body)
        .content_type(content_type)
        .send()
        .await?;
    Ok(())
}

/// Download an object from R2 as bytes.
pub async fn get_object_bytes(config: &Config, key: &str) -> Result<Vec<u8>> {
    let client = r2_client(config);
    let resp = client
        .get_object()
        .bucket(&config.r2_bucket_name)
        .key(key)
        .send()
        .await?;
    let bytes = resp.body.collect().await?.to_vec();
    Ok(bytes)
}
