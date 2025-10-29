use chrono::NaiveDateTime;
use iter_tools::Itertools;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use anyhow::Context;
use rusqlite::Connection;

use crate::{
    auth::{AppPermissions, Permissions},
    domain::intro_tool::{
        models::guild::{
            self, AddIntroToGuildError, AddIntroToGuildRequest, AddIntroToUserRequest, Channel,
            ChannelName, CreateChannelError, CreateChannelRequest, CreateGuildError,
            CreateGuildRequest, CreateUserError, CreateUserRequest, GetChannelError, GetGuildError,
            GetIntroError, GetUserError, Guild, GuildId, GuildRef, Intro, IntroId, User, UserName,
        },
        ports::IntroToolRepository,
    },
};

#[derive(Clone)]
pub struct Sqlite {
    conn: Arc<Mutex<Connection>>,
}

impl Sqlite {
    pub fn new(path: &str) -> rusqlite::Result<Self> {
        Ok(Self {
            conn: Arc::new(Mutex::new(Connection::open(path)?)),
        })
    }
}

impl IntroToolRepository for Sqlite {
    async fn get_guild(&self, guild_id: GuildId) -> Result<Guild, GetGuildError> {
        let guild = {
            let conn = self.conn.lock().await;

            let mut query = conn
                .prepare(
                    "
            select
                Guild.id,
                Guild.name,
                Guild.sound_delay
            from Guild
            where Guild.id = :guild_id
            ",
                )
                .context("failed to prepare query")?;

            query
                .query_row(&[(":guild_id", &guild_id.to_string())], |row| {
                    Ok(Guild::new(
                        row.get::<_, u64>(0)?.into(),
                        row.get(1)?,
                        row.get(2)?,
                        row.get::<_, u64>(0)?.into(),
                    ))
                })
                .context("failed to query row")?
        };

        Ok(guild
            .with_users(self.get_guild_users(guild_id).await?)
            .with_channels(self.get_guild_channels(guild_id).await?))
    }

