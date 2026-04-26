use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn collect_files(
    root: &Path,
    allowed_exts: &BTreeSet<String>,
) -> Result<Vec<PathBuf>, ScanError> {
    let mut stack = vec![root.to_path_buf()];
    let mut out = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }

            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let Some(ext) = extension_lower(&path) else {
                continue;
            };
            if allowed_exts.contains(&ext) {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

fn extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
}
