use std::{collections::HashMap, sync::Arc};

use anyhow::{Context as _, Result};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE, prelude::*};
use rspotify::{
    model::{AdditionalType, PlayableItem},
    prelude::{BaseClient, OAuthClient},
    scopes, AuthCodePkceSpotify, Credentials, OAuth, Token,
};
use serde::{Deserialize, Serialize};
use shuttle_runtime::tokio::sync::RwLock;

use crate::templates::{ComposeTemplate, IndexTemplate};

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

pub async fn get_index_js() -> impl IntoResponse {
    let mut header = HeaderMap::new();
    header.insert("Content-Type", "application/javascript".parse().unwrap());
    (
        header,
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/index.js")),
    )
}

pub async fn get_compose() -> impl IntoResponse {
    ComposeTemplate
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

    let token = {
        let token = pkce.get_token();
        let token = token
            .lock()
            .await
            .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "".into()))?;
        let token = token
            .clone()
            .context("Token is not found.")
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        token
    };

    let token =
        token2text(&token).map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "".to_string()))?;

    Ok(Redirect::to(&format!("/?token={token}")).into_response())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpResponse {
    pub track_name: String,
    pub track_url: String,
    pub artist_names: Vec<String>,
    pub album_name: String,
}

#[derive(Deserialize)]
pub struct GetNpRequest {
    pub token: String,
}

pub async fn get_np(
    Query(query): Query<GetNpRequest>,
) -> Result<Json<Option<NpResponse>>, (StatusCode, String)> {
    let token = text2token(&query.token).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let client = AuthCodePkceSpotify::from_token(token);

    let np = client
        .current_playing(Default::default(), Some(vec![&AdditionalType::Track]))
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, "".to_string()))?;
    Ok(Json(np.and_then(|np| {
        np.item.and_then(|item| match item {
            PlayableItem::Track(mut track) => {
                if let Some(track_url) = track.external_urls.remove("spotify") {
                    Some(NpResponse {
                        track_name: track.name,
                        track_url,
                        album_name: track.album.name,
                        artist_names: track
                            .artists
                            .into_iter()
                            .map(|artist| artist.name)
                            .collect(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        })
    })))
}

fn token2text(token: &Token) -> Result<String> {
    let text = serde_json::to_string(&token)?;
    Ok(URL_SAFE.encode(text.as_bytes()))
}

fn text2token(text: &str) -> Result<Token> {
    let bytes = URL_SAFE.decode(text)?;
    let text = String::from_utf8(bytes)?;
    serde_json::from_str(&text).map_err(From::from)
}
