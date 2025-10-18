use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use thiserror::Error;

use crate::domain::intro_tool::{
    models::guild::{ExternalGuildId, UserName},
    ports::{AuthService, ExternalUser},
};

#[derive(Clone)]
pub struct DiscordService;

#[derive(Clone)]
pub struct DiscordUser {
    token: String,
    username: String,
    guilds: Vec<u64>,
}

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error(transparent)]
    ApiRequest(#[from] reqwest::Error),
}

pub struct DiscordAuthParams {
    pub origin: String,
    pub code: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize)]
struct DiscordApiAuth {
    access_token: String,
    token_type: String,
    expires_in: usize,
    refresh_token: String,
    scope: String,
}

#[derive(Deserialize)]
struct DiscordApiUser {
    pub username: String,
}

#[derive(Deserialize)]
struct DiscordUserGuild {
    #[serde(deserialize_with = "serde_string_as_u64")]
    id: u64,
    name: String,
    owner: bool,
}

impl AuthService for DiscordService {
    type Params = DiscordAuthParams;
    type User = DiscordUser;
    type Error = DiscordError;

    async fn authenticate_user(params: Self::Params) -> Result<Self::User, Self::Error> {
        let mut data = HashMap::new();

        let redirect_uri = format!("{}/v2/auth", params.origin);
        data.insert("client_id", params.client_id.as_str());
        data.insert("client_secret", params.client_secret.as_str());
        data.insert("grant_type", "authorization_code");
        data.insert("code", &params.code);
        data.insert("redirect_uri", &redirect_uri);

        let client = reqwest::Client::new();

        let auth: DiscordApiAuth = client
            .post("https://discord.com/api/oauth2/token")
            .form(&data)
            .send()
            .await?
            .json()
            .await?;

        // Get authorized username
        let user: DiscordApiUser = client
            .get("https://discord.com/api/v10/users/@me")
            .bearer_auth(&auth.access_token)
            .send()
            .await?
            .json()
            .await?;

        // TODO: get bot's guilds so we only save users who are able to use the bot
        let discord_guilds: Vec<DiscordUserGuild> = client
            .get("https://discord.com/api/v10/users/@me/guilds")
            .bearer_auth(&auth.access_token)
            .send()
            .await?
            .json()
            .await?;

        Ok(Self::User {
            token: auth.access_token,
            username: user.username,
            guilds: discord_guilds.into_iter().map(|guild| guild.id).collect(),
        })
    }
}

impl ExternalUser for DiscordUser {
    fn external_token(&self) -> &str {
        &self.token
    }

    fn username(&self) -> UserName {
        self.username.clone().into()
    }

    fn guilds(&self) -> impl Iterator<Item = ExternalGuildId> {
        self.guilds.iter().map(|id| ExternalGuildId(*id))
    }
}

fn serde_string_as_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = <&str as Deserialize>::deserialize(deserializer)?;

    value
        .parse::<u64>()
        .map_err(|_| serde::de::Error::invalid_value(serde::de::Unexpected::Str(value), &"u64"))
}
