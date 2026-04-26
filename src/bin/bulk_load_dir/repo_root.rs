use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoRootError {
    #[error("failed to resolve current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
    #[error("repo root not found (missing .git in ancestors)")]
    NotFound,
}

pub fn resolve_repo_root_path(relative: &str) -> Result<PathBuf, RepoRootError> {
    let cwd = std::env::current_dir()?;
    let root = find_outermost_repo_root(&cwd).ok_or(RepoRootError::NotFound)?;
    Ok(root.join(relative))
}

fn find_outermost_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current: Option<&Path> = Some(start);
    let mut last_match: Option<PathBuf> = None;
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            last_match = Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    last_match
}
