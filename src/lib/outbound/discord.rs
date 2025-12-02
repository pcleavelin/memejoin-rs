use std::collections::HashMap;

use chrono::{Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::domain::intro_tool::{
    models::guild::{ExternalChannel, ExternalGuild, ExternalGuildId, UserName},
    ports::{AuthService, ExternalUser},
};

#[derive(Clone)]
pub struct DiscordService;

#[derive(Clone)]
pub struct DiscordUser {
    token: String,
    expires_at: NaiveDateTime,
    username: String,
    guilds: Vec<u64>,
}

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error(transparent)]
    ApiRequest(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
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

#[derive(Serialize)]
pub struct DiscordGuildChannelsRequest {
    pub guild_id: u64,
    pub bot_token: String,
}

#[derive(Deserialize)]
pub struct DiscordChannel {
    #[serde(rename = "type")]
    pub ty: u32,

    pub name: Option<String>,
}

#[derive(PartialEq, Eq)]
#[repr(u32)]
enum ChannelType {
    GuildText = 0,
    GuildVoice = 2,
}

impl AuthService for DiscordService {
    type Params = DiscordAuthParams;
    type User = DiscordUser;
    type Error = DiscordError;

    type Channel = DiscordChannel;

    type ListGuildsRequest = String;
    type ListGuildChannelsRequest = DiscordGuildChannelsRequest;

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
            expires_at: Utc::now().naive_utc() + Duration::seconds(auth.expires_in as _),
            username: user.username,
            guilds: discord_guilds.into_iter().map(|guild| guild.id).collect(),
        })
    }

    async fn get_guilds(req: Self::ListGuildsRequest) -> Result<Vec<ExternalGuild>, Self::Error> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://discord.com/api/v10/users/@me/guilds")
            .bearer_auth(&req)
            .send()
            .await?;

        let response_text = response.text().await?;
        tracing::debug!(?response_text);

        let discord_guilds: Vec<DiscordUserGuild> = serde_json::from_str(&response_text)?;

        Ok(discord_guilds
            .into_iter()
            .map(|guild| ExternalGuild {
                id: ExternalGuildId(guild.id),
                name: guild.name,
            })
            .collect())
    }

    async fn get_guild_channels(
        req: Self::ListGuildChannelsRequest,
    ) -> Result<Vec<ExternalChannel>, Self::Error> {
        let client = reqwest::Client::new();

        Ok(client
            .get(format!(
                "https://discord.com/api/v10/guilds/{}/channels",
                req.guild_id
            ))
            .header("Authorization", format!("Bot {}", req.bot_token))
            .send()
            .await?
            .json::<Vec<Self::Channel>>()
            .await?
            .into_iter()
            .filter(|channel| channel.ty == ChannelType::GuildVoice as u32)
            .filter_map(|channel| channel.name)
            .map(|name| ExternalChannel { name })
            .collect())
    }
}

impl ExternalUser for DiscordUser {
    fn external_token(&self) -> &str {
        &self.token
    }

    fn external_token_expires_at(&self) -> NaiveDateTime {
        self.expires_at
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
