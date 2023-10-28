use std::str::FromStr;

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

use crate::routes::Error;

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
    pub(crate) name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Permissions(pub(crate) u8);
impl Default for Permissions {
    fn default() -> Permissions {
        Permissions(0)
    }
}

impl Permissions {
    pub(crate) fn can(&self, perm: Permission) -> bool {
        (self.0 & (perm as u8) > 0) || (self.0 & (Permission::Moderator as u8) > 0)
    }

    pub(crate) fn add(&mut self, perm: Permission) {
        self.0 |= perm as u8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Sequence)]
#[repr(u8)]
pub(crate) enum Permission {
    None = 0,
    UploadSounds = 1,
    DeleteSounds = 2,
    Soundboard = 4,
    Moderator = 128,
}

impl Permission {
    pub(crate) fn all() -> u8 {
        0xFF
    }
}

impl ToString for Permission {
    fn to_string(&self) -> String {
        match self {
            Permission::None => todo!(),
            Permission::UploadSounds => "Upload Sounds".to_string(),
            Permission::DeleteSounds => "Delete Sounds".to_string(),
            Permission::Soundboard => "Soundboard".to_string(),
            Permission::Moderator => "Moderator".to_string(),
        }
    }
}

impl FromStr for Permission {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Upload Sounds" => Ok(Self::UploadSounds),
            "Delete Sounds" => Ok(Self::DeleteSounds),
            "Soundboard" => Ok(Self::Soundboard),
            "Moderator" => Ok(Self::Moderator),
            _ => Err(Self::Err::InvalidRequest),
        }
    }
}
