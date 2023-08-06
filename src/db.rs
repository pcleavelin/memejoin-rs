use std::path::Path;

use iter_tools::Itertools;
use rusqlite::{Connection, Result};
use tracing::{error, warn};

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

    pub fn get_user_guilds(&self, username: &str) -> Result<Vec<Guild>> {
        let mut query = self.conn.prepare(
            "
            SELECT
                id, name, soundDelay
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
                Intro.name
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
                        },
                        channel_name: row.get(2)?,
                        username: row.get(3)?,
                    })
                },
            )?
            .into_iter()
            .collect::<Result<Vec<UserIntro>>>();

        intros
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
            ",
            [username],
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

    pub fn remove_user_intro(
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
    pub id: String,
    pub name: String,
    pub sound_delay: u32,
}

pub struct Intro {
    pub id: i32,
    pub name: String,
}

pub struct UserIntro {
    pub intro: Intro,
    pub channel_name: String,
    pub username: String,
}
