use anyhow::{anyhow, Context};
use uuid::Uuid;

use crate::{
    lib::domain::intro_tool::{
        models::guild::{self, GetUserError, GuildId, IntroId, User},
        ports::{IntroToolRepository, IntroToolService},
    },
    media,
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
                // TODO: put this behind an interface
                let uuid = Uuid::new_v4().to_string();
                let temp_path = format!("./sounds/temp/{uuid}");
                let dest_path = format!("./sounds/{uuid}.mp3");

                // Write original file so its ready for codec conversion
                std::fs::write(&temp_path, bytes).context("failed to write temp file")?;
                media::normalize(&temp_path, &dest_path)
                    .await
                    .context("failed to normalize file")?;
                std::fs::remove_file(&temp_path).context("failed to remove temp file")?;

                dest_path
            }
            guild::IntroRequestData::Url(url) => {
                let uuid = Uuid::new_v4().to_string();
                let file_name = format!("sounds/{uuid}");

                // TODO: put this behind an interface
                let child = tokio::process::Command::new("yt-dlp")
                    .arg(url)
                    .args(["-o", &file_name])
                    .args(["-x", "--audio-format", "mp3"])
                    .spawn()
                    .context("failed to spawn yt-dlp process")?
                    .wait()
                    .await
                    .context("yt-dlp process failed")?;

                if !child.success() {
                    return Err(guild::AddIntroToGuildError::Unknown(anyhow!(
                        "yt-dlp terminated unsuccessfully"
                    )));
                }

                file_name
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
