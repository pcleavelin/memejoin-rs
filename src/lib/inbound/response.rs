use std::fmt::Debug;

use axum::response::Redirect;

use crate::lib::domain::intro_tool::models::guild::{
    GetChannelError, GetGuildError, GetIntroError,
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
