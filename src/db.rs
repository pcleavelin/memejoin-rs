use std::path::Path;

use rusqlite::{Connection, Result};

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
}

pub struct Guild {
    pub id: String,
    pub name: String,
    pub sound_delay: u32,
}
