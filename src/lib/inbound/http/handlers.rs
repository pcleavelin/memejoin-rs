use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
};

use crate::lib::{
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
    fn from_api(value: T, params: P) -> Result<Self, ApiError>;
}
trait IntoDomain<T, P> {
    fn into_domain(self, params: P) -> Result<T, ApiError>;
}

impl<I, O: FromApi<I, P>, P> IntoDomain<O, P> for I {
    fn into_domain(self, params: P) -> Result<O, ApiError> {
        O::from_api(self, params)
    }
}

impl FromApi<HashMap<String, String>, GuildId> for AddIntroToGuildRequest {
    fn from_api(value: HashMap<String, String>, params: GuildId) -> Result<Self, ApiError> {
        let Some(url) = value.get("url") else {
            return Err(ApiError::bad_request("url is required"));
        };

        let Some(name) = value.get("name") else {
            return Err(ApiError::bad_request("name is required"));
        };

        Ok(Self {
            guild_id: params,
            name: name.to_string(),
            volume: 0,
            data: IntroRequestData::Url(url.to_string()),
        })
    }
}

pub(super) async fn add_guild_intro<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Path(guild_id): Path<u64>,
    Query(params): Query<HashMap<String, String>>,
    user: User,
) -> Result<HeaderMap, ApiError> {
    let req = params.into_domain(guild_id.into())?;

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
