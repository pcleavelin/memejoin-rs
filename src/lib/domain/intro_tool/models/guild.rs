use std::collections::HashMap;

use thiserror::Error;

pub struct GuildId(u64);
pub struct ExternalGuildId(u64);
pub struct UserName(String);
pub struct ChannelName(String);
pub struct IntroId(i32);

pub struct Guild {
    id: GuildId,

    name: String,
    sound_delay: u32,
    external_id: ExternalGuildId,

    channels: Vec<Channel>,
    users: Vec<User>,
}

pub struct User {
    user: UserName,
    channel_intros: HashMap<ChannelName, Vec<Intro>>,
}

pub struct Channel {
    name: ChannelName,
}

pub struct Intro {
    id: IntroId,

    name: String,
    filename: String,
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
    guild_id: GuildId,
    name: String,
    volume: i32,
    filename: String,
}

pub struct AddIntroToUserRequest {
    user: UserName,
    guild_id: GuildId,
    channel_name: ChannelName,
    intro_id: IntroId,
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

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
