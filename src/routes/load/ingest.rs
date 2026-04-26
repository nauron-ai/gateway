use std::io::{Cursor, Read};
use std::sync::Arc;

use bytes::Bytes;
use uuid::Uuid;
use zip::read::ZipArchive;

use crate::config::AppConfig;
use crate::db::files::{AttachContextFileParams, CreateFileParams, FileOrigin, FileRecord};
use crate::error::GatewayError;
use crate::job_mode::JobLaunchMode;
use crate::state::AppState;

use super::form::UploadedFile;
use super::helpers::{compute_sha256, sanitize_filename};
use super::response::{LoadEntry, TopicInfo, UploadedLocation};

const ZIP_MIME_TYPES: [&str; 2] = ["application/zip", "application/x-zip-compressed"];
const OOXML_MIME_TYPES: [&str; 10] = [
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-word.document.macroenabled.12",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
    "application/vnd.ms-word.template.macroenabled.12",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-excel.sheet.macroenabled.12",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
    "application/vnd.ms-excel.template.macroenabled.12",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.ms-powerpoint.presentation.macroenabled.12",
];
const OOXML_EXTENSIONS: [&str; 10] = [
    "docx", "docm", "dotx", "dotm", "xlsx", "xlsm", "xltx", "xltm", "pptx", "pptm",
];

pub(super) fn is_zip_upload(file: &UploadedFile) -> bool {
    if is_ooxml_upload(file) {
        return false;
    }
    let ct_is_zip = file.content_type.as_deref().is_some_and(|content_type| {
        ZIP_MIME_TYPES
            .iter()
            .any(|mime| content_type.eq_ignore_ascii_case(mime))
    });
    let magic_is_zip = file.bytes.len() > 4 && &file.bytes[..4] == b"PK\x03\x04";
    ct_is_zip || magic_is_zip
}

fn is_ooxml_upload(file: &UploadedFile) -> bool {
    let mime_is_ooxml = file.content_type.as_deref().is_some_and(|content_type| {
        OOXML_MIME_TYPES
            .iter()
            .any(|mime| content_type.eq_ignore_ascii_case(mime))
    });
    let extension_is_ooxml = file.name.as_deref().is_some_and(ooxml_extension_in_name);
    mime_is_ooxml || extension_is_ooxml
}

fn ooxml_extension_in_name(name: &str) -> bool {
    if let Some((_, extension)) = name.rsplit_once('.') {
        OOXML_EXTENSIONS
            .iter()
            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    } else {
        false
    }
}

pub(super) fn extract_archive_items(file: &UploadedFile) -> Result<Vec<WorkItem>, GatewayError> {
    let mut archive = ZipArchive::new(Cursor::new(file.bytes.clone().to_vec()))
        .map_err(|err| GatewayError::Archive(err.to_string()))?;
    let mut items = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| GatewayError::Archive(err.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .map_err(|err| GatewayError::Archive(err.to_string()))?;

        if buffer.is_empty() {
            continue;
        }

        let bytes = Bytes::from(buffer);
        let safe_path = entry
            .enclosed_name()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.name().to_string());
        let file_name = entry
            .enclosed_name()
            .and_then(|path| path.file_name().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_else(|| safe_path.clone());

        let uploaded = UploadedFile {
            bytes,
            name: Some(file_name),
            content_type: None,
        };
        items.push(WorkItem::new(
            uploaded,
            FileOrigin::ArchiveEntry,
            Some(safe_path),
        ));
    }

    if items.is_empty() {
        return Err(GatewayError::InvalidField {
            field: "file".into(),
            message: "archive contains no files".into(),
        });
    }

    Ok(items)
}

pub(super) async fn upsert_file_record(
    state: &Arc<AppState>,
    file: &UploadedFile,
    sha_bytes: &[u8],
    ingest_key: &str,
) -> Result<(FileRecord, bool), GatewayError> {
    if let Some(record) = state.file_repo.find_by_hash(sha_bytes).await? {
        return Ok((record, false));
    }

    state
        .storage
        .upload(
            &state.config.input_bucket,
            ingest_key,
            file.bytes.clone(),
            file.content_type.as_deref(),
        )
        .await?;

    let record = state
        .file_repo
        .create_or_get_by_hash(CreateFileParams {
            sha256: sha_bytes,
            size_bytes: file.bytes.len() as i64,
            mime: file.content_type.as_deref(),
            storage_bucket: &state.config.input_bucket,
            storage_key: ingest_key,
        })
        .await?;
    Ok((record, true))
}

pub(super) fn build_metadata(name: &str, bytes: &Bytes) -> (Vec<u8>, String, String) {
    let sha_bytes = compute_sha256(bytes);
    let sha_hex = hex::encode(&sha_bytes);
    let sanitized = sanitize_filename(name);
    (sha_bytes, sha_hex, sanitized)
}

pub(super) fn build_entry(params: BuildEntryParams<'_>) -> LoadEntry {
    let BuildEntryParams {
        context_id,
        job_id,
        job_mode,
        file_record,
        sha_hex,
        deduplicated,
        original_path,
        config,
        pipeline_id,
    } = params;
    LoadEntry {
        job_id,
        doc_id: file_record.doc_id.unwrap_or(job_id),
        pipeline_id,
        context_id,
        file_id: file_record.id,
        sha256_hex: sha_hex,
        deduplicated,
        job_mode,
        source: UploadedLocation {
            bucket: file_record.storage_bucket.clone(),
            key: file_record.storage_key.clone(),
        },
        topics: TopicInfo {
            progress: config.progress_topic.clone(),
            result: config.result_topic.clone(),
            rdf_progress: config.rdf_progress_topic.clone(),
            rdf_result: config.rdf_result_topic.clone(),
        },
        pipeline_status_url: format!("/v1/pipelines/{pipeline_id}"),
        job_status_url: Some(format!("/v1/jobs/{job_id}")),
        original_path,
    }
}

pub(super) struct BuildEntryParams<'a> {
    pub context_id: i32,
    pub job_id: Uuid,
    pub job_mode: JobLaunchMode,
    pub file_record: &'a FileRecord,
    pub sha_hex: String,
    pub deduplicated: bool,
    pub original_path: Option<String>,
    pub config: &'a AppConfig,
    pub pipeline_id: Uuid,
}

pub(super) struct WorkItem {
    pub file: UploadedFile,
    pub origin: FileOrigin,
    pub original_path: Option<String>,
}

impl WorkItem {
    pub fn new(file: UploadedFile, origin: FileOrigin, original_path: Option<String>) -> Self {
        Self {
            file,
            origin,
            original_path,
        }
    }

    pub async fn attach_to_context(
        &self,
        state: &Arc<AppState>,
        context_id: i32,
        file_record: &FileRecord,
    ) -> Result<Uuid, GatewayError> {
        let context_file = state
            .file_repo
            .attach_to_context(AttachContextFileParams {
                context_id,
                file_id: file_record.id,
                pipeline_id: Uuid::new_v4(),
                origin: self.origin,
                original_name: self.file.name.as_deref().unwrap_or("document.bin"),
                original_path: self.original_path.as_deref(),
                media_type: self.file.content_type.as_deref(),
            })
            .await?;
        Ok(context_file.pipeline_id)
    }
}
