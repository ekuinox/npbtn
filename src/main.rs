mod routes;
mod templates;

use axum::{routing::get, Router};

use crate::routes::AppState;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(routes::get_index))
        .with_state(AppState {});

    Ok(router.into())
}
