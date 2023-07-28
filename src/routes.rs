use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Redirect},
    Form, Json,
};

use axum_extra::extract::{cookie::Cookie, CookieJar};
use reqwest::{Proxy, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    auth::{self, User},
    settings::FileIntro,
};
use crate::{
    media,
    settings::{ApiState, GuildUser, Intro, IntroIndex, UserSettings},
};

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
    // NOTE(pcleavelin): for some reason this doesn't serialize properly if a u64
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) channels: Vec<MeChannel<'a>>,
    pub(crate) permissions: auth::Permissions,
}

#[derive(Serialize)]
pub(crate) struct MeChannel<'a> {
    pub(crate) name: String,
    pub(crate) intros: &'a Vec<IntroIndex>,
}

#[derive(Deserialize)]
pub(crate) struct DeleteIntroRequest(Vec<String>);

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
    #[error("{0}")]
    Ffmpeg(String),

    #[error("ytdl terminated unsuccessfully")]
    YtdlTerminated,
    #[error("ffmpeg terminated unsuccessfully")]
    FfmpegTerminated,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("{self}");

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
            Self::Ffmpeg(error) => (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
            Self::YtdlTerminated | Self::FfmpegTerminated => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}

#[derive(Deserialize)]
struct DiscordUser {
    pub username: String,
}

#[derive(Deserialize)]
struct DiscordUserGuild {
    pub id: String,
    pub name: String,
    pub owner: bool,
}

pub(crate) async fn v2_auth(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), Error> {
    let Some(code) = params.get("code") else {
        return Err(Error::Auth("no code".to_string()));
    };

    info!("attempting to get access token with code {}", code);

    let mut data = HashMap::new();

    let redirect_uri = format!("{}/v2/auth", state.origin);
    data.insert("client_id", state.secrets.client_id.as_str());
    data.insert("client_secret", state.secrets.client_secret.as_str());
    data.insert("grant_type", "authorization_code");
    data.insert("code", code);
    data.insert("redirect_uri", &redirect_uri);

    let client = reqwest::Client::new();

    let auth: auth::Discord = client
        .post("https://discord.com/api/oauth2/token")
        .form(&data)
        .send()
        .await
        .map_err(|err| Error::Auth(err.to_string()))?
        .json()
        .await
        .map_err(|err| {
            error!(?err, "auth error");
            Error::Auth(err.to_string())
        })?;
    let token = Uuid::new_v4().to_string();

    // Get authorized username
    let user: DiscordUser = client
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(&auth.access_token)
        .send()
        .await?
        .json()
        .await?;

    // TODO: get bot's guilds so we only save users who are able to use the bot
    let discord_guilds: Vec<DiscordUserGuild> = client
        .get("https://discord.com/api/v10/users/@me/guilds")
        .bearer_auth(&auth.access_token)
        .send()
        .await?
        .json()
        .await
        .map_err(|err| Error::Auth(err.to_string()))?;

    let mut settings = state.settings.lock().await;
    let mut in_a_guild = false;
    for g in settings.guilds.iter_mut() {
        let Some(discord_guild) = discord_guilds
            .iter()
            .find(|discord_guild| discord_guild.id == g.0.to_string())
        else {
            continue;
        };

        in_a_guild = true;

        if !g.1.users.contains_key(&user.username) {
            g.1.users.insert(
                user.username.clone(),
                GuildUser {
                    permissions: if discord_guild.owner {
                        auth::Permissions(auth::Permission::all())
                    } else {
                        Default::default()
                    },
                },
            );
        }
    }

    if !in_a_guild {
        return Err(Error::NoGuildFound);
    }

    settings.auth_users.insert(
        token.clone(),
        auth::User {
            auth,
            name: user.username.clone(),
        },
    );
    // TODO: add permissions based on roles

    let mut cookie = Cookie::new("access_token", token.clone());
    cookie.set_path("/");
    cookie.set_secure(true);

    Ok((jar.add(cookie), Redirect::to("/")))
}

