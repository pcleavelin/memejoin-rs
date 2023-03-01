use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) run_api: bool,
    #[serde(default)]
    pub(crate) run_bot: bool,
    pub(crate) guilds: HashMap<u64, GuildSettings>,
}
impl TypeMapKey for Settings {
    type Value = Arc<Settings>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuildSettings {
    #[serde(alias = "userEnteredSoundDelay")]
    pub(crate) sound_delay: u64,
    pub(crate) channels: HashMap<String, ChannelSettings>,
    pub(crate) intros: Vec<Intro>,
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
    pub(crate) index: usize,
    pub(crate) volume: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserSettings {
    pub(crate) intros: Vec<IntroIndex>,
}
