pub mod api;
mod mapping;
pub(crate) mod proto;

lazy_static::lazy_static! {
    static ref BASE_URL: String = std::env::var("SIGSONG_SERVER_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:7878/api".to_string());
}

pub(crate) fn base_url() -> &'static str {
    &BASE_URL
}
