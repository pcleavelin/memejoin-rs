use std::{collections::HashMap, future::Future};

use crate::lib::domain::intro_tool::models::guild::ChannelName;

use super::models::guild::{
    AddIntroToGuildError, AddIntroToGuildRequest, AddIntroToUserError, AddIntroToUserRequest,
    Channel, CreateChannelError, CreateChannelRequest, CreateGuildError, CreateGuildRequest,
    CreateUserError, CreateUserRequest, GetChannelError, GetGuildError, GetIntroError,
    GetUserError, Guild, GuildId, GuildRef, Intro, User,
};

pub trait IntroToolService: Send + Sync + Clone + 'static {
    fn needs_setup(&self) -> impl Future<Output = bool> + Send;

    fn get_guild(
        &self,
        guild_id: impl Into<GuildId> + Send,
    ) -> impl Future<Output = Result<Guild, GetGuildError>> + Send;
    fn get_guild_users(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<User>, GetUserError>> + Send;
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
    fn get_user_from_api_key(
        &self,
        api_key: &str,
    ) -> impl Future<Output = Result<User, GetUserError>> + Send;

    async fn create_guild(&self, req: CreateGuildRequest) -> Result<Guild, CreateGuildError>;
    async fn create_user(&self, req: CreateUserRequest) -> Result<User, CreateUserError>;
    async fn create_channel(
        &self,
        req: CreateChannelRequest,
    ) -> Result<Channel, CreateChannelError>;

    async fn add_intro_to_guild(
        &self,
        req: AddIntroToGuildRequest,
    ) -> Result<(), AddIntroToGuildError>;

    async fn add_intro_to_user(
        &self,
        req: AddIntroToUserRequest,
    ) -> Result<(), AddIntroToUserError>;
}

pub trait IntroToolRepository: Send + Sync + Clone + 'static {
    fn get_guild(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Guild, GetGuildError>> + Send;
    fn get_guild_count(&self) -> impl Future<Output = Result<usize, GetGuildError>> + Send;

    fn get_guild_users(
        &self,
        guild_id: GuildId,
    ) -> impl Future<Output = Result<Vec<User>, GetUserError>> + Send;

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

    async fn create_guild(&self, req: CreateGuildRequest) -> Result<Guild, CreateGuildError>;
    async fn create_user(&self, req: CreateUserRequest) -> Result<User, CreateUserError>;
    async fn create_channel(
        &self,
        req: CreateChannelRequest,
    ) -> Result<Channel, CreateChannelError>;

    async fn add_intro_to_guild(
        &self,
        req: AddIntroToGuildRequest,
    ) -> Result<(), AddIntroToGuildError>;

    async fn add_intro_to_user(
        &self,
        req: AddIntroToUserRequest,
    ) -> Result<(), AddIntroToUserError>;
}
