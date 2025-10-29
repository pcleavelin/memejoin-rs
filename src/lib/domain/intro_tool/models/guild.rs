use std::{borrow::Cow, collections::HashMap};

use chrono::NaiveDateTime;
use thiserror::Error;

use crate::{
    auth::{AppPermissions, Permissions},
    domain::intro_tool::ports::AuthService,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiToken(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GuildId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternalGuildId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelName(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntroId(i32);

impl From<ApiToken> for Cow<'_, str> {
    fn from(value: ApiToken) -> Self {
        Cow::Owned(value.0)
    }
}

impl From<String> for ApiToken {
    fn from(value: String) -> Self {
        Self(value)
    }
}

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
    users: Vec<(User, Permissions)>,
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

    pub fn external_id(&self) -> ExternalGuildId {
        self.external_id
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

    pub fn users(&self) -> &[(User, Permissions)] {
        &self.users
    }

    pub fn channels(&self) -> &[Channel] {
        &self.channels
    }

    pub fn with_users(self, users: Vec<(User, Permissions)>) -> Self {
        Self { users, ..self }
    }

    pub fn with_channels(self, channels: Vec<Channel>) -> Self {
        Self { channels, ..self }
    }
}

pub struct ExternalGuild {
    pub id: ExternalGuildId,
    pub name: String,
}

#[derive(Debug)]
pub struct User {
    name: UserName,

    permissions: AppPermissions,

    api_key: String,
    api_key_expires_at: NaiveDateTime,
    discord_token: String,
    discord_token_expires_at: NaiveDateTime,

    channel_intros: HashMap<(GuildId, ChannelName), Vec<Intro>>,
}

impl User {
    pub fn new(
        name: impl Into<UserName>,
        permissions: AppPermissions,
        api_key: String,
        api_key_expires_at: NaiveDateTime,
        discord_token: String,
        discord_token_expires_at: NaiveDateTime,
    ) -> Self {
        Self {
            name: name.into(),
            permissions,
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

    pub fn permissions(&self) -> AppPermissions {
        self.permissions
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn api_key_expires_at(&self) -> NaiveDateTime {
        self.api_key_expires_at
    }

    pub fn external_token(&self) -> &str {
        &self.discord_token
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
    pub name: String,
    pub sound_delay: u32,
    pub external_id: ExternalGuildId,
}

pub struct CreateUserRequest {
    pub user: UserName,
}

pub struct CreateChannelRequest {
    pub guild_id: GuildId,
    pub channel_name: ChannelName,
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
    #[error("Could not get user")]
    CouldNotGetUser(#[from] GetUserError),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum CreateChannelError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum AddUserToGuildError {
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

#[derive(Debug, Error)]
pub enum AutheticateUserError<A: AuthService> {
    #[error("Could not fetch guild")]
    CouldNotFetchGuild(#[from] GetGuildError),

    #[error("Could not create user")]
    CouldNotCreateUser(#[from] CreateUserError),

    #[error("Could not fetch guild user")]
    CouldNotFetchUser(#[from] GetUserError),

    #[error("Could not add user to guild")]
    CouldNotAddUserToGuild(#[from] AddUserToGuildError),

    #[error("User not part of instance's guilds")]
    UserNotPartOfInstanceGuilds,

    #[error("Error authenticating user")]
    ExternalError(A::Error),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
