use std::collections::HashMap;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue},
};

use crate::{
    domain::intro_tool::{
        models::guild::{AddIntroToGuildRequest, GuildId, IntroRequestData, User},
        ports::IntroToolService,
    },
    inbound::{
        http::ApiState,
        response::{ApiError, ErrorAsRedirect},
    },
};

trait FromApi<T, P>: Sized {
    async fn from_api(value: T, params: P) -> Result<Self, ApiError>;
}
trait IntoDomain<T, P> {
    async fn into_domain(self, params: P) -> Result<T, ApiError>;
}

impl<I, O: FromApi<I, P>, P> IntoDomain<O, P> for I {
    async fn into_domain(self, params: P) -> Result<O, ApiError> {
        O::from_api(self, params).await
    }
}

impl FromApi<HashMap<String, String>, GuildId> for AddIntroToGuildRequest {
    async fn from_api(value: HashMap<String, String>, params: GuildId) -> Result<Self, ApiError> {
        let Some(url) = value.get("url") else {
            return Err(ApiError::bad_request("url is required"));
        };
        if url.is_empty() {
            return Err(ApiError::bad_request("url cannot be empty"));
        }

        let Some(name) = value.get("name") else {
            return Err(ApiError::bad_request("name is required"));
        };
        if name.is_empty() {
            return Err(ApiError::bad_request("name cannot be empty"));
        }

        Ok(Self {
            guild_id: params,
            name: name.to_string(),
            volume: 0,
            data: IntroRequestData::Url(url.to_string()),
        })
    }
}

impl FromApi<Multipart, GuildId> for AddIntroToGuildRequest {
    async fn from_api(mut form_data: Multipart, params: GuildId) -> Result<Self, ApiError> {
        let mut name = None;
        let mut file = None;

        while let Ok(Some(field)) = form_data.next_field().await {
            let Some(field_name) = field.name() else {
                continue;
            };

            if field_name.eq_ignore_ascii_case("name") {
                name = Some(field.text().await.map_err(|err| {
                    ApiError::bad_request(format!("expected text for name: {err:?}"))
                })?);
                continue;
            }

            if field_name.eq_ignore_ascii_case("file") {
                file = Some(field.bytes().await.map_err(|err| {
                    ApiError::bad_request(format!("expected bytes for file: {err:?}"))
                })?);
                continue;
            }
        }

        let Some(name) = name else {
            return Err(ApiError::bad_request("name is required"));
        };
        if name.is_empty() {
            return Err(ApiError::bad_request("name cannot be empty"));
        }

        let Some(file) = file else {
            return Err(ApiError::bad_request("file is required"));
        };
        if file.is_empty() {
            return Err(ApiError::bad_request("file cannot be empty"));
        }

        Ok(Self {
            guild_id: params,
            name: name.to_string(),
            volume: 0,
            data: IntroRequestData::Data(file.to_vec()),
        })
    }
}

pub(super) async fn add_guild_intro<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Path(guild_id): Path<u64>,
    Query(params): Query<HashMap<String, String>>,
    user: User,
) -> Result<HeaderMap, ApiError> {
    let req = params.into_domain(guild_id.into()).await?;

    let guild = state.intro_tool_service.get_guild(guild_id).await?;
    let user_guilds = state
        .intro_tool_service
        .get_user_guilds(user.name())
        .await?;

    // does user have access to this guild
    if !user_guilds
        .iter()
        .any(|guild_ref| guild_ref.id() == guild.id())
    {
        return Err(ApiError::forbidden(
            "You do not have access to this guild".to_string(),
        ));
    }

    state.intro_tool_service.add_intro_to_guild(req).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    Ok(headers)
}

pub(super) async fn upload_guild_intro<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Path(guild_id): Path<u64>,
    user: User,
    form_data: Multipart,
) -> Result<HeaderMap, ApiError> {
    let req = form_data.into_domain(guild_id.into()).await?;

    let guild = state.intro_tool_service.get_guild(guild_id).await?;
    let user_guilds = state
        .intro_tool_service
        .get_user_guilds(user.name())
        .await?;

    // does user have access to this guild
    if !user_guilds
        .iter()
        .any(|guild_ref| guild_ref.id() == guild.id())
    {
        return Err(ApiError::forbidden(
            "You do not have access to this guild".to_string(),
        ));
    }

    state.intro_tool_service.add_intro_to_guild(req).await?;

    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    Ok(headers)
}