pub(crate) async fn auth(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, Error> {
    let Some(code) = params.get("code") else {
        return Err(Error::Auth("no code".to_string()));
    };

    info!("attempting to get access token with code {}", code);

    let mut data = HashMap::new();

    let redirect_uri = format!("{}/auth", state.origin);
    data.insert("client_id", state.secrets.client_id.as_str());
    data.insert("client_secret", state.secrets.client_secret.as_str());
    data.insert("grant_type", "authorization_code");
    data.insert("code", code);
    data.insert("redirect_uri", &redirect_uri);

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

    // TODO: get bot's guilds so we only save users who are able to use the bot
    let discord_guilds: Vec<DiscordUserGuild> = client
        .get("https://discord.com/api/v10/users/@me/guilds")
        .bearer_auth(&auth.access_token)
        .send()
        .await?
        .json()
        .await
        .map_err(|err| Error::Auth(err.to_string()))?;

    let mut settings = state.settings.lock().await;
    let mut in_a_guild = false;
    for g in settings.guilds.iter_mut() {
        let Some(discord_guild) = discord_guilds
            .iter()
            .find(|discord_guild| discord_guild.id == g.0.to_string())
        else {
            continue;
        };

        in_a_guild = true;

        if !g.1.users.contains_key(&user.username) {
            g.1.users.insert(
                user.username.clone(),
                GuildUser {
                    permissions: if discord_guild.owner {
                        auth::Permissions(auth::Permission::all())
                    } else {
                        Default::default()
                    },
                },
            );
        }
    }

    if !in_a_guild {
        return Err(Error::NoGuildFound);
    }

    settings.auth_users.insert(
        token.clone(),
        auth::User {
            auth,
            name: user.username.clone(),
        },
    );
    // TODO: add permissions based on roles

    Ok(Json(json!({"token": token, "username": user.username})))
}

pub(crate) async fn v2_add_intro_to_user(
    State(state): State<ApiState>,
    Path((guild_id, channel)): Path<(u64, String)>,
    user: User,
    mut form_data: Multipart,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    let mut settings = state.settings.lock().await;

    let Some(guild) = settings.guilds.get_mut(&guild_id) else {
        return headers;
    };
    let Some(channel) = guild.channels.get_mut(&channel) else {
        return headers;
    };
    let Some(channel_user) = channel.users.get_mut(&user.name) else {
        return headers;
    };

    while let Ok(Some(field)) = form_data.next_field().await {
        let Some(field_name) = field.name() else {
            continue;
        };

        if !channel_user
            .intros
            .iter()
            .any(|intro| intro.index == field_name)
        {
            channel_user.intros.push(IntroIndex {
                index: field_name.to_string(),
                volume: 20,
            });
        }
    }

    // TODO: don't save on every change
    if let Err(err) = settings.save() {
        error!("Failed to save config: {err:?}");
    }

    headers
}

pub(crate) async fn v2_remove_intro_from_user(
    State(state): State<ApiState>,
    Path((guild_id, channel)): Path<(u64, String)>,
    user: User,
    mut form_data: Multipart,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    let mut settings = state.settings.lock().await;

    let Some(guild) = settings.guilds.get_mut(&guild_id) else {
        return headers;
    };
    let Some(channel) = guild.channels.get_mut(&channel) else {
        return headers;
    };
    let Some(channel_user) = channel.users.get_mut(&user.name) else {
        return headers;
    };

    while let Ok(Some(field)) = form_data.next_field().await {
        let Some(field_name) = field.name() else {
            continue;
        };

        if let Some(index) = channel_user
            .intros
            .iter()
            .position(|intro| intro.index == field_name)
        {
            channel_user.intros.remove(index);
        }
    }

    // TODO: don't save on every change
    if let Err(err) = settings.save() {
        error!("Failed to save config: {err:?}");
    }

    headers
}

pub(crate) async fn add_intro_to_user(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((guild, channel, intro_index)): Path<(u64, String, String)>,
) {
    let mut settings = state.settings.lock().await;
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else {
        return;
    };
    let user = match settings.auth_users.get(token) {
        Some(user) => user.name.clone(),
        None => return,
    };

    let Some(guild) = settings.guilds.get_mut(&guild) else {
        return;
    };
    let Some(channel) = guild.channels.get_mut(&channel) else {
        return;
    };
    let Some(user) = channel.users.get_mut(&user) else {
        return;
    };

    if !user.intros.iter().any(|intro| intro.index == intro_index) {
        user.intros.push(IntroIndex {
            index: intro_index,
            volume: 20,
        });

        // TODO: don't save on every change
        if let Err(err) = settings.save() {
            error!("Failed to save config: {err:?}");
        }
    }
}

pub(crate) async fn remove_intro_to_user(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((guild, channel, intro_index)): Path<(u64, String, String)>,
) {
    let mut settings = state.settings.lock().await;
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else {
        return;
    };
    let user = match settings.auth_users.get(token) {
        Some(user) => user.name.clone(),
        None => return,
    };

    let Some(guild) = settings.guilds.get_mut(&guild) else {
        return;
    };
    let Some(channel) = guild.channels.get_mut(&channel) else {
        return;
    };
    let Some(user) = channel.users.get_mut(&user) else {
        return;
    };

    if let Some(index) = user
        .intros
        .iter()
        .position(|intro| intro_index == intro.index)
    {
        user.intros.remove(index);
    }

    // TODO: don't save on every change
    if let Err(err) = settings.save() {
        error!("Failed to save config: {err:?}");
    }
}

pub(crate) async fn intros(State(state): State<ApiState>, Path(guild): Path<u64>) -> Json<Value> {
    let settings = state.settings.lock().await;
    let Some(guild) = settings.guilds.get(&guild) else {
        return Json(json!(IntroResponse::NoGuildFound));
    };

    Json(json!(IntroResponse::Intros(&guild.intros)))
}

pub(crate) async fn me(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Value>, Error> {
    let mut settings = state.settings.lock().await;
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else {
        return Err(Error::NoUserFound);
    };

    let (username, access_token) = match settings.auth_users.get(token) {
        Some(user) => (user.name.clone(), user.auth.access_token.clone()),
        None => return Err(Error::NoUserFound),
    };

    let mut me = Me {
        username: username.clone(),
        guilds: Vec::new(),
    };

    for g in settings.guilds.iter_mut() {
        // TODO: don't do this n^2 lookup

        let guild_user =
            g.1.users
                // TODO: why must clone
                .entry(username.clone())
                // TODO: check if owner for permissions
                .or_insert(Default::default());

        let mut guild = MeGuild {
            id: g.0.to_string(),
            name: g.1.name.clone(),
            channels: Vec::new(),
            permissions: guild_user.permissions,
        };

        for channel in g.1.channels.iter_mut() {
            let user_settings = channel
                .1
                .users
                .entry(username.clone())
                .or_insert(UserSettings { intros: Vec::new() });

            guild.channels.push(MeChannel {
                name: channel.0.to_owned(),
                intros: &user_settings.intros,
            });
        }

        me.guilds.push(guild);
    }

    if me.guilds.is_empty() {
        Ok(Json(json!(MeResponse::NoUserFound)))
    } else {
        Ok(Json(json!(MeResponse::Me(me))))
    }
}

pub(crate) async fn upload_guild_intro(
    State(state): State<ApiState>,
    Path(guild): Path<u64>,
    Query(mut params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    file: Bytes,
) -> Result<(), Error> {
    let mut settings = state.settings.lock().await;

    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else {
        return Err(Error::NoUserFound);
    };
    let Some(friendly_name) = params.remove("name") else {
        return Err(Error::InvalidRequest);
    };

    {
        let Some(guild) = settings.guilds.get(&guild) else {
            return Err(Error::NoGuildFound);
        };
        let auth_user = match settings.auth_users.get(token) {
            Some(user) => user,
            None => return Err(Error::NoUserFound),
        };
        let Some(guild_user) = guild.users.get(&auth_user.name) else {
            return Err(Error::NoUserFound);
        };

        if !guild_user.permissions.can(auth::Permission::UploadSounds) {
            return Err(Error::InvalidPermission);
        }
    }

    let Some(guild) = settings.guilds.get_mut(&guild) else {
        return Err(Error::NoGuildFound);
    };
    let uuid = Uuid::new_v4().to_string();
    let temp_path = format!("./sounds/temp/{uuid}");
    let dest_path = format!("./sounds/{uuid}.mp3");

    // Write original file so its ready for codec conversion
    std::fs::write(&temp_path, file)?;
    media::normalize(&temp_path, &dest_path).await?;
    std::fs::remove_file(&temp_path)?;

    guild.intros.insert(
        uuid.clone(),
        Intro::File(FileIntro {
            filename: format!("{uuid}.mp3"),
            friendly_name,
        }),
    );

    Ok(())
}

pub(crate) async fn add_guild_intro(
    State(state): State<ApiState>,
    Path(guild): Path<u64>,
    Query(mut params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<(), Error> {
    let mut settings = state.settings.lock().await;
    // TODO: make this an impl on HeaderMap
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else {
        return Err(Error::NoUserFound);
    };
    let Some(url) = params.remove("url") else {
        return Err(Error::InvalidRequest);
    };
    let Some(friendly_name) = params.remove("name") else {
        return Err(Error::InvalidRequest);
    };

    {
        let Some(guild) = settings.guilds.get(&guild) else {
            return Err(Error::NoGuildFound);
        };
        let auth_user = match settings.auth_users.get(token) {
            Some(user) => user,
            None => return Err(Error::NoUserFound),
        };
        let Some(guild_user) = guild.users.get(&auth_user.name) else {
            return Err(Error::NoUserFound);
        };

        if !guild_user.permissions.can(auth::Permission::UploadSounds) {
            return Err(Error::InvalidPermission);
        }
    }

    let Some(guild) = settings.guilds.get_mut(&guild) else {
        return Err(Error::NoGuildFound);
    };

    let uuid = Uuid::new_v4().to_string();
    let child = tokio::process::Command::new("yt-dlp")
        .arg(&url)
        .args(["-o", &format!("sounds/{uuid}")])
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

pub(crate) async fn delete_guild_intro(
    State(state): State<ApiState>,
    Path(guild): Path<u64>,
    headers: HeaderMap,
    Json(body): Json<DeleteIntroRequest>,
) -> Result<(), Error> {
    let mut settings = state.settings.lock().await;
    // TODO: make this an impl on HeaderMap
    let Some(token) = headers.get("token").and_then(|v| v.to_str().ok()) else {
        return Err(Error::NoUserFound);
    };

    {
        let Some(guild) = settings.guilds.get(&guild) else {
            return Err(Error::NoGuildFound);
        };
        let auth_user = match settings.auth_users.get(token) {
            Some(user) => user,
            None => return Err(Error::NoUserFound),
        };
        let Some(guild_user) = guild.users.get(&auth_user.name) else {
            return Err(Error::NoUserFound);
        };

        if !guild_user.permissions.can(auth::Permission::DeleteSounds) {
            return Err(Error::InvalidPermission);
        }
    }

    let Some(guild) = settings.guilds.get_mut(&guild) else {
        return Err(Error::NoGuildFound);
    };

    // Remove intro from any users
    for channel in guild.channels.iter_mut() {
        for user in channel.1.users.iter_mut() {
            user.1
                .intros
                .retain(|user_intro| !body.0.iter().any(|intro| &user_intro.index == intro));
        }
    }

    for intro in &body.0 {
        guild.intros.remove(intro);
    }

    Ok(())
}
