use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};
use uuid::Uuid;

use crate::settings::{ApiState, Intro, IntroIndex};
use crate::{auth, settings::FileIntro};

#[derive(Serialize)]
pub(crate) enum IntroResponse<'a> {
    Intros(&'a HashMap<String, Intro>),
    NoGuildFound,
}

#[derive(Serialize)]
pub(crate) enum MeResponse<'a> {
    Me(Me<'a>),
    NoUserFound,
}

#[derive(Serialize)]
pub(crate) struct Me<'a> {
    pub(crate) username: String,
    pub(crate) guilds: Vec<MeGuild<'a>>,
}

#[derive(Serialize)]
pub(crate) struct MeGuild<'a> {
    pub(crate) name: String,
    pub(crate) channels: Vec<MeChannel<'a>>,
}

#[derive(Serialize)]
pub(crate) struct MeChannel<'a> {
    pub(crate) name: String,
    pub(crate) intros: &'a Vec<IntroIndex>,
}

pub(crate) async fn health() -> &'static str {
    "Hello!"
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    GetUser(#[from] reqwest::Error),

    #[error("User doesn't exist")]
    NoUserFound,
    #[error("Guild doesn't exist")]
    NoGuildFound,
    #[error("invalid request")]
    InvalidRequest,

    #[error("Invalid permissions for request")]
    InvalidPermission,
    #[error("{0}")]
    Ytdl(#[from] std::io::Error),

    #[error("ytdl terminated unsuccessfully")]
    YtdlTerminated,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Auth(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
            Self::GetUser(error) => (StatusCode::UNAUTHORIZED, error.to_string()).into_response(),

            Self::NoGuildFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            Self::NoUserFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),

            Self::InvalidPermission => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            Self::Ytdl(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
            Self::YtdlTerminated => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}

#[derive(Deserialize)]
struct DiscordUser {
    pub username: String,
}

pub(crate) async fn auth(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, Error> {
    let Some(code) = params.get("code") else {
        return Err(Error::Auth("no code".to_string()));
    };

    info!("attempting to get access token with code {}", code);

    let mut data = HashMap::new();
    data.insert("client_id", state.secrets.client_id.as_str());
    data.insert("client_secret", state.secrets.client_secret.as_str());
    data.insert("grant_type", "authorization_code");
    data.insert("code", code);
    data.insert("redirect_uri", "http://localhost:5173/auth");

    let client = reqwest::Client::new();

    let auth: auth::Discord = client
        .post("https://discord.com/api/oauth2/token")
        .form(&data)
        .send()
        .await
        .map_err(|err| Error::Auth(err.to_string()))?
        .json()
        .await
        .map_err(|err| Error::Auth(err.to_string()))?;
    let token = Uuid::new_v4().to_string();

    // Get authorized username
    let user: DiscordUser = client
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(&auth.access_token)
        .send()
        .await?
        .json()
        .await?;

    let mut settings = state.settings.lock().await;
    settings.auth_users.insert(
        token.clone(),
        auth::User {
            auth,
            // TODO: replace with roles
            permissions: auth::Permissions::default(),
            name: user.username.clone(),
        },
    );

    Ok(Json(json!({"token": token, "username": user.username})))
}

pub(crate) async fn add_intro_to_user(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((guild, channel, intro_index)): Path<(u64, String, String)>,
) {
    let mut settings = state.settings.lock().await;
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else { return; };
    let user = match settings.auth_users.get(token) {
        Some(user) => user.name.clone(),
        None => return,
    };

    let Some(guild) = settings.guilds.get_mut(&guild) else { return; };
    let Some(channel) = guild.channels.get_mut(&channel) else { return; };
    let Some(user) = channel.users.get_mut(&user) else { return; };

    if !user.intros.iter().any(|intro| intro.index == intro_index) {
        user.intros.push(IntroIndex {
            index: intro_index,
            volume: 20,
        });

        if let Err(err) = settings.save() {
            error!("Failed to save config: {err:?}");
        }
    }
}

pub(crate) async fn remove_intro_to_user(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((guild, channel, intro_index)): Path<(u64, String, String)>,
) {
    let mut settings = state.settings.lock().await;
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else { return; };
    let user = match settings.auth_users.get(token) {
        Some(user) => user.name.clone(),
        None => return,
    };

    let Some(guild) = settings.guilds.get_mut(&guild) else { return; };
    let Some(channel) = guild.channels.get_mut(&channel) else { return; };
    let Some(user) = channel.users.get_mut(&user) else { return; };

    if let Some(index) = user
        .intros
        .iter()
        .position(|intro| intro_index == intro.index)
    {
        user.intros.remove(index);
    }

    if let Err(err) = settings.save() {
        error!("Failed to save config: {err:?}");
    }
}

pub(crate) async fn intros(
    State(state): State<Arc<ApiState>>,
    Path(guild): Path<u64>,
) -> Json<Value> {
    let settings = state.settings.lock().await;
    let Some(guild) = settings.guilds.get(&guild) else { return Json(json!(IntroResponse::NoGuildFound)); };

    Json(json!(IntroResponse::Intros(&guild.intros)))
}

pub(crate) async fn me(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> Json<Value> {
    let settings = state.settings.lock().await;
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else { return Json(json!(MeResponse::NoUserFound)); };

    let user = match settings.auth_users.get(token) {
        Some(user) => user.name.clone(),
        None => return Json(json!(MeResponse::NoUserFound)),
    };

    let mut me = Me {
        username: user.clone(),
        guilds: Vec::new(),
    };

    for g in &settings.guilds {
        let mut guild = MeGuild {
            name: g.0.to_string(),
            channels: Vec::new(),
        };

        for channel in &g.1.channels {
            let user_settings = channel.1.users.iter().find(|u| *u.0 == user);

            let Some(user) = user_settings else { continue; };

            guild.channels.push(MeChannel {
                name: channel.0.to_owned(),
                intros: &user.1.intros,
            });
        }

        me.guilds.push(guild);
    }

    if me.guilds.is_empty() {
        Json(json!(MeResponse::NoUserFound))
    } else {
        Json(json!(MeResponse::Me(me)))
    }
}

pub(crate) async fn add_guild_intro(
    State(state): State<Arc<ApiState>>,
    Path((guild, url)): Path<(u64, String)>,
    Query(mut params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<(), Error> {
    let mut settings = state.settings.lock().await;
    // TODO: make this an impl on HeaderMap
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else { return Err(Error::NoUserFound); };
    let Some(friendly_name) = params.remove("name") else { return Err(Error::InvalidRequest); };
    let user = match settings.auth_users.get(token) {
        Some(user) => user,
        None => return Err(Error::NoUserFound),
    };

    if !user.permissions.can(auth::Permission::DownloadSounds) {
        return Err(Error::InvalidPermission);
    }

    let Some(guild) = settings.guilds.get_mut(&guild) else { return Err(Error::NoGuildFound); };

    let uuid = Uuid::new_v4().to_string();
    let child = tokio::process::Command::new("yt-dlp")
        .arg(&url)
        .args(["-o", &format!("./sounds/{uuid}")])
        .args(["-x", "--audio-format", "mp3"])
        .spawn()
        .map_err(Error::Ytdl)?
        .wait()
        .await
        .map_err(Error::Ytdl)?;

    if !child.success() {
        return Err(Error::YtdlTerminated);
    }

    guild.intros.insert(
        uuid.clone(),
        Intro::File(FileIntro {
            filename: format!("{uuid}.mp3"),
            friendly_name,
        }),
    );

    Ok(())
}
