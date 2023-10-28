#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(async_closure)]

mod auth;
mod db;
mod htmx;
mod media;
mod page;
mod routes;
pub mod settings;

use axum::http::Method;
use axum::routing::{get, post};
use axum::Router;
use settings::ApiState;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};

use serenity::async_trait;
use serenity::model::prelude::{Channel, ChannelId, GuildId, Member, Ready};
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::*;

use crate::settings::Settings;

enum HandlerMessage {
    Ready(Context),
    PlaySound(Context, Member, ChannelId),
    TrackEnded(GuildId),
}

struct Handler {
    tx: std::sync::Mutex<mpsc::Sender<HandlerMessage>>,
}

struct TrackEventHandler {
    tx: mpsc::Sender<HandlerMessage>,
    guild_id: GuildId,
}

#[async_trait]
impl songbird::EventHandler for TrackEventHandler {
    async fn act<'a, 'b, 'c>(
        &'a self,
        ctx: &'b songbird::EventContext<'c>,
    ) -> Option<songbird::Event> {
        if let songbird::EventContext::Track(_) = ctx {
            if let Err(err) = self
                .tx
                .send(HandlerMessage::TrackEnded(self.guild_id))
                .await
            {
                error!("Failed to send track end message to handler: {err}");
            }
        }

        None
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let tx = self
            .tx
            .lock()
            .expect("failed to get message sender lock")
            .clone();

        tx.send(HandlerMessage::Ready(ctx))
            .await
            .unwrap_or_else(|err| panic!("failed to send ready message to handler: {err}"));

        info!("{} is ready", ready.user.name);
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        if old.is_none() {
            if let (Some(member), Some(channel_id)) = (new.member, new.channel_id) {
                if member.user.name == "MemeJoin" {
                    return;
                }

                info!(
                    "{}#{} joined voice channel {:?} in {:?}",
                    member.user.name,
                    member.user.discriminator,
                    channel_id.name(&ctx.cache).await,
                    member
                        .guild_id
                        .name(&ctx.cache)
                        .unwrap_or("no_guild_name".to_string())
                );

                let tx = self
                    .tx
                    .lock()
                    .expect("couldn't get lock for Handler messenger")
                    .clone();

                if let Err(err) = tx
                    .send(HandlerMessage::PlaySound(ctx, member, channel_id))
                    .await
                {
                    error!("Failed to send play sound message to handler: {err}");
                }
            }
        }
    }
}

fn spawn_api(db: Arc<tokio::sync::Mutex<db::Database>>) {
    let secrets = auth::DiscordSecret {
        client_id: env::var("DISCORD_CLIENT_ID").expect("expected DISCORD_CLIENT_ID env var"),
        client_secret: env::var("DISCORD_CLIENT_SECRET")
            .expect("expected DISCORD_CLIENT_SECRET env var"),
    };
    let origin = env::var("APP_ORIGIN").expect("expected APP_ORIGIN");

    let state = ApiState {
        db,
        secrets,
        origin: origin.clone(),
    };

    tokio::spawn(async move {
        let api = Router::new()
            .route("/", get(page::home))
            .route("/index.html", get(page::home))
            .route("/login", get(page::login))
            .route("/guild/:guild_id", get(page::guild_dashboard))
            .route(
                "/guild/:guild_id/permissions/update",
                post(routes::update_guild_permissions),
            )
            .route("/v2/auth", get(routes::v2_auth))
            .route(
                "/v2/intros/add/:guild_id/:channel",
                post(routes::v2_add_intro_to_user),
            )
            .route(
                "/v2/intros/remove/:guild_id/:channel",
                post(routes::v2_remove_intro_from_user),
            )
            .route("/v2/intros/:guild/add", get(routes::v2_add_guild_intro))
            .route(
                "/v2/intros/:guild/upload",
                post(routes::v2_upload_guild_intro),
            )
            .route("/health", get(routes::health))
            .layer(
                CorsLayer::new()
                    .allow_origin([origin.parse().unwrap()])
                    .allow_headers(Any)
                    .allow_methods([Method::GET, Method::POST, Method::DELETE]),
            )
            .with_state(state);
        let addr = SocketAddr::from(([0, 0, 0, 0], 8100));
        info!("socket listening on {addr}");
        axum::Server::bind(&addr)
            .serve(api.into_make_service())
            .await
            .unwrap();
    });
}

