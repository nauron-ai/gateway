use aws_config::BehaviorVersion;
use aws_sdk_s3::client::Client;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Credentials, Region};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::head_bucket::HeadBucketError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::time::{Duration, timeout};

use crate::config::StorageSettings;

const STORAGE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct StorageClient {
    client: Client,
}

impl StorageClient {
    pub async fn new(settings: StorageSettings) -> Result<Self, StorageError> {
        let creds = Credentials::new(
            settings.access_key,
            settings.secret_key,
            None,
            None,
            "gateway",
        );
        let loader = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(Region::new(settings.region))
            .endpoint_url(settings.endpoint);
        let shared = loader.load().await;
        let config = S3ConfigBuilder::from(&shared)
            .force_path_style(settings.force_path_style)
            .build();
        Ok(Self {
            client: Client::from_conf(config),
        })
    }

    pub async fn upload(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
        content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(body));
        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }
        request.send().await.map_err(StorageError::from)?;
        Ok(())
    }

    pub async fn download(&self, bucket: &str, key: &str) -> Result<Bytes, StorageError> {
        let operation = async {
            let response = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(StorageError::from)?;
            let mut reader = response.body.into_async_read();
            let mut buffer = Vec::new();
            reader
                .read_to_end(&mut buffer)
                .await
                .map_err(StorageError::ReadBody)?;
            Ok(Bytes::from(buffer))
        };

        timeout(STORAGE_DOWNLOAD_TIMEOUT, operation)
            .await
            .map_err(|_| StorageError::DownloadTimeout(STORAGE_DOWNLOAD_TIMEOUT))?
    }

    pub async fn check_bucket(&self, bucket: &str) -> Result<(), StorageError> {
        self.client
            .head_bucket()
            .bucket(bucket)
            .send()
            .await
            .map_err(StorageError::from)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("s3 upload failed: {0}")]
    Put(#[from] SdkError<PutObjectError>),
    #[error("s3 download failed: {0}")]
    Get(#[from] SdkError<GetObjectError>),
    #[error("s3 download timed out after {}s", .0.as_secs())]
    DownloadTimeout(Duration),
    #[error("s3 head bucket failed: {0}")]
    Head(#[from] SdkError<HeadBucketError>),
    #[error("failed to read s3 object body: {0}")]
    ReadBody(#[from] std::io::Error),
}
