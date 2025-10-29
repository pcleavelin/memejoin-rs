use std::fmt::Debug;

use axum::{
    Json,
    response::{Html, IntoResponse, Redirect},
};
use reqwest::StatusCode;
use serde::Serialize;

use crate::{
    domain::intro_tool::{
        models::guild::{
            AddIntroToGuildError, AddIntroToUserError, AddUserToGuildError, AutheticateUserError,
            CreateUserError, GetChannelError, GetGuildError, GetIntroError, GetUserError,
        },
        ports::AuthService,
    },
    htmx::{Build, HtmxBuilder, Tag},
    inbound::http::page::page_header,
    outbound::discord::DiscordError,
};

pub(super) trait ErrorAsRedirect<T>: Sized {
    fn as_redirect(self, origin: impl AsRef<str>, path: impl AsRef<str>) -> Result<T, Redirect>;
}

impl<T: Debug> ErrorAsRedirect<T> for Result<T, GetGuildError> {
    fn as_redirect(self, origin: impl AsRef<str>, path: impl AsRef<str>) -> Result<T, Redirect> {
        match self {
            Ok(value) => Ok(value),
            Err(GetGuildError::NotFound)
            | Err(GetGuildError::CouldNotFetchUsers(_))
            | Err(GetGuildError::CouldNotFetchChannels(_))
            | Err(GetGuildError::Unknown(_)) => {
                tracing::error!(err = ?self, "failed to get guild");

                Err(Redirect::to(&format!(
                    "{}/{}",
                    origin.as_ref(),
                    path.as_ref()
                )))
            }
        }
    }
}

impl<T: Debug> ErrorAsRedirect<T> for Result<T, GetChannelError> {
    fn as_redirect(self, origin: impl AsRef<str>, path: impl AsRef<str>) -> Result<T, Redirect> {
        match self {
            Ok(value) => Ok(value),
            Err(GetChannelError::NotFound) | Err(GetChannelError::Unknown(_)) => {
                tracing::error!(err = ?self, "failed to get channel");

                Err(Redirect::to(&format!(
                    "{}/{}",
                    origin.as_ref(),
                    path.as_ref()
                )))
            }
        }
    }
}

impl<T: Debug> ErrorAsRedirect<T> for Result<T, GetIntroError> {
    fn as_redirect(self, origin: impl AsRef<str>, path: impl AsRef<str>) -> Result<T, Redirect> {
        match self {
            Ok(value) => Ok(value),
            Err(GetIntroError::NotFound) | Err(GetIntroError::Unknown(_)) => {
                tracing::error!(err = ?self, "failed to get intro");

                Err(Redirect::to(&format!(
                    "{}/{}",
                    origin.as_ref(),
                    path.as_ref()
                )))
            }
        }
    }
}

pub(super) struct PageError(pub ApiError);

impl IntoResponse for PageError {
    fn into_response(self) -> axum::response::Response {
        Html(
            page_header("MemeJoin - Error")
                .builder(Tag::Div, |b| {
                    b.attribute("class", "container")
                        .builder_text(
                            Tag::Header2,
                            &format!("Uh oh! - Status Code {}", self.0.status_code()),
                        )
                        .builder(Tag::Blockquote, |b| b.text(self.0.message()))
                        .builder(Tag::Empty, |b| b.text("<br/>"))
                        .builder(Tag::Anchor, |b| {
                            b.attribute("role", "button")
                                .text("Go Back")
                                .attribute("href", "/")
                        })
                })
                .build(),
        )
        .into_response()
    }
}

pub(super) struct ApiResponse<T: Serialize>(StatusCode, Json<T>);

#[derive(Serialize, Debug)]
#[serde(tag = "status")]
pub(super) enum ApiError {
    NotFound {
        message: String,
    },
    BadRequest {
        message: String,
    },
    Forbidden {
        message: String,
    },
    InternalServerError {
        #[serde(skip)]
        message: String,
    },
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &str {
        match self {
            ApiError::NotFound { message } => message,
            ApiError::BadRequest { message } => message,
            ApiError::Forbidden { message } => message,
            ApiError::InternalServerError { message } => message,
        }
    }

    pub(super) fn not_found(message: impl ToString) -> Self {
        Self::NotFound {
            message: message.to_string(),
        }
    }

