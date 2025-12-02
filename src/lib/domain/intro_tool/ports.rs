use std::{collections::HashMap, future::Future};

use chrono::NaiveDateTime;

use crate::{
    auth::{AppPermissions, Permissions},
    domain::intro_tool::models::guild::{
        AddUserToGuildError, AutheticateUserError, ExternalChannel, ExternalGuild, ExternalGuildId,
        UserName,
    },
};

use super::models::guild::{
    AddIntroToGuildError, AddIntroToGuildRequest, AddIntroToUserError, AddIntroToUserRequest,
    ApiToken, Channel, ChannelName, CreateChannelError, CreateChannelRequest, CreateGuildError,
    CreateGuildRequest, CreateUserError, CreateUserRequest, GetChannelError, GetGuildError,
    GetIntroError, GetUserError, Guild, GuildId, GuildRef, Intro, IntroId, User,
};

pub trait IntroToolService: Send + Sync + Clone + 'static {
    fn needs_setup(&self) -> impl Future<Output = bool> + Send;

    fn authenticate_user<A: AuthService>(
        &self,
        params: A::Params,
    ) -> impl Future<Output = Result<ApiToken, AutheticateUserError<A>>> + Send;

    fn get_guild(
        &self,
        guild_id: impl Into<GuildId> + Send,
    ) -> impl Future<Output = Result<Guild, GetGuildError>> + Send;
    fn get_guilds(&self) -> impl Future<Output = Result<Vec<GuildRef>, GetGuildError>> + Send;
    fn get_guild_channels(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Channel>, GetChannelError>> + Send;
    fn get_guild_users(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<(User, Permissions)>, GetUserError>> + Send;
    fn get_guild_intros(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Intro>, GetIntroError>> + Send;
    fn get_user(
        &self,
        username: impl AsRef<str> + Send,
    ) -> impl Future<Output = Result<User, GetUserError>> + Send;
    fn get_user_guilds(
        &self,
        username: impl AsRef<str> + Send,
    ) -> impl Future<Output = Result<Vec<GuildRef>, GetGuildError>> + Send;
    fn get_user_external_guilds<A: AuthService>(
        &self,
        req: A::ListGuildsRequest,
    ) -> impl Future<Output = Result<Vec<ExternalGuild>, A::Error>> + Send;
    fn get_external_guild_channels<A: AuthService>(
        &self,
        req: A::ListGuildChannelsRequest,
    ) -> impl Future<Output = Result<Vec<ExternalChannel>, A::Error>> + Send;
    fn get_user_from_api_key(
        &self,
        api_key: &str,
    ) -> impl Future<Output = Result<User, GetUserError>> + Send;

    fn set_user_guild_permissions(
        &self,
        username: &str,
        guild_id: GuildId,
        permissions: Permissions,
    ) -> impl Future<Output = Result<(), GetUserError>> + Send;

    fn set_user_intro(
        &self,
        req: AddIntroToUserRequest,
    ) -> impl Future<Output = Result<(), AddIntroToUserError>> + Send;

    fn refresh_user_token(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<String, GetUserError>> + Send;
    fn refresh_user_external_token(
        &self,
        username: &str,
        token: &str,
        expires_at: NaiveDateTime,
    ) -> impl Future<Output = Result<(), GetUserError>> + Send;

    fn create_guild(
        &self,
        req: CreateGuildRequest,
    ) -> impl Future<Output = Result<Guild, CreateGuildError>> + Send;

    fn create_user(
        &self,
        req: CreateUserRequest,
    ) -> impl Future<Output = Result<User, CreateUserError>> + Send;

    fn add_user_to_guild(
        &self,
        guild_id: GuildId,
        username: &str,
    ) -> impl Future<Output = Result<(), AddUserToGuildError>> + Send;

    fn create_channels(
        &self,
        req: CreateChannelRequest,
    ) -> impl Future<Output = Result<(), CreateChannelError>> + Send;

    fn add_intro_to_guild(
        &self,
        req: AddIntroToGuildRequest,
    ) -> impl Future<Output = Result<IntroId, AddIntroToGuildError>> + Send;
}

pub trait IntroToolRepository: Send + Sync + Clone + 'static {
    fn get_guild(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Guild, GetGuildError>> + Send;
    fn get_guilds(&self) -> impl Future<Output = Result<Vec<GuildRef>, GetGuildError>> + Send;
    fn get_guild_count(&self) -> impl Future<Output = Result<usize, GetGuildError>> + Send;

    fn get_guild_users(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<(User, Permissions)>, GetUserError>> + Send;

    fn get_guild_channels(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Channel>, GetChannelError>> + Send;
    fn get_guild_intros(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<Intro>, GetIntroError>> + Send;

    fn get_user(
        &self,
        username: impl AsRef<str> + Send,
    ) -> impl Future<Output = Result<User, GetUserError>> + Send;

    fn get_user_channel_intros(
        &self,
        username: impl AsRef<str> + Send,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<HashMap<(GuildId, ChannelName), Vec<Intro>>, GetIntroError>> + Send;

    fn get_user_guilds(
        &self,
        username: impl AsRef<str> + Send,
    ) -> impl Future<Output = Result<Vec<GuildRef>, GetGuildError>> + Send;

    fn get_user_from_api_key(
        &self,
        api_key: &str,
    ) -> impl Future<Output = Result<User, GetUserError>> + Send;

    fn set_user_api_key(
        &self,
        username: &str,
        api_key: &str,
        expires_at: NaiveDateTime,
    ) -> impl Future<Output = Result<(), GetUserError>> + Send;

    fn set_user_external_token(
        &self,
        username: &str,
        token: &str,
        expires_at: NaiveDateTime,
    ) -> impl Future<Output = Result<(), GetUserError>> + Send;

    fn set_user_app_permissions(
        &self,
        username: &str,
        app_permissions: AppPermissions,
    ) -> impl Future<Output = Result<(), GetUserError>> + Send;

    fn set_user_guild_permissions(
        &self,
        username: &str,
        guild_id: GuildId,
        permissions: Permissions,
    ) -> impl Future<Output = Result<(), GetUserError>> + Send;

    fn set_user_intro(
        &self,
        req: AddIntroToUserRequest,
    ) -> impl Future<Output = Result<(), AddIntroToUserError>> + Send;

    fn create_guild(
        &self,
        req: CreateGuildRequest,
    ) -> impl Future<Output = Result<Guild, CreateGuildError>> + Send;

    fn create_user(
        &self,
        req: CreateUserRequest,
    ) -> impl Future<Output = Result<(), CreateUserError>> + Send;

    fn add_user_to_guild(
        &self,
        guild_id: GuildId,
        username: &str,
    ) -> impl Future<Output = Result<(), AddUserToGuildError>> + Send;

    fn create_channels(
        &self,
        req: CreateChannelRequest,
    ) -> impl Future<Output = Result<(), CreateChannelError>> + Send;

    fn add_intro_to_guild(
        &self,
        name: &str,
        guild_id: GuildId,
        filename: String,
    ) -> impl Future<Output = Result<IntroId, AddIntroToGuildError>> + Send;
}

pub trait ExternalUser: Send + Sync + Clone + 'static {
    fn external_token(&self) -> &str;
    fn external_token_expires_at(&self) -> NaiveDateTime;
    fn username(&self) -> UserName;
    fn guilds(&self) -> impl Iterator<Item = ExternalGuildId>;
}
pub trait AuthService: Send + Sync + Clone + 'static {
    type Params: Send;
    type User: ExternalUser + Send;
    type Error: std::error::Error + Send;

    type Channel: Send;

    type ListGuildsRequest: Send;
    type ListGuildChannelsRequest: Send;

    fn authenticate_user(
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::User, Self::Error>> + Send;

    fn get_guilds(
        req: Self::ListGuildsRequest,
    ) -> impl Future<Output = Result<Vec<ExternalGuild>, Self::Error>> + Send;

    fn get_guild_channels(
        req: Self::ListGuildChannelsRequest,
    ) -> impl Future<Output = Result<Vec<ExternalChannel>, Self::Error>> + Send;
}

pub trait RemoteAudioFetcher: Send + Sync + Clone + 'static {
    fn fetch_remote_audio(
        &self,
        url: &str,
        name: &str,
    ) -> impl Future<Output = Result<String, anyhow::Error>> + Send;
}

pub trait LocalAudioFetcher: Send + Sync + Clone + 'static {
    fn save_local_audio(
        &self,
        bytes: &[u8],
        name: &str,
    ) -> impl Future<Output = Result<String, anyhow::Error>> + Send;
}
