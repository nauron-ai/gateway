use axum::http::{HeaderName, HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

const ALLOWED_ORIGIN_SUFFIXES: &[&str] = &[".lovable.dev", ".nauron.ai"];
const ALLOWED_ORIGINS: &[&str] = &[
    "https://lovable.dev",
    "https://lovableproject.com",
    "https://nauron.ai",
];
const DEV_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:4173",
    "http://localhost:5173",
    "http://127.0.0.1:3000",
    "http://127.0.0.1:4173",
    "http://127.0.0.1:5173",
];

fn is_localhost(origin: &str) -> bool {
    let host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or(origin)
        .trim_end_matches('/');

    host.starts_with("localhost")
        || host.starts_with("127.0.0.1")
        || host.starts_with("0.0.0.0")
        || host.starts_with("[::1]")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|octet| octet.parse::<u8>().ok())
            .map(|octet| (16..=31).contains(&octet))
            .unwrap_or(false)
}

fn is_allowed_origin(origin: &HeaderValue) -> bool {
    let Ok(origin_str) = origin.to_str() else {
        return false;
    };
    if is_localhost(origin_str) || DEV_ORIGINS.contains(&origin_str) {
        return true;
    }
    let host = origin_str.strip_prefix("https://").unwrap_or(origin_str);
    ALLOWED_ORIGINS.contains(&origin_str)
        || ALLOWED_ORIGIN_SUFFIXES
            .iter()
            .any(|suffix| host.ends_with(suffix))
}

pub fn build_cors_layer() -> CorsLayer {
    // In dev you can set ALLOW_ALL_ORIGINS=1 to bypass origin checks.
    if std::env::var("ALLOW_ALL_ORIGINS").is_ok() {
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|_, _| true))
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|origin, _| {
                is_allowed_origin(origin)
            }))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                HeaderName::from_static("x-requested-with"),
            ])
            .allow_credentials(true)
    }
}
