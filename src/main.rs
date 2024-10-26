mod routes;
mod templates;

use axum::{routing::get, Router};
use rspotify::Credentials;
use shuttle_runtime::SecretStore;

use crate::routes::AppState;

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    let spotify_client_id = secrets
        .get("SPOTIFY_CLIENT_ID")
        .expect("`SPOTIFY_CLIENT_ID` is empty.");
    let spotify_client_secret = secrets
        .get("SPOTIFY_CLIENT_SECRET")
        .expect("`SPOTIFY_CLIENT_SECRET is empty.`");
    let spotify_redirect_uri = secrets
        .get("SPOTIFY_REDIRECT_URI")
        .expect("`SPOTIFY_REDIRECT_URI is empty.`");

    let credentials = Credentials::new(&spotify_client_id, &spotify_client_secret);

    let router = Router::new()
        .route("/", get(routes::get_index))
        .route("/compose", get(routes::get_compose))
        .route("/index.js", get(routes::get_index_js))
        .route("/spotify/auth", get(routes::get_spotify_auth))
        .route("/spotify/callback", get(routes::get_spotify_callback))
        .route("/np", get(routes::get_np))
        .with_state(AppState {
            credentials,
            callback_uri: spotify_redirect_uri,
            pkces: Default::default(),
        });

    Ok(router.into())
}
