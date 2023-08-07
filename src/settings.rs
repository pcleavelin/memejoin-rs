use std::sync::Arc;

use crate::{
    auth,
    db::{self, Database},
};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts, response::Redirect};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use tracing::error;

// TODO: make this is wrapped type so cloning isn't happening
#[derive(Clone)]
pub(crate) struct ApiState {
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub secrets: auth::DiscordSecret,
    pub origin: String,
}

#[async_trait]
impl FromRequestParts<ApiState> for db::User {
    type Rejection = Redirect;

    async fn from_request_parts(
        Parts { headers, .. }: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&headers);

        if let Some(token) = jar.get("access_token") {
            match state.db.lock().await.get_user_from_api_key(token.value()) {
                Ok(user) => {
                    let now = Utc::now().naive_utc();
                    if user.api_key_expires_at < now || user.discord_token_expires_at < now {
                        Err(Redirect::to(&format!("{}/login", state.origin)))
                    } else {
                        Ok(user)
                    }
                }
                Err(err) => {
                    error!(?err, "failed to authenticate user");

                    Err(Redirect::to(&format!("{}/login", state.origin)))
                }
            }
        } else {
            Err(Redirect::to(&format!("{}/login", state.origin)))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) run_api: bool,
    #[serde(default)]
    pub(crate) run_bot: bool,
}
impl TypeMapKey for Settings {
    type Value = Arc<Settings>;
}
