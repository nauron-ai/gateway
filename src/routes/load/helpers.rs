use bytes::Bytes;

use crate::config::AppConfig;

pub fn compose_ingest_key(config: &AppConfig, sha_hex: &str, filename: &str) -> String {
    let suffix = format!("ingest/{}/{}/{}", &sha_hex[..2], sha_hex, filename);
    if config.input_prefix.is_empty() {
        suffix
    } else {
        format!("{}/{}", config.input_prefix, suffix)
    }
}

pub fn compute_sha256(bytes: &Bytes) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

pub fn sanitize_filename(name: &str) -> String {
    let mut clean = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
            clean.push(ch);
        } else {
            clean.push('_');
        }
    }
    clean
}
