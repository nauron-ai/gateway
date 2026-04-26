use std::sync::Arc;

use crate::db::files::FileOrigin;
use crate::error::GatewayError;
use crate::state::AppState;
use tracing::info;

use super::form::UploadRequest;
use super::helpers::compose_ingest_key;
use super::ingest::{
    BuildEntryParams, WorkItem, build_entry, build_metadata, extract_archive_items, is_zip_upload,
    upsert_file_record,
};
use super::job_actions::decide_job_action;
use super::response::{LoadContextResponse, LoadEntry};

pub async fn process_upload(
    state: Arc<AppState>,
    request: UploadRequest,
) -> Result<LoadContextResponse, GatewayError> {
    let UploadRequest { form, file } = request;
    let context_id = form.context_id;
    let user_id = form.user_id.clone();
    let dry_run = form.dry_run.unwrap_or(false);

    ensure_context_exists(&state, context_id).await?;

    let work_items = build_work_items(file)?;
    let mut entries = Vec::with_capacity(work_items.len());
    for item in work_items {
        entries.push(process_item(&state, context_id, user_id.clone(), dry_run, item).await?);
    }
    Ok(LoadContextResponse::from_entries(entries))
}

async fn ensure_context_exists(state: &Arc<AppState>, context_id: i32) -> Result<(), GatewayError> {
    if state.context_repo.get(context_id).await?.is_some() {
        Ok(())
    } else {
        Err(GatewayError::ContextNotFound(context_id))
    }
}

fn build_work_items(file: super::form::UploadedFile) -> Result<Vec<WorkItem>, GatewayError> {
    if is_zip_upload(&file) {
        return extract_archive_items(&file);
    }
    Ok(vec![WorkItem::new(file, FileOrigin::Upload, None)])
}

async fn process_item(
    state: &Arc<AppState>,
    context_id: i32,
    user_id: Option<String>,
    dry_run: bool,
    item: WorkItem,
) -> Result<LoadEntry, GatewayError> {
    let display_name = item
        .file
        .name
        .clone()
        .unwrap_or_else(|| "document.bin".to_string());
    let (sha_bytes, sha_hex, sanitized_name) = build_metadata(&display_name, &item.file.bytes);
    let ingest_key = compose_ingest_key(&state.config, &sha_hex, &sanitized_name);
    let (mut file_record, was_new) =
        upsert_file_record(state, &item.file, &sha_bytes, &ingest_key).await?;
    if was_new {
        state
            .metrics
            .record_dedup_miss(file_record.id, context_id, &sha_hex);
    } else {
        state
            .metrics
            .record_dedup_hit(file_record.id, context_id, &sha_hex);
    }
    if matches!(item.origin, FileOrigin::ArchiveEntry) {
        state
            .metrics
            .record_zip_entry(file_record.id, context_id, !was_new);
    }
    info!(
        file_id = file_record.id,
        context_id,
        sha256 = sha_hex.as_str(),
        deduplicated = !was_new,
        origin = ?item.origin,
        "attached file to context"
    );

    let pipeline_id = item
        .attach_to_context(state, context_id, &file_record)
        .await?;

    let (job_id, job_mode) = decide_job_action(
        state,
        &mut file_record,
        context_id,
        pipeline_id,
        user_id,
        dry_run,
    )
    .await?;

    Ok(build_entry(BuildEntryParams {
        context_id,
        job_id,
        job_mode,
        file_record: &file_record,
        sha_hex,
        deduplicated: !was_new,
        original_path: item.original_path,
        config: &state.config,
        pipeline_id,
    }))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use bytes::Bytes;
    use zip::write::FileOptions;

    use crate::db::files::FileOrigin;

    use super::super::form::UploadedFile;
    use super::build_work_items;

    #[test]
    fn ooxml_marked_as_zip_stays_single_upload_item() {
        let file = UploadedFile {
            bytes: Bytes::from_static(b"PK\x03\x04\x14\x00"),
            name: Some("document.docx".into()),
            content_type: Some("application/zip".into()),
        };
        let work_items = build_work_items(file).expect("work items should be built");
        assert_eq!(work_items.len(), 1);
        assert!(matches!(work_items[0].origin, FileOrigin::Upload));
    }

    #[test]
    fn regular_zip_expands_to_archive_entries() {
        let file = UploadedFile {
            bytes: zip_bytes_with_single_file(),
            name: Some("bundle.zip".into()),
            content_type: Some("application/zip".into()),
        };
        let work_items = build_work_items(file).expect("zip should expand");
        assert_eq!(work_items.len(), 1);
        assert!(matches!(work_items[0].origin, FileOrigin::ArchiveEntry));
    }

    fn zip_bytes_with_single_file() -> Bytes {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file("a.txt", FileOptions::default())
            .expect("zip entry should open");
        writer
            .write_all(b"hello")
            .expect("zip entry should write payload");
        let cursor = writer.finish().expect("zip should finalize");
        Bytes::from(cursor.into_inner())
    }
}
