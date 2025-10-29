use chrono::{Duration, NaiveDateTime, Utc};

use crate::{
    auth::AppPermissions,
    domain::intro_tool::{
        models::{self, guild::IntroId},
        ports::IntroToolService,
    },
};

use super::ports::AuthService;

#[derive(Clone)]
pub struct DebugService<S>
where
    S: IntroToolService,
{
    impersonated_username: String,
    permissions: AppPermissions,
    wrapped_service: S,
}

impl<S> DebugService<S>
where
    S: IntroToolService,
{
    pub fn new(
        wrapped_service: S,
        impersonated_username: String,
        permissions: AppPermissions,
    ) -> Self {
        Self {
            wrapped_service,
            impersonated_username,
            permissions,
        }
    }
}

impl<S> IntroToolService for DebugService<S>
where
    S: IntroToolService,
{
    async fn needs_setup(&self) -> bool {
        self.wrapped_service.needs_setup().await
    }

    async fn authenticate_user<A: AuthService>(
        &self,
        params: A::Params,
    ) -> Result<models::guild::ApiToken, models::guild::AutheticateUserError<A>> {
        self.wrapped_service.authenticate_user(params).await
    }

    async fn get_guild(
        &self,
        guild_id: impl Into<models::guild::GuildId> + Send,
    ) -> Result<models::guild::Guild, models::guild::GetGuildError> {
        self.wrapped_service.get_guild(guild_id).await
    }

    async fn get_guilds(
        &self,
    ) -> Result<Vec<models::guild::GuildRef>, models::guild::GetGuildError> {
        self.wrapped_service.get_guilds().await
    }

    async fn get_guild_users(
        &self,
        guild_id: models::guild::GuildId,
    ) -> Result<Vec<(models::guild::User, crate::auth::Permissions)>, models::guild::GetUserError>
    {
        self.wrapped_service.get_guild_users(guild_id).await
    }

    async fn get_guild_intros(
        &self,
        guild_id: models::guild::GuildId,
    ) -> Result<Vec<models::guild::Intro>, models::guild::GetIntroError> {
        self.wrapped_service.get_guild_intros(guild_id).await
    }

    async fn get_user(
        &self,
        username: impl AsRef<str> + Send,
    ) -> Result<models::guild::User, models::guild::GetUserError> {
        self.wrapped_service.get_user(username).await
    }

    async fn get_user_guilds(
        &self,
        username: impl AsRef<str> + Send,
    ) -> Result<Vec<models::guild::GuildRef>, models::guild::GetGuildError> {
        self.wrapped_service.get_user_guilds(username).await
    }

    async fn get_user_external_guilds<A: AuthService>(
        &self,
        req: <A as AuthService>::ListGuildsRequest,
    ) -> Result<Vec<models::guild::ExternalGuild>, A::Error> {
        self.wrapped_service
            .get_user_external_guilds::<A>(req)
            .await
    }

    async fn get_user_from_api_key(
        &self,
        _api_key: &str,
    ) -> Result<models::guild::User, models::guild::GetUserError> {
        let user = self
            .wrapped_service
            .get_user(&self.impersonated_username)
            .await?;

        Ok(models::guild::User::new(
            self.impersonated_username.clone(),
            self.permissions,
            "testApiKey".into(),
            Utc::now().naive_utc() + Duration::days(1),
            user.external_token().to_string(),
            Utc::now().naive_utc() + Duration::days(1),
        )
        .with_channel_intros(user.intros().clone()))
    }

    async fn set_user_intro(
        &self,
        req: models::guild::AddIntroToUserRequest,
    ) -> Result<(), models::guild::AddIntroToUserError> {
        self.wrapped_service.set_user_intro(req).await
    }

    async fn refresh_user_token(
        &self,
        username: &str,
    ) -> Result<String, models::guild::GetUserError> {
        self.wrapped_service.refresh_user_token(username).await
    }

    async fn refresh_user_external_token(
        &self,
        username: &str,
        token: &str,
        expires_at: NaiveDateTime,
    ) -> Result<(), models::guild::GetUserError> {
        self.wrapped_service
            .refresh_user_external_token(username, token, expires_at)
            .await
    }

    async fn create_guild(
        &self,
        req: models::guild::CreateGuildRequest,
    ) -> Result<models::guild::Guild, models::guild::CreateGuildError> {
        self.wrapped_service.create_guild(req).await
    }

    async fn create_user(
        &self,
        req: models::guild::CreateUserRequest,
    ) -> Result<models::guild::User, models::guild::CreateUserError> {
        self.wrapped_service.create_user(req).await
    }

    async fn add_user_to_guild(
        &self,
        guild_id: models::guild::GuildId,
        username: &str,
    ) -> Result<(), models::guild::AddUserToGuildError> {
        self.wrapped_service
            .add_user_to_guild(guild_id, username)
            .await
    }

    async fn create_channel(
        &self,
        req: models::guild::CreateChannelRequest,
    ) -> Result<models::guild::Channel, models::guild::CreateChannelError> {
        self.wrapped_service.create_channel(req).await
    }

    async fn add_intro_to_guild(
        &self,
        req: models::guild::AddIntroToGuildRequest,
    ) -> Result<IntroId, models::guild::AddIntroToGuildError> {
        self.wrapped_service.add_intro_to_guild(req).await
    }
}
