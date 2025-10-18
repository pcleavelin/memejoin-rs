use std::collections::HashMap;

use chrono::NaiveDateTime;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GuildId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternalGuildId(u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelName(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntroId(i32);

impl From<u64> for GuildId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<u64> for ExternalGuildId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<i32> for IntroId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<String> for UserName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<String> for ChannelName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ChannelName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GuildId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for UserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for ChannelName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for IntroId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct Guild {
    guild: GuildRef,

    channels: Vec<Channel>,
    users: Vec<User>,
}

#[derive(Debug)]
pub struct GuildRef {
    id: GuildId,
    name: String,
    sound_delay: u32,
    external_id: ExternalGuildId,
}

impl GuildRef {
    pub fn id(&self) -> GuildId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl GuildRef {
    pub fn new(id: GuildId, name: String, sound_delay: u32, external_id: ExternalGuildId) -> Self {
        Self {
            id,
            name,
            sound_delay,
            external_id,
        }
    }
}

impl Guild {
    pub fn new(id: GuildId, name: String, sound_delay: u32, external_id: ExternalGuildId) -> Self {
        Self {
            guild: GuildRef {
                id,
                name,
                sound_delay,
                external_id,
            },
            channels: vec![],
            users: vec![],
        }
    }

    pub fn id(&self) -> GuildId {
        self.guild.id()
    }

    pub fn name(&self) -> &str {
        self.guild.name()
    }

    pub fn users(&self) -> &[User] {
        &self.users
    }

    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    pub fn with_users(self, users: Vec<User>) -> Self {
        Self { users, ..self }
    }

    pub fn with_channels(self, channels: Vec<Channel>) -> Self {
        Self { channels, ..self }
    }
}

#[derive(Debug)]
pub struct User {
    name: UserName,

    api_key: String,
    api_key_expires_at: NaiveDateTime,
    discord_token: String,
    discord_token_expires_at: NaiveDateTime,

    channel_intros: HashMap<(GuildId, ChannelName), Vec<Intro>>,
}

impl User {
    pub fn new(
        name: impl Into<UserName>,
        api_key: String,
        api_key_expires_at: NaiveDateTime,
        discord_token: String,
        discord_token_expires_at: NaiveDateTime,
    ) -> Self {
        Self {
            name: name.into(),
            api_key,
            api_key_expires_at,
            discord_token,
            discord_token_expires_at,
            channel_intros: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name.0
    }

    pub fn intros(&self) -> &HashMap<(GuildId, ChannelName), Vec<Intro>> {
        &self.channel_intros
    }

    pub fn api_key_expires_at(&self) -> NaiveDateTime {
        self.api_key_expires_at
    }

    pub fn discord_token_expires_at(&self) -> NaiveDateTime {
        self.discord_token_expires_at
    }

    pub fn with_channel_intros(
        self,
        channel_intros: HashMap<(GuildId, ChannelName), Vec<Intro>>,
    ) -> Self {
        Self {
            channel_intros,
            ..self
        }
    }
}

#[derive(Debug)]
pub struct Channel {
    name: ChannelName,
}

impl Channel {
    pub fn new(name: ChannelName) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &ChannelName {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct Intro {
    id: IntroId,

    name: String,
    filename: String,
}

impl Intro {
    pub fn new(id: IntroId, name: String, filename: String) -> Self {
        Self { id, name, filename }
    }

    pub fn id(&self) -> IntroId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct CreateGuildRequest {
    name: String,
    sound_delay: u32,
    external_id: ExternalGuildId,
}

pub struct CreateUserRequest {
    user: UserName,
}

pub struct CreateChannelRequest {
    guild_id: GuildId,
    channel_name: ChannelName,
}

pub struct AddIntroToGuildRequest {
    pub guild_id: GuildId,
    pub name: String,
    pub volume: i32,

    pub data: IntroRequestData,
}

pub enum IntroRequestData {
    Data(Vec<u8>),
    Url(String),
}

pub struct AddIntroToUserRequest {
    pub user: UserName,
    pub guild_id: GuildId,
    pub channel_name: ChannelName,
    pub intro_id: IntroId,
}

#[derive(Debug, Error)]
pub enum CreateGuildError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum CreateUserError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum CreateChannelError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum AddIntroToGuildError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum AddIntroToUserError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetGuildError {
    #[error("Guild not found")]
    NotFound,

    #[error("Could not fetch guild users")]
    CouldNotFetchUsers(#[from] GetUserError),

    #[error("Could not fetch guild channels")]
    CouldNotFetchChannels(#[from] GetChannelError),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetUserError {
    #[error("User not found")]
    NotFound,

    #[error("Could not fetch user guilds")]
    CouldNotFetchGuilds(#[from] Box<GetGuildError>),

    #[error("Could not fetch user channel intros")]
    CouldNotFetchChannelIntros(#[from] GetIntroError),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetChannelError {
    #[error("Channel not found")]
    NotFound,

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum GetIntroError {
    #[error("Intro not found")]
    NotFound,

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
