use std::collections::HashMap;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::{Html, IntoResponse, Redirect},
};

use axum_extra::extract::{cookie::Cookie, CookieJar};
use chrono::{Duration, Utc};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Deserializer};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    auth::{self},
    db,
    htmx::Build,
    page,
};
use crate::{media, settings::ApiState};

pub(crate) async fn health() -> &'static str {
    "Hello!"
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    GetUser(#[from] reqwest::Error),

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

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("{self}");

        match self {
            Self::Auth(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
            Self::GetUser(error) => (StatusCode::UNAUTHORIZED, error.to_string()).into_response(),

            Self::NoGuildFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            Self::InvalidRequest => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),

            Self::InvalidPermission => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            Self::Ytdl(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
            Self::Ffmpeg(error) => (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
            Self::YtdlTerminated | Self::FfmpegTerminated => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }

            Self::Database(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
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
    #[serde(deserialize_with = "serde_string_as_u64")]
    pub id: u64,
    pub owner: bool,
}

fn serde_string_as_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = <&str as Deserialize>::deserialize(deserializer)?;

    value
        .parse::<u64>()
        .map_err(|_| serde::de::Error::invalid_value(serde::de::Unexpected::Str(value), &"u64"))
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

    let db = state.db.lock().await;

    let guilds = db.get_guilds().map_err(Error::Database)?;
    let mut in_a_guild = false;
    for guild in guilds {
        let Some(discord_guild) = discord_guilds
            .iter()
            .find(|discord_guild| discord_guild.id == guild.id)
        else {
            continue;
        };

        in_a_guild = true;

        let now = Utc::now().naive_utc();
        db.insert_user(
            &user.username,
            &token,
            now + Duration::weeks(4),
            &auth.access_token,
            now + Duration::seconds(auth.expires_in as i64),
        )
        .map_err(Error::Database)?;

        db.insert_user_guild(&user.username, guild.id)
            .map_err(Error::Database)?;

        if db.get_user_permissions(&user.username, guild.id).is_err() {
            db.insert_user_permission(
                &user.username,
                guild.id,
                if discord_guild.owner {
                    auth::Permissions(auth::Permission::all())
                } else {
                    Default::default()
                },
            )
            .map_err(Error::Database)?;
        }
    }

    if !in_a_guild {
        return Err(Error::NoGuildFound);
    }

    // TODO: add permissions based on roles

    let uri = Url::parse(&state.origin).expect("should be a valid url");

    let mut cookie = Cookie::new("access_token", token.clone());
    cookie.set_path(uri.path().to_string());
    cookie.set_secure(true);

    Ok((jar.add(cookie), Redirect::to(&format!("{}/", state.origin))))
}

pub(crate) async fn v2_add_intro_to_user(
    State(state): State<ApiState>,
    Path((guild_id, channel)): Path<(u64, String)>,
    user: db::User,
    mut form_data: Multipart,
) -> Result<Html<String>, Redirect> {
    let db = state.db.lock().await;

    while let Ok(Some(field)) = form_data.next_field().await {
        let Some(intro_id) = field.name() else {
            continue;
        };

        let intro_id = intro_id.parse::<i32>().map_err(|err| {
            error!(?err, "invalid intro id");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;

        db.insert_user_intro(&user.name, guild_id, &channel, intro_id)
            .map_err(|err| {
                error!(?err, "failed to add user intro");
                // TODO: change to actual error
                Redirect::to(&format!("{}/login", state.origin))
            })?;
    }

    let guild_intros = db.get_guild_intros(guild_id).map_err(|err| {
        error!(?err, %guild_id, "couldn't get guild intros");
        // TODO: change to actual error
        Redirect::to(&format!("{}/login", state.origin))
    })?;

    let intros = db
        .get_user_channel_intros(&user.name, guild_id, &channel)
        .map_err(|err| {
            error!(?err, user = %user.name, %guild_id, "couldn't get user intros");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;

    Ok(Html(
        page::channel_intro_selector(
            &state.origin,
            guild_id,
            &channel,
            intros.iter(),
            guild_intros.iter(),
        )
        .build(),
    ))
}

pub(crate) async fn v2_remove_intro_from_user(
    State(state): State<ApiState>,
    Path((guild_id, channel)): Path<(u64, String)>,
    user: db::User,
    mut form_data: Multipart,
) -> Result<Html<String>, Redirect> {
    let db = state.db.lock().await;

    while let Ok(Some(field)) = form_data.next_field().await {
        let Some(intro_id) = field.name() else {
            continue;
        };

        let intro_id = intro_id.parse::<i32>().map_err(|err| {
            error!(?err, "invalid intro id");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;

        db.delete_user_intro(&user.name, guild_id, &channel, intro_id)
            .map_err(|err| {
                error!(?err, "failed to remove user intro");
                // TODO: change to actual error
                Redirect::to(&format!("{}/login", state.origin))
            })?;
    }

    let guild_intros = db.get_guild_intros(guild_id).map_err(|err| {
        error!(?err, %guild_id, "couldn't get guild intros");
        // TODO: change to actual error
        Redirect::to(&format!("{}/login", state.origin))
    })?;

    let intros = db
        .get_user_channel_intros(&user.name, guild_id, &channel)
        .map_err(|err| {
            error!(?err, user = %user.name, %guild_id, "couldn't get user intros");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;

    Ok(Html(
        page::channel_intro_selector(
            &state.origin,
            guild_id,
            &channel,
            intros.iter(),
            guild_intros.iter(),
        )
        .build(),
    ))
}

pub(crate) async fn v2_upload_guild_intro(
    State(state): State<ApiState>,
    Path(guild_id): Path<u64>,
    user: db::User,
    mut form_data: Multipart,
) -> Result<HeaderMap, Error> {
    let db = state.db.lock().await;
    let mut name = None;
    let mut file = None;

    if !db
        .get_guilds()
        .map_err(Error::Database)?
        .into_iter()
        .any(|guild| guild.id == guild_id)
    {
        return Err(Error::NoGuildFound);
    }

    let user_permissions = db
        .get_user_permissions(&user.name, guild_id)
        .map_err(Error::Database)?;

    if !user_permissions.can(auth::Permission::UploadSounds) {
        return Err(Error::InvalidPermission);
    }

    while let Ok(Some(field)) = form_data.next_field().await {
        let Some(field_name) = field.name() else {
            continue;
        };

        if field_name.eq_ignore_ascii_case("name") {
            name = Some(field.text().await.map_err(|_| Error::InvalidRequest)?);
            continue;
        }

        if field_name.eq_ignore_ascii_case("file") {
            file = Some(field.bytes().await.map_err(|_| Error::InvalidRequest)?);
            continue;
        }
    }

    let Some(name) = name else {
        return Err(Error::InvalidRequest);
    };
    let Some(file) = file else {
        return Err(Error::InvalidRequest);
    };

    let uuid = Uuid::new_v4().to_string();
    let temp_path = format!("./sounds/temp/{uuid}");
    let dest_path = format!("./sounds/{uuid}.mp3");

    // Write original file so its ready for codec conversion
    std::fs::write(&temp_path, file)?;
    media::normalize(&temp_path, &dest_path).await?;
    std::fs::remove_file(&temp_path)?;

    db.insert_intro(&name, 0, guild_id, &format!("{uuid}.mp3"))
        .map_err(Error::Database)?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    Ok(headers)
}

pub(crate) async fn v2_add_guild_intro(
    State(state): State<ApiState>,
    Path(guild_id): Path<u64>,
    Query(mut params): Query<HashMap<String, String>>,
    user: db::User,
) -> Result<HeaderMap, Error> {
    let db = state.db.lock().await;
    let Some(url) = params.remove("url") else {
        return Err(Error::InvalidRequest);
    };
    let Some(name) = params.remove("name") else {
        return Err(Error::InvalidRequest);
    };

    if !db
        .get_guilds()
        .map_err(Error::Database)?
        .into_iter()
        .any(|guild| guild.id == guild_id)
    {
        return Err(Error::NoGuildFound);
    }

    let user_permissions = db
        .get_user_permissions(&user.name, guild_id)
        .map_err(Error::Database)?;

    if !user_permissions.can(auth::Permission::UploadSounds) {
        return Err(Error::InvalidPermission);
    }

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

    db.insert_intro(&name, 0, guild_id, &format!("{uuid}.mp3"))
        .map_err(Error::Database)?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    Ok(headers)
}
