use std::path::Path;

use chrono::NaiveDateTime;
use rusqlite::{Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::auth;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            conn: Connection::open(path)?,
        })
    }

    pub(crate) fn get_guild_users(&self, guild_id: u64) -> Result<Vec<String>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                username
            FROM UserGuild
            WHERE guild_id = :guild_id
            ",
        )?;

        // NOTE(pcleavelin): for some reason this needs to be a let-binding or else
        // the compiler complains about it being dropped too early (maybe I should update the compiler version)
        let users = query
            .query_map(&[(":guild_id", &guild_id.to_string())], |row| row.get(0))?
            .collect::<Result<Vec<String>>>()?;

        Ok(users)
    }

    pub(crate) fn get_guilds(&self) -> Result<Vec<Guild>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                id, name, sound_delay
            FROM Guild
            ",
        )?;

        // NOTE(pcleavelin): for some reason this needs to be a let-binding or else
        // the compiler complains about it being dropped too early (maybe I should update the compiler version)
        let guilds = query
            .query_map([], |row| {
                Ok(Guild {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sound_delay: row.get(2)?,
                })
            })?
            .into_iter()
            .collect::<Result<Vec<Guild>>>();

        guilds
    }

    pub(crate) fn get_user_from_api_key(&self, api_key: &str) -> Result<User> {
        self.conn.query_row(
            "
            SELECT
                username AS name, api_key, api_key_expires_at, discord_token, discord_token_expires_at
            FROM User
            WHERE api_key = ?1
            ",
            [api_key],
            |row| {
                Ok(User {
                    name: row.get(0)?,
                    api_key: row.get(1)?,
                    api_key_expires_at: row.get(2)?,
                    discord_token: row.get(3)?,
                    discord_token_expires_at: row.get(4)?,
                })
            },
        )
    }

    pub(crate) fn get_user(&self, username: &str) -> Result<Option<User>> {
        self.conn
            .query_row(
                "
            SELECT
                username AS name, api_key, api_key_expires_at, discord_token, discord_token_expires_at
            FROM User
            WHERE name = ?1
            ",
                [username],
                |row| {
                    Ok(User {
                        name: row.get(0)?,
                        api_key: row.get(1)?,
                        api_key_expires_at: row.get(2)?,
                        discord_token: row.get(3)?,
                        discord_token_expires_at: row.get(4)?,
                    })
                },
            )
            .optional()
    }

    pub fn get_user_guilds(&self, username: &str) -> Result<Vec<Guild>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                id, name, sound_delay
            FROM Guild
            LEFT JOIN UserGuild ON UserGuild.guild_id = Guild.id
            WHERE UserGuild.username = :username
            ",
        )?;

        // NOTE(pcleavelin): for some reason this needs to be a let-binding or else
        // the compiler complains about it being dropped too early (maybe I should update the compiler version)
        let guilds = query
            .query_map(&[(":username", username)], |row| {
                Ok(Guild {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sound_delay: row.get(2)?,
                })
            })?
            .into_iter()
            .collect::<Result<Vec<Guild>>>();

        guilds
    }

    pub fn get_guild_intros(&self, guild_id: u64) -> Result<Vec<Intro>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                Intro.id,
                Intro.name,
                Intro.filename
            FROM Intro
            WHERE
                Intro.guild_id = :guild_id
            ",
        )?;

        // NOTE(pcleavelin): for some reason this needs to be a let-binding or else
        // the compiler complains about it being dropped too early (maybe I should update the compiler version)
        let intros = query
            .query_map(
                &[
                    // :vomit:
                    (":guild_id", &guild_id.to_string()),
                ],
                |row| {
                    Ok(Intro {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        filename: row.get(2)?,
                    })
                },
            )?
            .into_iter()
            .collect::<Result<Vec<Intro>>>();

        intros
    }

    pub fn get_all_user_intros(&self, guild_id: u64) -> Result<Vec<UserIntro>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                Intro.id,
                Intro.name,
                Intro.filename,
                UI.channel_name,
                UI.username
            FROM Intro
            LEFT JOIN UserIntro UI ON UI.intro_id = Intro.id
            WHERE
                UI.guild_id = :guild_id
            ORDER BY UI.username DESC, UI.channel_name DESC, UI.intro_id;
            ",
        )?;

        // NOTE(pcleavelin): for some reason this needs to be a let-binding or else
        // the compiler complains about it being dropped too early (maybe I should update the compiler version)
        let intros = query
            .query_map(
                &[
                    // :vomit:
                    (":guild_id", &guild_id.to_string()),
                ],
                |row| {
                    Ok(UserIntro {
                        intro: Intro {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            filename: row.get(2)?,
                        },
                        channel_name: row.get(3)?,
                        username: row.get(4)?,
                    })
                },
            )?
            .into_iter()
            .collect::<Result<Vec<UserIntro>>>();

        intros
    }

    pub(crate) fn get_all_user_permissions(
        &self,
        guild_id: u64,
    ) -> Result<Vec<(String, auth::Permissions)>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                username,
                permissions
            FROM UserPermission
            WHERE
                guild_id = :guild_id
            ",
        )?;

        let permissions = query
            .query_map(
                &[
                    // :vomit:
                    (":guild_id", &guild_id.to_string()),
                ],
                |row| Ok((row.get(0)?, auth::Permissions(row.get(1)?))),
            )?
            .collect::<Result<Vec<(String, auth::Permissions)>>>()?;

        Ok(permissions)
    }

    pub(crate) fn get_user_permissions(
        &self,
        username: &str,
        guild_id: u64,
    ) -> Result<auth::Permissions> {
        self.conn.query_row(
            "
            SELECT
                permissions
            FROM UserPermission
            WHERE
                username = ?1
            AND guild_id = ?2
            ",
            [username, &guild_id.to_string()],
            |row| Ok(auth::Permissions(row.get(0)?)),
        )
    }

    pub(crate) fn get_guild_channels(&self, guild_id: u64) -> Result<Vec<String>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                Channel.name
            FROM Channel
            WHERE
                Channel.guild_id = :guild_id
            ORDER BY Channel.name DESC
            ",
        )?;

        // NOTE(pcleavelin): for some reason this needs to be a let-binding or else
        // the compiler complains about it being dropped too early (maybe I should update the compiler version)
        let intros = query
            .query_map(
                &[
                    // :vomit:
                    (":guild_id", &guild_id.to_string()),
                ],
                |row| Ok(row.get(0)?),
            )?
            .into_iter()
            .collect::<Result<Vec<String>>>();

        intros
    }

    pub(crate) fn get_user_channel_intros(
        &self,
        username: &str,
        guild_id: u64,
        channel_name: &str,
    ) -> Result<Vec<Intro>> {
        let all_user_intros = self.get_all_user_intros(guild_id)?.into_iter();

        let intros = all_user_intros
            .filter(|intro| &intro.username == &username && &intro.channel_name == channel_name)
            .map(|intro| intro.intro)
            .collect();

        Ok(intros)
    }

    pub fn insert_user(
        &self,
        username: &str,
        api_key: &str,
        api_key_expires_at: NaiveDateTime,
        discord_token: &str,
        discord_token_expires_at: NaiveDateTime,
    ) -> Result<()> {
        let affected = self.conn.execute(
            "INSERT INTO
                User (username, api_key, api_key_expires_at, discord_token, discord_token_expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(username) DO UPDATE SET api_key = ?2, api_key_expires_at = ?3, discord_token = ?4, discord_token_expires_at = ?5",
            &[
                username,
                api_key,
                &api_key_expires_at.to_string(),
                discord_token,
                &discord_token_expires_at.to_string(),
            ],
        )?;

        if affected < 1 {
            warn!("no rows affected when attempting to insert new user");
        }

        Ok(())
    }

    pub fn insert_intro(
        &self,
        name: &str,
        volume: i32,
        guild_id: u64,
        filename: &str,
    ) -> Result<()> {
        let affected = self.conn.execute(
            "INSERT INTO
                Intro (name, volume, guild_id, filename)
            VALUES (?1, ?2, ?3, ?4)",
            &[name, &volume.to_string(), &guild_id.to_string(), filename],
        )?;

        if affected < 1 {
            warn!("no rows affected when attempting to insert intro");
        }

        Ok(())
    }

    pub fn insert_user_guild(&self, username: &str, guild_id: u64) -> Result<()> {
        let affected = self.conn.execute(
            "INSERT OR IGNORE INTO UserGuild (username, guild_id) VALUES (?1, ?2)",
            &[username, &guild_id.to_string()],
        )?;

        if affected < 1 {
            warn!("no rows affected when attempting to insert user guild");
        }

        Ok(())
    }

    pub fn insert_user_intro(
        &self,
        username: &str,
        guild_id: u64,
        channel_name: &str,
        intro_id: i32,
    ) -> Result<()> {
        let affected = self.conn.execute(
            "INSERT INTO UserIntro (username, guild_id, channel_name, intro_id) VALUES (?1, ?2, ?3, ?4)",
            &[
                username,
                &guild_id.to_string(),
                channel_name,
                &intro_id.to_string(),
            ],
        )?;

        if affected < 1 {
            warn!("no rows affected when attempting to insert user intro");
        }

        Ok(())
    }

    pub(crate) fn insert_user_permission(
        &self,
        username: &str,
        guild_id: u64,
        permissions: auth::Permissions,
    ) -> Result<()> {
        let affected = self.conn.execute(
            "
            INSERT INTO
                UserPermission (username, guild_id, permissions)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(username, guild_id) DO UPDATE SET permissions = ?3",
            &[username, &guild_id.to_string(), &permissions.0.to_string()],
        )?;

        if affected < 1 {
            warn!("no rows affected when attempting to insert user permissions");
        }

        Ok(())
    }

    pub fn delete_user_intro(
        &self,
        username: &str,
        guild_id: u64,
        channel_name: &str,
        intro_id: i32,
    ) -> Result<()> {
        let affected = self.conn.execute(
            "DELETE FROM
                UserIntro
            WHERE 
                username = ?1 
            AND guild_id = ?2 
            AND channel_name = ?3 
            AND intro_id = ?4",
            &[
                username,
                &guild_id.to_string(),
                channel_name,
                &intro_id.to_string(),
            ],
        )?;

        if affected < 1 {
            warn!("no rows affected when attempting to delete user intro");
        }

        Ok(())
    }
}

pub struct Guild {
    pub id: u64,
    pub name: String,
    pub sound_delay: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub api_key: String,
    pub api_key_expires_at: NaiveDateTime,
    pub discord_token: String,
    pub discord_token_expires_at: NaiveDateTime,
}

pub struct Intro {
    pub id: i32,
    pub name: String,
    pub filename: String,
}

pub struct UserIntro {
    pub intro: Intro,
    pub channel_name: String,
    pub username: String,
}
