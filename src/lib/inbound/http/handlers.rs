use std::{collections::HashMap, str::FromStr};

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::{Html, Redirect},
};
use serde::Deserialize;

use crate::{
    auth::{AppPermission, Permission, Permissions},
    domain::intro_tool::{
        models::guild::{
            AddIntroToGuildRequest, AddIntroToUserRequest, ChannelName, CreateGuildRequest,
            GuildId, IntroRequestData, UpdateUserGuildPermissionsRequest, User, UserName,
        },
        ports::IntroToolService,
    },
    htmx::Build,
    inbound::{
        http::{ApiState, page},
        response::{ApiError, ErrorAsRedirect},
    },
    outbound::discord::DiscordService,
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

impl FromApi<Multipart, (GuildId, UserName, ChannelName)> for AddIntroToUserRequest {
    async fn from_api(
        mut value: Multipart,
        (guild_id, user, channel_name): (GuildId, UserName, ChannelName),
    ) -> Result<Self, ApiError> {
        let intro_id = value
            .next_field()
            .await
            .map_err(|err| ApiError::bad_request(format!("expected intro id: {err:?}")))?
            .ok_or(ApiError::bad_request("intro id is required"))?
            .name()
            .ok_or(ApiError::bad_request("intro id is required"))?
            .parse::<i32>()
            .map_err(|err| ApiError::bad_request(format!("invalid intro id: {err:?}")))?
            .into();

        Ok(Self {
            user,
            guild_id,
            channel_name,
            intro_id,
        })
    }
}

impl FromApi<Multipart, GuildId> for UpdateUserGuildPermissionsRequest {
    async fn from_api(mut value: Multipart, guild_id: GuildId) -> Result<Self, ApiError> {
        let mut permissions = HashMap::<_, Permissions>::new();

        while let Ok(Some(field)) = value.next_field().await {
            let Some(field_name) = field.name() else {
                continue;
            };

            if let Some((username, permission)) = field_name.split_once('#') {
                let permission = Permission::from_str(permission)?;

                let username = username.to_string();
                if field.text().await.map_err(ApiError::bad_request)? == "on" {
                    permissions
                        .entry(username.into())
                        .and_modify(|value| {
                            value.add(permission);
                        })
                        .or_insert_with(|| {
                            let mut perm = Permissions::default();
                            perm.add(permission);
                            perm
                        });
                }
            }
        }

        Ok(Self {
            guild_id,
            permissions,
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

pub(super) async fn set_user_intro<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Path((guild_id, channel)): Path<(u64, String)>,
    user: User,
    form_data: Multipart,
) -> Result<Html<String>, ApiError> {
    let req = form_data
        .into_domain((
            guild_id.into(),
            user.name().to_string().into(),
            channel.clone().into(),
        ))
        .await?;

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

    // TODO: check if channel exists

    state.intro_tool_service.set_user_intro(req).await?;
    let user = state.intro_tool_service.get_user(user.name()).await?;

    let guild_intros = state
        .intro_tool_service
        .get_guild_intros(guild_id.into())
        .await?;
    let intros = user
        .intros()
        .get(&(guild.id(), channel.clone().into()))
        .map(|intros| intros.iter())
        .unwrap_or_default();

    Ok(Html(
        page::channel_intro_selector(
            &state.origin,
            guild_id,
            &channel.into(),
            intros,
            guild_intros.iter(),
        )
        .build(),
    ))
}

#[derive(Debug, Deserialize)]
pub(super) struct GuildSetupParams {
    name: String,
}

pub(super) async fn guild_setup<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Path(guild_id): Path<u64>,
    user: User,
    Query(GuildSetupParams { name }): Query<GuildSetupParams>,
) -> Result<Redirect, ApiError> {
    // FIXME: move this into the service impl

    if !user.permissions().can(AppPermission::AddGuild) {
        return Err(ApiError::forbidden("invalid permissions"));
    }

    let Some(external_guild) = state
        .intro_tool_service
        .get_user_external_guilds::<DiscordService>(user.external_token().to_string())
        .await?
        .into_iter()
        .find(|external| external.id.0 == guild_id)
    else {
        return Err(ApiError::forbidden("invalid guild"));
    };

    let new_guild = state
        .intro_tool_service
        .create_guild(CreateGuildRequest {
            name,
            sound_delay: 0,
            external_id: external_guild.id,
        })
        .await
        .map_err(ApiError::internal)?;

    state
        .intro_tool_service
        .add_user_to_guild(guild_id.into(), user.name())
        .await
        .map_err(ApiError::internal)?;

    Ok(Redirect::to(&format!(
        "{}/guild/{}",
        state.origin,
        new_guild.id()
    )))
}

pub(super) async fn update_guild_permissions<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Path(guild_id): Path<u64>,
    user: User,
    form_data: Multipart,
) -> Result<HeaderMap, ApiError> {
    if !user.permissions().can(AppPermission::Admin) {
        return Err(ApiError::forbidden("invalid permissions"));
    }

    let req: UpdateUserGuildPermissionsRequest = form_data.into_domain(guild_id.into()).await?;

    let guild_users = state
        .intro_tool_service
        .get_guild_users(guild_id.into())
        .await?
        .iter()
        .filter(|(_, perms)| !perms.can(Permission::Moderator))
        .map(|(user, _)| user)
        .map(|user| {
            if let Some(new_perms) = req.permissions.get(user.name()) {
                (user.name().to_string(), *new_perms)
            } else {
                (user.name().to_string(), Default::default())
            }
        })
        .collect::<HashMap<_, _>>();

    for (username, permissions) in guild_users {
        state
            .intro_tool_service
            .set_user_guild_permissions(username.as_str(), guild_id.into(), permissions)
            .await?;
    }

    let mut headers = HeaderMap::new();
    headers.insert("HX-Refresh", HeaderValue::from_static("true"));

    Ok(headers)
}