async fn spawn_bot(db: Arc<tokio::sync::Mutex<db::Database>>) {
    let token = env::var("DISCORD_TOKEN").expect("expected DISCORD_TOKEN env var");
    let songbird = songbird::Songbird::serenity();

    let (tx, mut rx) = mpsc::channel(10);

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            tx: std::sync::Mutex::new(tx.clone()),
        })
        .register_songbird_with(songbird.clone())
        .await
        .expect("Error creating client");

    info!("Starting bot with token '{token}'");
    tokio::spawn(async move {
        if let Err(err) = client.start().await {
            error!("An error occurred while running the client: {err:?}");
        }
    });

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                HandlerMessage::Ready(ctx) => {
                    info!("Got Ready message");

                    let songbird = songbird::get(&ctx).await.expect("no songbird instance");

                    let guilds = match db.lock().await.get_guilds() {
                        Ok(guilds) => guilds,
                        Err(err) => {
                            error!(?err, "failed to get guild on bot ready");
                            continue;
                        }
                    };

                    for guild in guilds {
                        let handler_lock = songbird.get_or_insert(GuildId(guild.id));

                        let mut handler = handler_lock.lock().await;

                        handler.add_global_event(
                            songbird::Event::Track(songbird::TrackEvent::End),
                            TrackEventHandler {
                                tx: tx.clone(),
                                guild_id: GuildId(guild.id),
                            },
                        );
                    }
                }
                HandlerMessage::TrackEnded(guild_id) => {
                    info!("Got TrackEnded message");

                    if let Some(manager) = songbird.get(guild_id) {
                        let mut handler = manager.lock().await;
                        let queue = handler.queue();

                        if queue.is_empty() {
                            info!("Track Queue is empty, leaving voice channel");
                            if let Err(err) = handler.leave().await {
                                error!("Failed to leave channel: {err:?}");
                            }
                        }
                    }
                }

                HandlerMessage::PlaySound(ctx, member, channel_id) => {
                    info!("Got PlaySound message");

                    let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx.cache)
                    else {
                        error!("Failed to get cached channel from member!");
                        continue;
                    };

                    let intros = match db.lock().await.get_user_channel_intros(
                        &member.user.name,
                        channel.guild_id.0,
                        channel.name(),
                    ) {
                        Ok(intros) => intros,
                        Err(err) => {
                            error!(
                                ?err,
                                "failed to get user channel intros when playing sound through bot"
                            );
                            continue;
                        }
                    };

                    // TODO: randomly choose a intro to play
                    let Some(intro) = intros.first() else {
                        error!("couldn't get user intro, none exist");
                        continue;
                    };

                    let source = match songbird::ffmpeg(format!("sounds/{}", &intro.filename)).await
                    {
                        Ok(source) => source,
                        Err(err) => {
                            error!(
                                "Error starting file source from {}: {err:?}",
                                intro.filename
                            );
                            continue;
                        }
                    };

                    match songbird.join(member.guild_id, channel_id).await {
                        (handler_lock, Ok(())) => {
                            let mut handler = handler_lock.lock().await;

                            let _track_handler = handler.enqueue_source(source);
                            // TODO: set volume
                        }

                        (_, Err(err)) => {
                            error!("Failed to join voice channel {}: {err:?}", channel.name());
                        }
                    }
                }
            }
        }
    });
}

#[tokio::main]
#[instrument]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let settings = serde_json::from_str::<Settings>(
        &std::fs::read_to_string("config/settings.json").expect("no config/settings.json"),
    )
    .expect("error parsing settings file");
    info!("{settings:?}");

    let (run_api, run_bot) = (settings.run_api, settings.run_bot);
    let db = Arc::new(tokio::sync::Mutex::new(
        db::Database::new("./config/db.sqlite").expect("couldn't open sqlite db"),
    ));

    if run_api {
        spawn_api(db.clone());
    }
    if run_bot {
        spawn_bot(db).await;
    }

    info!("spawned background tasks");

    let _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shuttdown down.");

    Ok(())
}
