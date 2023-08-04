use std::{collections::HashMap, sync::Arc};

use crate::{auth, db::Database};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts, response::Redirect};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use tracing::trace;
use uuid::Uuid;

type UserToken = String;

// TODO: make this is wrapped type so cloning isn't happening
#[derive(Clone)]
pub(crate) struct ApiState {
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub settings: Arc<tokio::sync::Mutex<Settings>>,
    pub secrets: auth::DiscordSecret,
    pub origin: String,
}

#[async_trait]
impl FromRequestParts<ApiState> for crate::auth::User {
    type Rejection = Redirect;

    async fn from_request_parts(
        Parts { headers, .. }: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&headers);

        if let Some(token) = jar.get("access_token") {
            match state.settings.lock().await.auth_users.get(token.value()) {
                // :vomit:
                Some(user) => Ok(user.clone()),
                None => Err(Redirect::to("/login")),
            }
        } else {
            Err(Redirect::to("/login"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) run_api: bool,
    #[serde(default)]
    pub(crate) run_bot: bool,
    pub(crate) guilds: HashMap<u64, GuildSettings>,

    #[serde(default)]
    pub(crate) auth_users: HashMap<UserToken, auth::User>,
}
impl TypeMapKey for Settings {
    type Value = Arc<Settings>;
}

impl Settings {
    pub(crate) fn save(&self) -> Result<(), std::io::Error> {
        trace!("attempting to save config");
        let serialized = serde_json::to_string_pretty(&self)?;

        std::fs::copy(
            "./config/settings.json",
            format!(
                "./config/{}-settings.json.old",
                chrono::Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S")
            ),
        )?;
        trace!("created copy of original settings");

        std::fs::write("./config/settings.json", serialized)?;

        trace!("saved settings to disk");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuildSettings {
    pub(crate) name: String,
    pub(crate) sound_delay: u64,
    #[serde(default)]
    pub(crate) channels: HashMap<String, ChannelSettings>,
    #[serde(default)]
    pub(crate) intros: HashMap<String, Intro>,
    #[serde(default)]
    pub(crate) users: HashMap<String, GuildUser>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuildUser {
    pub(crate) permissions: auth::Permissions,
}

pub(crate) trait IntroFriendlyName {
    fn friendly_name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Intro {
    File(FileIntro),
    Online(OnlineIntro),
}

impl IntroFriendlyName for Intro {
    fn friendly_name(&self) -> &str {
        match self {
            Self::File(intro) => intro.friendly_name(),
            Self::Online(intro) => intro.friendly_name(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileIntro {
    pub(crate) filename: String,
    pub(crate) friendly_name: String,
}

impl IntroFriendlyName for FileIntro {
    fn friendly_name(&self) -> &str {
        &self.friendly_name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnlineIntro {
    pub(crate) url: String,
    pub(crate) friendly_name: String,
}

impl IntroFriendlyName for OnlineIntro {
    fn friendly_name(&self) -> &str {
        &self.friendly_name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelSettings {
    #[serde(alias = "enterUsers")]
    pub(crate) users: HashMap<String, UserSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntroIndex {
    pub(crate) index: String,
    pub(crate) volume: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserSettings {
    pub(crate) intros: Vec<IntroIndex>,
}
