use chrono::{Duration, NaiveDateTime, Utc};
use iter_tools::Itertools;
use uuid::Uuid;

use crate::{
    auth::{AppPermission, AppPermissions, Permissions},
    domain::intro_tool::{
        models::guild::{
            self, ApiToken, AutheticateUserError, CreateUserRequest, ExternalGuild, GetUserError,
            GuildId, IntroId, User,
        },
        ports::{
            ExternalUser, IntroToolRepository, IntroToolService, LocalAudioFetcher,
            RemoteAudioFetcher,
        },
    },
};

use super::ports::AuthService;

#[derive(Clone)]
pub struct Service<R, RA, LA>
where
    R: IntroToolRepository,
    RA: RemoteAudioFetcher,
    LA: LocalAudioFetcher,
{
    repo: R,
    remote_audio_fetcher: RA,
    local_audio_fetcher: LA,
}

impl<R, RA, LA> Service<R, RA, LA>
where
    R: IntroToolRepository,
    RA: RemoteAudioFetcher,
    LA: LocalAudioFetcher,
{
    pub fn new(repo: R, remote_audio_fetcher: RA, local_audio_fetcher: LA) -> Self {
        Self {
            repo,
            remote_audio_fetcher,
            local_audio_fetcher,
        }
    }
}

impl<R, RA, LA> IntroToolService for Service<R, RA, LA>
where
    R: IntroToolRepository,
    RA: RemoteAudioFetcher,
    LA: LocalAudioFetcher,
{
    async fn needs_setup(&self) -> bool {
        let Ok(guild_count) = self.repo.get_guild_count().await else {
            return false;
        };

        guild_count == 0
    }

    async fn authenticate_user<A: AuthService>(
        &self,
        params: A::Params,
    ) -> Result<ApiToken, AutheticateUserError<A>> {
        let external_user = A::authenticate_user(params)
            .await
            .map_err(AutheticateUserError::ExternalError)?;

        let needs_setup = self.needs_setup().await;

        let guilds = self.get_guilds().await?;
        let external_user_guilds = guilds
            .iter()
            .filter(|guild| external_user.guilds().contains(&guild.external_id()))
            .collect::<Vec<_>>();

        if !needs_setup && external_user_guilds.is_empty() {
            return Err(AutheticateUserError::UserNotPartOfInstanceGuilds);
        }

        tracing::debug!("before first get_user");

        let user = match self.get_user(external_user.username()).await {
            Ok(user) => Some(user),
            Err(GetUserError::NotFound) => None,

            Err(err) => return Err(AutheticateUserError::CouldNotFetchUser(err)),
        };

        tracing::debug!("before create user and refresh_user_token");

        match user {
            Some(user) => {
                self.refresh_user_token(user.name()).await?;
            }
            None => {
                self.create_user(CreateUserRequest {
                    user: external_user.username().clone(),
                    api_key: Uuid::new_v4(),
                    expires_at: Utc::now().naive_utc() + Duration::weeks(4),
                    external_token: external_user.external_token().to_string(),
                    external_token_expires_at: external_user.external_token_expires_at(),
                })
                .await?;
            }
        }

        tracing::debug!("before get_user");

        let user = self.get_user(external_user.username()).await?;

        if needs_setup {
            self.repo
                .set_user_app_permissions(user.name(), AppPermissions(AppPermission::all()))
                .await?;
        }

        tracing::debug!("before get_user_guilds");
        let user_guilds = self.get_user_guilds(user.name()).await?;

        tracing::debug!("before refresh_user_external_token");
        self.refresh_user_external_token(
            user.name(),
            external_user.external_token(),
            external_user.external_token_expires_at(),
        )
        .await?;

        let guilds_to_add_user =
            user_guilds
                .iter()
                .map(|guild| guild.id())
                .filter(|user_guild_id| {
                    external_user_guilds
                        .iter()
                        .map(|external_guild| external_guild.id())
                        .contains(user_guild_id)
                });

        tracing::debug!("before add_user_to_guild");
        for guild in guilds_to_add_user {
            self.add_user_to_guild(guild, user.name()).await?;
        }

        Ok(user.api_key().to_string().into())
    }

    async fn get_guild(
        &self,
        guild_id: impl Into<GuildId>,
    ) -> Result<guild::Guild, guild::GetGuildError> {
        self.repo.get_guild(guild_id.into()).await
    }

    async fn get_guilds(&self) -> Result<Vec<guild::GuildRef>, guild::GetGuildError> {
        self.repo.get_guilds().await
    }

    async fn get_guild_users(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<(User, Permissions)>, GetUserError> {
        self.repo.get_guild_users(guild_id).await
    }

    async fn get_guild_intros(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<guild::Intro>, guild::GetIntroError> {
        self.repo.get_guild_intros(guild_id).await
    }

    async fn get_user(
        &self,
        username: impl AsRef<str> + Send,
    ) -> Result<guild::User, guild::GetUserError> {
        self.repo.get_user(username).await
    }

    async fn get_user_guilds(
        &self,
        username: impl AsRef<str> + Send,
    ) -> Result<Vec<guild::GuildRef>, guild::GetGuildError> {
        self.repo.get_user_guilds(username).await
    }

    async fn get_user_external_guilds<A: AuthService>(
        &self,
        req: <A as AuthService>::ListGuildsRequest,
    ) -> Result<Vec<ExternalGuild>, A::Error> {
        A::get_guilds(req).await
    }

    async fn get_user_from_api_key(&self, api_key: &str) -> Result<User, GetUserError> {
        self.repo.get_user_from_api_key(api_key).await
    }

    async fn set_user_guild_permissions(
        &self,
        username: &str,
        guild_id: GuildId,
        permissions: Permissions,
    ) -> Result<(), GetUserError> {
        self.repo
            .set_user_guild_permissions(username, guild_id, permissions)
            .await
    }

    async fn set_user_intro(
        &self,
        req: guild::AddIntroToUserRequest,
    ) -> Result<(), guild::AddIntroToUserError> {
        self.repo.set_user_intro(req).await
    }

    async fn refresh_user_token(&self, username: &str) -> Result<String, GetUserError> {
        let user = self.get_user(username).await?;

        let user_token = if user.api_key_expires_at() >= Utc::now().naive_utc() {
            user.api_key().to_string()
        } else {
            Uuid::new_v4().to_string()
        };

        let expires_at = Utc::now().naive_utc() + Duration::weeks(4);

        self.repo
            .set_user_api_key(username, &user_token, expires_at)
            .await?;

        Ok(user_token)
    }

    async fn refresh_user_external_token(
        &self,
        username: &str,
        token: &str,
        expires_at: NaiveDateTime,
    ) -> Result<(), GetUserError> {
        self.repo
            .set_user_external_token(username, token, expires_at)
            .await?;

        Ok(())
    }

    async fn create_guild(
        &self,
        req: guild::CreateGuildRequest,
    ) -> Result<guild::Guild, guild::CreateGuildError> {
        self.repo.create_guild(req).await
    }

    async fn create_user(
        &self,
        req: guild::CreateUserRequest,
    ) -> Result<guild::User, guild::CreateUserError> {
        let username = req.user.clone();

        self.repo.create_user(req).await?;

        Ok(self.get_user(username.as_ref()).await?)
    }

    async fn add_user_to_guild(
        &self,
        guild_id: GuildId,
        username: &str,
    ) -> Result<(), guild::AddUserToGuildError> {
        self.repo.add_user_to_guild(guild_id, username).await
    }

    async fn create_channel(
        &self,
        req: guild::CreateChannelRequest,
    ) -> Result<guild::Channel, guild::CreateChannelError> {
        self.repo.create_channel(req).await
    }

    async fn add_intro_to_guild(
        &self,
        req: guild::AddIntroToGuildRequest,
    ) -> Result<IntroId, guild::AddIntroToGuildError> {
        let file_name = match &req.data {
            guild::IntroRequestData::Data(bytes) => {
                self.local_audio_fetcher
                    .save_local_audio(bytes, Uuid::new_v4().to_string().as_str())
                    .await?
            }
            guild::IntroRequestData::Url(url) => {
                self.remote_audio_fetcher
                    .fetch_remote_audio(url, Uuid::new_v4().to_string().as_str())
                    .await?
            }
        };

        self.repo
            .add_intro_to_guild(&req.name, req.guild_id, file_name)
            .await
    }
}
