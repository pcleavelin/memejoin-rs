use crate::lib::domain::intro_tool::{
    models::guild::{
        self, AddIntroToGuildError, AddIntroToGuildRequest, AddIntroToUserRequest, Channel,
        CreateChannelError, CreateChannelRequest, CreateGuildError, CreateGuildRequest,
        CreateUserError, CreateUserRequest, GetGuildError, Guild, GuildId, User,
    },
    ports::IntroToolRepository,
};

pub struct Sqlite {}

impl Sqlite {
    pub fn new(path: &str) -> Result<Self, std::io::Error> {
        todo!()
    }
}

impl IntroToolRepository for Sqlite {
    async fn get_guild(&self, guild_id: GuildId) -> Result<Guild, GetGuildError> {
        todo!()
    }

    async fn create_guild(&self, req: CreateGuildRequest) -> Result<Guild, CreateGuildError> {
        todo!()
    }

    async fn create_user(&self, req: CreateUserRequest) -> Result<User, CreateUserError> {
        todo!()
    }

    async fn create_channel(
        &self,
        req: CreateChannelRequest,
    ) -> Result<Channel, CreateChannelError> {
        todo!()
    }

    async fn add_intro_to_guild(
        &self,
        req: AddIntroToGuildRequest,
    ) -> Result<(), AddIntroToGuildError> {
        todo!()
    }

    async fn add_intro_to_user(
        &self,
        req: AddIntroToUserRequest,
    ) -> Result<(), guild::AddIntroToUserError> {
        todo!()
    }
}
