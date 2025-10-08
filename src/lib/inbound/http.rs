mod handlers;
mod page;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::Redirect,
    routing::{get, post},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use reqwest::Method;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::{
    auth,
    lib::domain::intro_tool::{models::guild::User, ports::IntroToolService},
};

#[derive(Clone)]
pub(crate) struct ApiState<S>
where
    S: IntroToolService,
{
    intro_tool_service: Arc<S>,

    pub secrets: auth::DiscordSecret,
    pub origin: String,
}

#[axum::async_trait]
impl<S: IntroToolService> FromRequestParts<ApiState<S>> for User {
    type Rejection = Redirect;

    async fn from_request_parts(
        Parts { headers, .. }: &mut Parts,
        state: &ApiState<S>,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(headers);

        if let Some(token) = jar.get("access_token") {
            match state
                .intro_tool_service
                .get_user_from_api_key(token.value())
                .await
            {
                Ok(user) => {
                    let now = Utc::now().naive_utc();
                    if user.api_key_expires_at() < now || user.discord_token_expires_at() < now {
                        Err(Redirect::to(&format!("{}/login", state.origin)))
                    } else {
                        Ok(user)
                    }
                }
                Err(err) => {
                    tracing::error!(?err, "failed to authenticate user");

                    Err(Redirect::to(&format!("{}/login", state.origin)))
                }
            }
        } else {
            Err(Redirect::to(&format!("{}/login", state.origin)))
        }
    }
}

pub struct HttpServer {
    make_service: axum::routing::IntoMakeService<axum::Router>,
}

impl HttpServer {
    pub fn new(
        intro_tool_service: impl IntroToolService,
        secrets: auth::DiscordSecret,
        origin: String,
    ) -> anyhow::Result<Self> {
        let state = ApiState {
            intro_tool_service: Arc::new(intro_tool_service),
            secrets,
            origin: origin.clone(),
        };

        let router = routes()
            .layer(
                CorsLayer::new()
                    .allow_origin([origin.parse().unwrap()])
                    .allow_headers(tower_http::cors::Any)
                    .allow_methods([Method::GET, Method::POST, Method::DELETE]),
            )
            .with_state(state);

        Ok(Self {
            make_service: router.into_make_service(),
        })
    }

    pub async fn run(self) {
        let addr = SocketAddr::from(([0, 0, 0, 0], 8100));
        info!("socket listening on {addr}");

        axum::Server::bind(&addr)
            .serve(self.make_service)
            .await
            .expect("couldn't start http server");
    }
}

fn routes<S>() -> axum::Router<ApiState<S>>
where
    S: IntroToolService,
{
    axum::Router::<ApiState<S>>::new()
        .route("/", get(page::home))
        .route("/login", get(page::login))
        .route("/guild/:guild_id", get(page::guild_dashboard))
        .route("/v2/intros/:guild/add", get(handlers::add_guild_intro))

    // .route("/guild/:guild_id/setup", get(routes::guild_setup))
    // .route(
    //     "/guild/:guild_id/add_channel",
    //     post(routes::guild_add_channel),
    // )
    // .route(
    //     "/guild/:guild_id/permissions/update",
    //     post(routes::update_guild_permissions),
    // )
    // .route("/v2/auth", get(routes::v2_auth))
    // .route(
    //     "/v2/intros/add/:guild_id/:channel",
    //     post(routes::v2_add_intro_to_user),
    // )
    // .route(
    //     "/v2/intros/remove/:guild_id/:channel",
    //     post(routes::v2_remove_intro_from_user),
    // )
    // .route("/v2/intros/:guild/add", get(routes::v2_add_guild_intro))
    // .route(
    //     "/v2/intros/:guild/upload",
    //     post(routes::v2_upload_guild_intro),
    // )
    // .route("/health", get(routes::health))
}