    async fn get_guilds(&self) -> Result<Vec<GuildRef>, GetGuildError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                SELECT
                    Guild.id,
                    Guild.name,
                    Guild.sound_delay
                FROM Guild
                LEFT JOIN UserGuild ON Guild.id = UserGuild.guild_id
                LEFT JOIN User ON User.username = UserGuild.username
                ",
            )
            .context("failed to prepare query")?;

        let guilds = query
            .query_map([], |row| {
                Ok(GuildRef::new(
                    row.get::<_, u64>(0)?.into(),
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, u64>(0)?.into(),
                ))
            })
            .context("failed to map prepared query")?
            .collect::<Result<_, _>>()
            .context("failed to fetch guild user rows")?;

        Ok(guilds)
    }

    async fn get_guild_count(&self) -> Result<usize, GetGuildError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                select
                    count(*)
                from Guild
                ",
            )
            .context("failed to prepare query")?;

        Ok(query
            .query_row([], |row| row.get::<_, usize>(0))
            .context("failed to query row")?)
    }

    async fn get_guild_users(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<(User, Permissions)>, GetUserError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                SELECT
                    User.username AS name,
                    UAP.permissions,
                    UGP.permissions,
                    User.api_key,
                    User.api_key_expires_at,
                    User.discord_token,
                    User.discord_token_expires_at
                FROM UserGuild
                LEFT JOIN User ON User.username = UserGuild.username
                LEFT JOIN UserPermission UGP ON UGP.username = User.username and UGP.guild_id = UserGuild.guild_id
                LEFT JOIN UserAppPermission UAP ON UAP.username = User.username
                WHERE UserGuild.guild_id = :guild_id
                ",
            )
            .context("failed to prepare query")?;

        let users = query
            .query_map(&[(":guild_id", &guild_id.to_string())], |row| {
                Ok((
                    User::new(
                        UserName::from(row.get::<_, String>(0)?),
                        row.get::<_, Option<u8>>(1)?
                            .map(AppPermissions)
                            .unwrap_or_default(),
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ),
                    row.get::<_, Option<u8>>(2)?
                        .map(Permissions)
                        .unwrap_or_default(),
                ))
            })
            .context("failed to map prepared query")?
            .collect::<Result<_, _>>()
            .context("failed to fetch guild user rows")?;

        Ok(users)
    }

    async fn get_guild_channels(&self, guild_id: GuildId) -> Result<Vec<Channel>, GetChannelError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                SELECT
                    Channel.name
                FROM Channel
                WHERE
                    Channel.guild_id = :guild_id
                ORDER BY Channel.name DESC
                ",
            )
            .context("failed to prepare query")?;

        let channels = query
            .query_map(&[(":guild_id", &guild_id.to_string())], |row| {
                Ok(Channel::new(row.get::<_, String>(0)?.into()))
            })
            .context("failed to map prepared query")?
            .collect::<Result<_, _>>()
            .context("failed to fetch guild channel rows")?;

        Ok(channels)
    }

    async fn get_guild_intros(&self, guild_id: GuildId) -> Result<Vec<Intro>, GetIntroError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                SELECT
                    Intro.id,
                    Intro.name,
                    Intro.filename
                FROM Intro
                WHERE
                    Intro.guild_id = :guild_id
                ",
            )
            .context("failed to prepare query")?;

        let intros = query
            .query_map(&[(":guild_id", &guild_id.to_string())], |row| {
                Ok(Intro::new(
                    row.get::<_, i32>(0)?.into(),
                    row.get(1)?,
                    row.get(2)?,
                ))
            })
            .context("failed to map prepared query")?
            .collect::<Result<_, _>>()
            .context("failed to fetch guild intro rows")?;

        Ok(intros)
    }

    async fn get_user(&self, username: impl AsRef<str>) -> Result<User, GetUserError> {
        let user = {
            let conn = self.conn.lock().await;

            let mut query = conn
                .prepare(
                    "
                    SELECT
                        User.username AS name,
                        UAP.permissions,
                        api_key,
                        api_key_expires_at,
                        discord_token,
                        discord_token_expires_at
                    FROM User
                    LEFT JOIN UserAppPermission UAP ON UAP.username = User.username
                    WHERE User.username = :username
                    ",
                )
                .context("failed to prepare query")?;

            query
                .query_row(&[(":username", username.as_ref())], |row| {
                    Ok(User::new(
                        UserName::from(row.get::<_, String>(0)?),
                        row.get::<_, Option<u8>>(1)?
                            .map(AppPermissions)
                            .unwrap_or_default(),
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })
                .context("failed to query row")?
        };

        let guilds = self
            .get_user_guilds(username.as_ref())
            .await
            .map_err(Box::new)?;

        let mut intros = HashMap::new();
        for guild in guilds {
            intros.extend(
                self.get_user_channel_intros(username.as_ref(), guild.id())
                    .await?,
            );
        }

        Ok(user.with_channel_intros(intros))
    }

    async fn get_user_channel_intros(
        &self,
        username: impl AsRef<str>,
        guild_id: GuildId,
    ) -> Result<HashMap<(GuildId, ChannelName), Vec<Intro>>, GetIntroError> {
        let conn = self.conn.lock().await;

        struct ChannelIntro {
            channel_name: ChannelName,
            intro: Intro,
        }

        let mut query = conn
            .prepare(
                "
                SELECT
                    Intro.id,
                    Intro.name,
                    Intro.filename,
                    UI.channel_name
                FROM Intro
                LEFT JOIN UserIntro UI ON UI.intro_id = Intro.id
                WHERE
                    UI.username = ?1
                    AND UI.guild_id = ?2
                ",
            )
            .context("failed to prepare query")?;

        let intros = query
            .query_map([username.as_ref(), &guild_id.to_string()], |row| {
                Ok(ChannelIntro {
                    channel_name: ChannelName::from(row.get::<_, String>(3)?),
                    intro: Intro::new(row.get::<_, i32>(0)?.into(), row.get(1)?, row.get(2)?),
                })
            })
            .context("failed to map prepared query")?
            .collect::<Result<Vec<ChannelIntro>, _>>()
            .context("failed to fetch user channel intro rows")?;

        let intros = intros
            .into_iter()
            .map(|intro| ((guild_id, intro.channel_name), intro.intro))
            .into_group_map();

        Ok(intros)
    }

    async fn get_user_guilds(
        &self,
        username: impl AsRef<str>,
    ) -> Result<Vec<GuildRef>, GetGuildError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                SELECT
                    Guild.id,
                    Guild.name,
                    Guild.sound_delay
                FROM Guild
                LEFT JOIN UserGuild ON Guild.id = UserGuild.guild_id
                LEFT JOIN User ON User.username = UserGuild.username
                WHERE User.username = :username
                ",
            )
            .context("failed to prepare query")?;

        let guilds = query
            .query_map(&[(":username", username.as_ref())], |row| {
                Ok(GuildRef::new(
                    row.get::<_, u64>(0)?.into(),
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, u64>(0)?.into(),
                ))
            })
            .context("failed to map prepared query")?
            .collect::<Result<_, _>>()
            .context("failed to fetch guild user rows")?;

        Ok(guilds)
    }

    async fn get_user_from_api_key(&self, api_key: &str) -> Result<User, GetUserError> {
        let username = {
            let conn = self.conn.lock().await;

            let mut query = conn
                .prepare(
                    "
                    SELECT
                        username AS name
                    FROM User
                    WHERE api_key = :api_key
                    ",
                )
                .context("failed to prepare query")?;

            query
                .query_row(&[(":api_key", api_key)], |row| {
                    Ok(UserName::from(row.get::<_, String>(0)?))
                })
                .context("failed to query row")?
        };

        self.get_user(username).await
    }

    async fn set_user_api_key(
        &self,
        username: &str,
        api_key: &str,
        expires_at: NaiveDateTime,
    ) -> Result<(), GetUserError> {
        let conn = self.conn.lock().await;

        conn.execute(
            "
            UPDATE User
            SET api_key = ?1, api_key_expires_at = ?2
            WHERE username = ?3
            ",
            [api_key, &expires_at.to_string(), username],
        )
        .context("failed to update user api key")?;

        Ok(())
    }

    async fn set_user_external_token(
        &self,
        username: &str,
        token: &str,
        expires_at: NaiveDateTime,
    ) -> Result<(), GetUserError> {
        let conn = self.conn.lock().await;

        conn.execute(
            "
            UPDATE User
            SET discord_token = ?1, discord_token_expires_at = ?2
            WHERE username = ?3
            ",
            [token, &expires_at.to_string(), username],
        )
        .context("failed to update user external token")?;

        Ok(())
    }

    async fn set_user_intro(
        &self,
        req: AddIntroToUserRequest,
    ) -> Result<(), guild::AddIntroToUserError> {
        let conn = self.conn.lock().await;

        conn.execute(
            "
                DELETE FROM UserIntro
                WHERE username = ?1
                AND guild_id = ?2
                AND channel_name = ?3
                ",
            [
                &req.user.to_string(),
                &req.guild_id.to_string(),
                &req.channel_name.to_string(),
            ],
        )
        .context("failed to delete user intros")?;

        conn.execute(
            "
                INSERT INTO
                    UserIntro (username, guild_id, channel_name, intro_id)
                VALUES (?1, ?2, ?3, ?4)",
            [
                &req.user.to_string(),
                &req.guild_id.to_string(),
                &req.channel_name.to_string(),
                &req.intro_id.to_string(),
            ],
        )
        .context("failed to insert user intro")?;

        Ok(())
    }

    async fn create_guild(&self, req: CreateGuildRequest) -> Result<Guild, CreateGuildError> {
        let conn = self.conn.lock().await;

        let guild_id: GuildId = req.external_id.0.into();

        conn.execute(
            "
            INSERT INTO
                Guild (id, name, sound_delay)
            VALUES (?1, ?2, ?3)
            ",
            [
                &guild_id.to_string(),
                &req.name,
                &req.sound_delay.to_string(),
            ],
        )
        .context("failed to insert guild")?;

        Ok(Guild::new(
            guild_id,
            req.name,
            req.sound_delay,
            req.external_id,
        ))
    }

    async fn create_user(&self, req: CreateUserRequest) -> Result<(), CreateUserError> {
        let conn = self.conn.lock().await;

        conn.execute(
            "
            INSERT INTO
                User (username)
            VALUES (?1)
            ",
            [req.user.as_ref()],
        )
        .context("failed to insert user")?;

        Ok(())
    }

    async fn add_user_to_guild(
        &self,
        guild_id: GuildId,
        username: &str,
    ) -> Result<(), guild::AddUserToGuildError> {
        let conn = self.conn.lock().await;

        conn.execute(
            "
            INSERT OR IGNORE INTO UserGuild (username, guild_id) VALUES (?1, ?2)
            ",
            [username, &guild_id.to_string()],
        )
        .context("failed to insert user guild")?;

        Ok(())
    }

    async fn create_channel(
        &self,
        req: CreateChannelRequest,
    ) -> Result<Channel, CreateChannelError> {
        todo!()
    }

    async fn add_intro_to_guild(
        &self,
        name: &str,
        guild_id: GuildId,
        filename: String,
    ) -> Result<IntroId, AddIntroToGuildError> {
        let conn = self.conn.lock().await;

        let mut query = conn
            .prepare(
                "
                INSERT INTO Intro
                (
                    name,
                    volume,
                    guild_id,
                    filename
                )
                VALUES
                (
                    :name,
                    :volume,
                    :guild_id,
                    :filename
                )
                RETURNING id
                ",
            )
            .context("failed to prepare query")?;

        let intro_id = query
            .query_row(
                &[
                    (":name", name),
                    (":volume", &0.to_string()),
                    (":guild_id", &guild_id.to_string()),
                    (":filename", &filename),
                ],
                |row| Ok(row.get::<_, i32>(0)?.into()),
            )
            .context("failed to query row")?;

        Ok(intro_id)
    }
}
