use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::state::AppState;

pub mod chat;
pub mod connections;
pub mod contexts;
pub mod documents;
pub mod files;
pub mod users;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/connections", get(connections::list_connection_events))
        .route("/documents/{doc_id}", get(documents::download_document))
        .route("/files", get(files::list_files))
        .route("/files/{file_id}", get(files::file_details))
        .route("/files/{file_id}/retry", post(files::retry_file))
        .route(
            "/contexts/{context_id}/files",
            get(contexts::list_context_files),
        )
        .merge(chat::router())
        .route("/users", get(users::list_users).post(users::create_user))
        .route("/users/{user_id}", patch(users::update_user))
}
