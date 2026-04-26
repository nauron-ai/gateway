use axum::extract::Multipart;
use bytes::Bytes;

use crate::error::GatewayError;
use crate::routes::load::response::LoadContextForm;

#[derive(Debug)]
pub struct UploadRequest {
    pub form: LoadContextForm,
    pub file: UploadedFile,
}

#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub bytes: Bytes,
    pub name: Option<String>,
    pub content_type: Option<String>,
}

impl UploadRequest {
    pub async fn parse(mut payload: Multipart) -> Result<Self, GatewayError> {
        let mut file: Option<UploadedFile> = None;
        let mut user_id = None;
        let mut dry_run = false;
        let mut context_id: Option<i32> = None;

        while let Some(field) = payload.next_field().await? {
            match field.name() {
                Some("file") => {
                    let filename = field.file_name().map(|s| s.to_string());
                    let content_type = field.content_type().map(|s| s.to_string());
                    let bytes = field.bytes().await?;
                    file = Some(UploadedFile {
                        bytes,
                        name: filename,
                        content_type,
                    });
                }
                Some("user_id") => {
                    user_id = Some(field.text().await?);
                }
                Some("dry_run") => {
                    let value = field.text().await?;
                    dry_run = value
                        .parse::<bool>()
                        .map_err(|_| GatewayError::InvalidField {
                            field: "dry_run".into(),
                            message: "expected true/false".into(),
                        })?;
                }
                Some("context_id") => {
                    let value = field.text().await?;
                    let parsed = value
                        .parse::<i32>()
                        .map_err(|_| GatewayError::InvalidField {
                            field: "context_id".into(),
                            message: "expected integer".into(),
                        })?;
                    if parsed <= 0 {
                        return Err(GatewayError::InvalidField {
                            field: "context_id".into(),
                            message: "must be positive".into(),
                        });
                    }
                    context_id = Some(parsed);
                }
                _ => {}
            }
        }

        let file = file.ok_or(GatewayError::MissingFile)?;
        if file.bytes.is_empty() {
            return Err(GatewayError::InvalidField {
                field: "file".into(),
                message: "uploaded file is empty".into(),
            });
        }
        let context_id = context_id.ok_or_else(|| GatewayError::InvalidField {
            field: "context_id".into(),
            message: "field is required".into(),
        })?;

        let metadata = LoadContextForm {
            file: file
                .name
                .clone()
                .unwrap_or_else(|| String::from("uploaded file")),
            context_id,
            user_id,
            dry_run: Some(dry_run),
        };

        Ok(Self {
            form: metadata,
            file,
        })
    }
}
