use std::{collections::HashMap, sync::Arc};

use crate::auth;
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use tracing::trace;
use uuid::Uuid;

type UserToken = String;

pub(crate) struct ApiState {
    pub settings: Arc<tokio::sync::Mutex<Settings>>,
    pub secrets: auth::DiscordSecret,
    pub origin: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Intro {
    File(FileIntro),
    Online(OnlineIntro),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileIntro {
    pub(crate) filename: String,
    pub(crate) friendly_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnlineIntro {
    pub(crate) url: String,
    pub(crate) friendly_name: String,
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
