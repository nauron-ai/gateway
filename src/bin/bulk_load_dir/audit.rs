use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::audit_md::{
    escape_md, parse_context_id, parse_done_entries, parse_max_row_index, rewrite_context_id,
};
use crate::upload::UploadOutcome;

const PLACEHOLDER: &str = "-";
const STATUS_UPLOADED: &str = "UPLOADED";
const STATUS_FAILED: &str = "FAILED";
const STATUS_SKIPPED_TOO_LARGE: &str = "SKIPPED_TOO_LARGE";

#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub audit_path: PathBuf,
    pub base_url: String,
    pub input_dir: PathBuf,
    pub title: String,
    pub description: String,
    pub mode: String,
}

#[derive(Debug)]
pub struct AuditState {
    config: AuditConfig,
    context_id: Option<i32>,
    done_entries: BTreeMap<String, u64>,
    next_row_index: usize,
}

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("audit io error: {0}")]
    Io(#[from] io::Error),
}

impl AuditState {
    pub fn load_or_init(config: AuditConfig) -> Result<Self, AuditError> {
        let mut state = Self {
            config,
            context_id: None,
            done_entries: BTreeMap::new(),
            next_row_index: 1,
        };
        if state.config.audit_path.exists() {
            state.reload_from_file()?;
        } else {
            state.write_header()?;
        }
        Ok(state)
    }

    pub fn context_id(&self) -> Option<i32> {
        self.context_id
    }

    pub fn persist_context(&mut self, context_id: i32) -> Result<(), AuditError> {
        self.context_id = Some(context_id);
        self.rewrite_header_with_context(context_id)?;
        Ok(())
    }

    pub fn is_done(&self, rel_path: &str, size_bytes: u64) -> bool {
        self.done_entries
            .get(rel_path)
            .is_some_and(|existing| *existing == size_bytes)
    }

    pub fn next_row_index(&self) -> usize {
        self.next_row_index
    }

    pub fn append_row(
        &mut self,
        index: usize,
        rel_path: &str,
        abs_path: &Path,
        fallback_context_id: i32,
        outcome: UploadOutcome,
    ) -> Result<(), AuditError> {
        let (size_bytes, metadata_error) = match fs::metadata(abs_path) {
            Ok(meta) => (meta.len(), None),
            Err(err) => (0, Some(format!("metadata error: {}", err))),
        };
        let ext = match abs_path.extension().and_then(|s| s.to_str()) {
            Some(value) => value.to_ascii_lowercase(),
            None => String::new(),
        };

        let (
            done,
            status,
            context_id,
            pipeline_id,
            job_id,
            file_id,
            sha256_hex,
            dedup,
            status_url,
            error,
        ) = match outcome {
            UploadOutcome::Uploaded(load) => (
                true,
                STATUS_UPLOADED,
                load.context_id,
                load.pipeline_id.to_string(),
                load.job_id.to_string(),
                Some(load.file_id),
                load.sha256_hex,
                Some(load.deduplicated),
                load.status_url,
                None,
            ),
            UploadOutcome::SkippedTooLarge {
                size_bytes,
                max_bytes,
            } => (
                true,
                STATUS_SKIPPED_TOO_LARGE,
                fallback_context_id,
                PLACEHOLDER.to_string(),
                PLACEHOLDER.to_string(),
                None,
                PLACEHOLDER.to_string(),
                None,
                PLACEHOLDER.to_string(),
                Some(format!("size {size_bytes} > max_bytes {max_bytes}")),
            ),
            UploadOutcome::Failed(error) => (
                false,
                STATUS_FAILED,
                fallback_context_id,
                PLACEHOLDER.to_string(),
                PLACEHOLDER.to_string(),
                None,
                PLACEHOLDER.to_string(),
                None,
                PLACEHOLDER.to_string(),
                Some(error),
            ),
        };

        let file_id = match file_id {
            Some(value) => value.to_string(),
            None => PLACEHOLDER.to_string(),
        };
        let dedup = match dedup {
            Some(value) => value.to_string(),
            None => PLACEHOLDER.to_string(),
        };
        let error = match (error, metadata_error) {
            (Some(value), _) => value,
            (None, Some(value)) => value,
            (None, None) => PLACEHOLDER.to_string(),
        };
        let done_cell = if done { "[x]" } else { "[ ]" };

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.audit_path)?;
        writeln!(
            file,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            index,
            done_cell,
            escape_md(rel_path),
            size_bytes,
            ext,
            status,
            context_id,
            escape_md(&pipeline_id),
            escape_md(&job_id),
            escape_md(&file_id),
            escape_md(&sha256_hex),
            escape_md(&dedup),
            escape_md(&status_url),
            escape_md(&error)
        )?;
        file.flush()?;

        if done {
            self.done_entries.insert(rel_path.to_string(), size_bytes);
        }
        self.next_row_index = index + 1;
        Ok(())
    }

    fn reload_from_file(&mut self) -> Result<(), AuditError> {
        let content = fs::read_to_string(&self.config.audit_path)?;
        self.context_id = parse_context_id(&content);
        self.done_entries = parse_done_entries(&content);
        self.next_row_index = parse_max_row_index(&content).saturating_add(1);
        Ok(())
    }

    fn write_header(&self) -> Result<(), AuditError> {
        let now: DateTime<Utc> = Utc::now();
        let mut file = fs::File::create(&self.config.audit_path)?;
        writeln!(file, "# Bulk load audit")?;
        writeln!(file)?;
        writeln!(file, "- started_at: {}", now.to_rfc3339())?;
        writeln!(file, "- base_url: {}", self.config.base_url)?;
        writeln!(file, "- input_dir: {}", self.config.input_dir.display())?;
        writeln!(file, "- context_id: {}", PLACEHOLDER)?;
        writeln!(file, "- mode: {}", self.config.mode)?;
        writeln!(file, "- title: {}", self.config.title)?;
        writeln!(file, "- description: {}", self.config.description)?;
        writeln!(file)?;
        writeln!(
            file,
            "| # | done | path | size_bytes | ext | status | context_id | pipeline_id | job_id | file_id | sha256_hex | deduplicated | status_url | error |"
        )?;
        writeln!(
            file,
            "| - | ---- | ---- | ---------: | --- | ------ | ---------: | ---------- | ------ | ------: | --------- | ----------- | ---------- | ----- |"
        )?;
        Ok(())
    }

    fn rewrite_header_with_context(&self, context_id: i32) -> Result<(), AuditError> {
        let content = fs::read_to_string(&self.config.audit_path)?;
        fs::write(
            &self.config.audit_path,
            rewrite_context_id(&content, context_id),
        )?;
        Ok(())
    }
}

pub fn to_rel_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}
