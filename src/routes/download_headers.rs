use axum::http::HeaderValue;

pub fn attachment_disposition(filename: &str) -> Option<HeaderValue> {
    let escaped = filename
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], "");

    if escaped.is_empty() {
        return None;
    }

    HeaderValue::from_str(&format!("attachment; filename=\"{escaped}\"")).ok()
}
