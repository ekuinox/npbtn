use std::{collections::HashMap, sync::Arc};

use anyhow::{Context as _, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use rspotify::{prelude::OAuthClient as _, scopes, AuthCodePkceSpotify, Credentials, OAuth};
use serde::Deserialize;
use shuttle_runtime::tokio::sync::RwLock;

use crate::templates::IndexTemplate;

#[derive(Clone)]
pub struct AppState {
    pub callback_uri: String,
    pub credentials: Credentials,
    pub pkces: Arc<RwLock<HashMap<String, AuthCodePkceSpotify>>>,
}

impl AppState {
    pub async fn authorize_url(&self) -> Result<String> {
        let oauth = OAuth {
            redirect_uri: self.callback_uri.clone(),
            scopes: scopes!(
                "user-read-currently-playing",
                "playlist-modify-private",
                "user-top-read"
            ),
            ..Default::default()
        };

        let state = oauth.state.clone();

        let mut pkce = AuthCodePkceSpotify::new(self.credentials.clone(), oauth);
        let url = pkce.get_authorize_url(Default::default())?;

        let mut pkces = self.pkces.write().await;
        pkces.insert(state, pkce);

        Ok(url)
    }

    pub async fn pkce(&self, state: &str) -> Result<AuthCodePkceSpotify> {
        let pkce = self
            .pkces
            .write()
            .await
            .remove(state)
            .context("Not found")?;
        Ok(pkce)
    }
}

pub async fn get_index() -> impl IntoResponse {
    IndexTemplate
}

pub async fn get_spotify_auth(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let url = state
        .authorize_url()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
pub struct GetSpotifyCallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn get_spotify_callback(
    State(state): State<AppState>,
    Query(query): Query<GetSpotifyCallbackQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pkce = state
        .pkce(&query.state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    pkce.request_token(&query.code)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let me = pkce
        .me()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    dbg!(&me);

    Ok("")
}
