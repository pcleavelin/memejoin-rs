use super::models::guild::{
    AddIntroToGuildError, AddIntroToGuildRequest, AddIntroToUserError, AddIntroToUserRequest,
    Channel, CreateChannelError, CreateChannelRequest, CreateGuildError, CreateGuildRequest,
    CreateUserError, CreateUserRequest, GetGuildError, Guild, GuildId, User,
};

pub trait IntroToolService {
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

pub trait IntroToolRepository {
    async fn get_guild(&self, guild_id: GuildId) -> Result<Guild, GetGuildError>;

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
