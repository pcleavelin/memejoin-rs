#[derive(Clone)]
pub struct DiscordSecret {
    pub client_id: String,
    pub client_secret: String,
    pub bot_token: String,
}

/*
use std::str::FromStr;

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Discord {
    pub(crate) access_token: String,
    pub(crate) token_type: String,
    pub(crate) expires_in: usize,
    pub(crate) refresh_token: String,
    pub(crate) scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) auth: Discord,
    pub(crate) name: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppPermissions(pub(crate) u8);

impl AppPermissions {
    pub(crate) fn can(&self, perm: AppPermission) -> bool {
        (self.0 & (perm as u8) > 0) || (self.0 & (AppPermission::Admin as u8) > 0)
    }

    // FIXME: eventually use this
    #[allow(dead_code)]
    pub(crate) fn add(&mut self, perm: Permission) {
        self.0 |= perm as u8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Sequence)]
#[repr(u8)]
pub(crate) enum AppPermission {
    None = 0,
    AddGuild = 1,
    Admin = 128,
}

impl AppPermission {
    pub(crate) fn all() -> u8 {
        0xFF
    }
}

impl std::fmt::Display for AppPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AppPermission::None => todo!(),
                AppPermission::AddGuild => "Add Guild".to_string(),
                AppPermission::Admin => "Admin".to_string(),
            }
        )
    }
}

impl FromStr for AppPermission {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Add Guild" => Ok(Self::AddGuild),
            "Admin" => Ok(Self::Admin),
            _ => Err(Self::Err::InvalidRequest),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct Permissions(pub(crate) u8);

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
    AddChannel = 8,
    Moderator = 128,
}

impl Permission {
    pub(crate) fn all() -> u8 {
        0xFF
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Permission::None => todo!(),
                Permission::UploadSounds => "Upload Sounds".to_string(),
                Permission::DeleteSounds => "Delete Sounds".to_string(),
                Permission::Soundboard => "Soundboard".to_string(),
                Permission::AddChannel => "Add Channel".to_string(),
                Permission::Moderator => "Moderator".to_string(),
            },
        )
    }
}

impl FromStr for Permission {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Upload Sounds" => Ok(Self::UploadSounds),
            "Delete Sounds" => Ok(Self::DeleteSounds),
            "Soundboard" => Ok(Self::Soundboard),
            "Add Channel" => Ok(Self::AddChannel),
            "Moderator" => Ok(Self::Moderator),
            _ => Err(Self::Err::InvalidRequest),
        }
    }
}
*/
