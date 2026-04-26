use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvFileError {
    #[error("failed to read env file: {0}")]
    Read(#[from] std::io::Error),
    #[error("missing required value: {key}")]
    Missing { key: String },
}

pub fn read_env_file(path: &Path) -> Result<BTreeMap<String, String>, EnvFileError> {
    let content = fs::read_to_string(path)?;
    let mut out = BTreeMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        out.insert(key.trim().to_string(), value.trim().to_string());
    }

    Ok(out)
}

pub fn resolve_value(
    cli_value: Option<String>,
    env: &BTreeMap<String, String>,
    key: &str,
) -> Result<String, EnvFileError> {
    if let Some(value) = cli_value {
        return Ok(value);
    }
    env.get(key).cloned().ok_or_else(|| EnvFileError::Missing {
        key: key.to_string(),
    })
}
