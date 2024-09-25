use axum::response::IntoResponse;

use crate::templates::IndexTemplate;

#[derive(Default, PartialEq, Clone)]
pub struct AppState {}

pub async fn get_index() -> impl IntoResponse {
    IndexTemplate
}
