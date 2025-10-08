use crate::lib::domain::intro_tool::{
    models::guild::{GetUserError, GuildId, User},
    ports::{IntroToolRepository, IntroToolService},
};

use super::models;

#[derive(Clone)]
pub struct Service<R>
where
    R: IntroToolRepository,
{
    repo: R,
}

impl<R> Service<R>
where
    R: IntroToolRepository,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R> IntroToolService for Service<R>
where
    R: IntroToolRepository,
{
    async fn needs_setup(&self) -> bool {
        let Ok(guild_count) = self.repo.get_guild_count().await else {
            return false;
        };

        guild_count == 0
    }

    async fn get_guild(
        &self,
        guild_id: impl Into<GuildId>,
    ) -> Result<models::guild::Guild, models::guild::GetGuildError> {
        self.repo.get_guild(guild_id.into()).await
    }

    async fn get_guild_users(&self, guild_id: GuildId) -> Result<Vec<User>, GetUserError> {
        self.repo.get_guild_users(guild_id).await
    }
    async fn get_guild_intros(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<models::guild::Intro>, models::guild::GetIntroError> {
        self.repo.get_guild_intros(guild_id).await
    }

    async fn get_user(
        &self,
        username: impl AsRef<str> + Send,
    ) -> Result<models::guild::User, models::guild::GetUserError> {
        self.repo.get_user(username).await
    }

    async fn get_user_guilds(
        &self,
        username: impl AsRef<str> + Send,
    ) -> Result<Vec<models::guild::GuildRef>, models::guild::GetGuildError> {
        self.repo.get_user_guilds(username).await
    }

    async fn get_user_from_api_key(&self, api_key: &str) -> Result<User, GetUserError> {
        self.repo.get_user_from_api_key(api_key).await
    }

    async fn create_guild(
        &self,
        req: models::guild::CreateGuildRequest,
    ) -> Result<models::guild::Guild, models::guild::CreateGuildError> {
        self.repo.create_guild(req).await
    }

    async fn create_user(
        &self,
        req: models::guild::CreateUserRequest,
    ) -> Result<models::guild::User, models::guild::CreateUserError> {
        self.repo.create_user(req).await
    }

    async fn create_channel(
        &self,
        req: models::guild::CreateChannelRequest,
    ) -> Result<models::guild::Channel, models::guild::CreateChannelError> {
        self.repo.create_channel(req).await
    }

    async fn add_intro_to_guild(
        &self,
        req: models::guild::AddIntroToGuildRequest,
    ) -> Result<(), models::guild::AddIntroToGuildError> {
        self.repo.add_intro_to_guild(req).await
    }

    async fn add_intro_to_user(
        &self,
        req: models::guild::AddIntroToUserRequest,
    ) -> Result<(), models::guild::AddIntroToUserError> {
        self.repo.add_intro_to_user(req).await
    }
}
