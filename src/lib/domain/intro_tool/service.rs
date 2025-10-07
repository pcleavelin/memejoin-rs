use crate::lib::domain::intro_tool::ports::{IntroToolRepository, IntroToolService};

use super::models;

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