    pub(super) fn bad_request(message: impl ToString) -> Self {
        Self::BadRequest {
            message: message.to_string(),
        }
    }

    pub(super) fn forbidden(message: impl ToString) -> Self {
        Self::Forbidden {
            message: message.to_string(),
        }
    }

    pub(super) fn internal(message: impl ToString) -> Self {
        Self::InternalServerError {
            message: message.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(err = ?self, "error");

        (self.status_code(), Json(self)).into_response()
    }
}

impl From<ApiError> for PageError {
    fn from(value: ApiError) -> Self {
        Self(value)
    }
}

impl From<GetGuildError> for ApiError {
    fn from(value: GetGuildError) -> Self {
        match value {
            GetGuildError::NotFound => Self::not_found("Guild not found"),
            GetGuildError::CouldNotFetchUsers(get_user_error) => {
                tracing::error!(err = ?get_user_error, "could not fetch users from guild");

                Self::internal("Could not fetch users from guild".to_string())
            }
            GetGuildError::CouldNotFetchChannels(get_channel_error) => {
                tracing::error!(err = ?get_channel_error, "could not fetch channels from guild");

                Self::internal("Could not fetch channels from guild".to_string())
            }
            GetGuildError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl From<GetUserError> for ApiError {
    fn from(value: GetUserError) -> Self {
        match value {
            GetUserError::NotFound => Self::not_found("User not found"),
            GetUserError::CouldNotFetchGuilds(get_guild_error) => {
                tracing::error!(err = ?get_guild_error, "could not fetch guilds from user");

                Self::internal("Could not fetch guilds from user".to_string())
            }
            GetUserError::CouldNotFetchChannelIntros(get_channel_intro_error) => {
                tracing::error!(err = ?get_channel_intro_error, "could not fetch channel intros from user");

                Self::internal("Could not fetch channel intros from user".to_string())
            }
            GetUserError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl From<AddIntroToGuildError> for ApiError {
    fn from(value: AddIntroToGuildError) -> Self {
        match value {
            AddIntroToGuildError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl From<AddIntroToUserError> for ApiError {
    fn from(value: AddIntroToUserError) -> Self {
        match value {
            AddIntroToUserError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl From<GetIntroError> for ApiError {
    fn from(value: GetIntroError) -> Self {
        match value {
            GetIntroError::NotFound => Self::not_found("Intro not found"),
            GetIntroError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl From<CreateUserError> for ApiError {
    fn from(value: CreateUserError) -> Self {
        match value {
            CreateUserError::CouldNotGetUser(err) => err.into(),
            CreateUserError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl From<AddUserToGuildError> for ApiError {
    fn from(value: AddUserToGuildError) -> Self {
        match value {
            AddUserToGuildError::Unknown(error) => {
                tracing::error!(err = ?error, "unknown error");

                Self::internal(error.to_string())
            }
        }
    }
}

impl<A: AuthService> From<AutheticateUserError<A>> for ApiError
where
    <A as AuthService>::Error: Into<ApiError>,
{
    fn from(value: AutheticateUserError<A>) -> Self {
        match value {
            AutheticateUserError::CouldNotFetchGuild(err) => err.into(),
            AutheticateUserError::CouldNotCreateUser(err) => err.into(),
            AutheticateUserError::CouldNotFetchUser(err) => err.into(),
            AutheticateUserError::CouldNotAddUserToGuild(err) => err.into(),
            AutheticateUserError::UserNotPartOfInstanceGuilds => {
                Self::internal("User not part of instance guilds")
            }
            AutheticateUserError::ExternalError(err) => err.into(),
            AutheticateUserError::Unknown(err) => {
                tracing::error!(err = ?err, "unknown error");

                Self::internal(err.to_string())
            }
        }
    }
}

impl From<DiscordError> for ApiError {
    fn from(value: DiscordError) -> Self {
        match value {
            DiscordError::ApiRequest(error) => {
                tracing::error!(err = ?error, "api request error");

                Self::internal(error.to_string())
            }
            DiscordError::SerdeJson(error) => {
                tracing::error!(err = ?error, "serde json error");

                Self::internal(format!("{:#?}", error))
            }
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::internal(format!("error making request to external service: {value}",))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::internal(format!("unknown error: {value}",))
    }
}
