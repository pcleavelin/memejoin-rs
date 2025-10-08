use uuid::Uuid;

use crate::domain::intro_tool::{
    models::guild::{self, GetUserError, GuildId, IntroId, User},
    ports::{IntroToolRepository, IntroToolService, LocalAudioFetcher, RemoteAudioFetcher},
};

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

    async fn get_guild(
        &self,
        guild_id: impl Into<GuildId>,
    ) -> Result<guild::Guild, guild::GetGuildError> {
        self.repo.get_guild(guild_id.into()).await
    }

    async fn get_guild_users(&self, guild_id: GuildId) -> Result<Vec<User>, GetUserError> {
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

    async fn get_user_from_api_key(&self, api_key: &str) -> Result<User, GetUserError> {
        self.repo.get_user_from_api_key(api_key).await
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
        self.repo.create_user(req).await
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

    async fn add_intro_to_user(
        &self,
        req: guild::AddIntroToUserRequest,
    ) -> Result<(), guild::AddIntroToUserError> {
        self.repo.add_intro_to_user(req).await
    }
}
