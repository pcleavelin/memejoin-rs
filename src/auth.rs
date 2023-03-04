use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use tracing::trace;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Discord {
    pub(crate) access_token: String,
    pub(crate) token_type: String,
    pub(crate) expires_in: usize,
    pub(crate) refresh_token: String,
    pub(crate) scope: String,
}

#[derive(Clone)]
pub(crate) struct DiscordSecret {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) auth: Discord,
    #[serde(default)]
    pub(crate) permissions: Permissions,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Permissions(pub(crate) u8);
impl Default for Permissions {
    fn default() -> Permissions {
        Permissions(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum Permission {
    None,
    DownloadSounds,
}

impl Permissions {
    pub(crate) fn can(&self, perm: Permission) -> bool {
        self.0 & (perm as u8) > 0
    }
}
