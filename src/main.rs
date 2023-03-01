#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(async_closure)]

mod routes;
pub mod settings;

use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use futures::StreamExt;
use songbird::tracks::TrackQueue;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

use serde::Deserialize;
use serenity::async_trait;
use serenity::model::prelude::{Channel, ChannelId, GuildId, Member, Ready};
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::*;

use crate::settings::{Intro, Settings};

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

fn spawn_api(settings: Arc<Mutex<Settings>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let api = Router::new()
            .route("/health", get(routes::health))
            .route("/me/:user", get(routes::me))
            .route("/intros/:guild", get(routes::intros))
            .layer(
                CorsLayer::new()
                    .allow_origin("*".parse::<HeaderValue>().unwrap())
                    .allow_methods([Method::GET]),
            )
            .with_state(settings);
        let addr = SocketAddr::from(([0, 0, 0, 0], 7756));
        info!("socket listening on {addr}");
        axum::Server::bind(&addr)
            .serve(api.into_make_service())
            .await
            .unwrap();
    })
}

async fn spawn_bot(settings: Arc<Mutex<Settings>>) -> Vec<tokio::task::JoinHandle<()>> {
    let mut tasks = vec![];

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
    tasks.push(tokio::spawn(async move {
        if let Err(err) = client.start().await {
            error!("An error occurred while running the client: {err:?}");
        }
    }));

    tasks.push(tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                HandlerMessage::Ready(ctx) => {
                    info!("Got Ready message");
                    let settings = settings.lock().await;

                    let songbird = songbird::get(&ctx).await.expect("no songbird instance");

                    for guild_id in settings.guilds.keys() {
                        let handler_lock = songbird.get_or_insert(GuildId(*guild_id));

                        let mut handler = handler_lock.lock().await;

                        handler.add_global_event(
                            songbird::Event::Track(songbird::TrackEvent::End),
                            TrackEventHandler {
                                tx: tx.clone(),
                                guild_id: GuildId(*guild_id),
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
                    let settings = settings.lock().await;

                    let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx.cache) else {
                        error!("Failed to get cached channel from member!");
                        continue;
                    };

                    let Some(guild_settings) = settings.guilds.get(channel.guild_id.as_u64()) else { continue; };
                    let Some(channel_settings) = guild_settings.channels.get(channel.name()) else { continue; };
                    let Some(user) = channel_settings.users.get(&member.user.name) else { continue; };

                    // TODO: randomly choose a intro to play
                    let Some(intro) = user.intros.first() else { continue; };

                    let source = match guild_settings.intros.get(intro.index) {
                            Some(Intro::Online(intro)) => match songbird::ytdl(&intro.url).await {
                                Ok(source) => source,
                                Err(err) => {
                                    error!(
                                        "Error starting youtube source from {}: {err:?}",
                                        intro.url
                                        );
                                    continue;
                                }
                            },
                            Some(Intro::File(intro)) => {
                                match songbird::ffmpeg(format!("sounds/{}", &intro.filename)).await {
                                    Ok(source) => source,
                                    Err(err) => {
                                        error!(
                                            "Error starting file source from {}: {err:?}",
                                            intro.filename
                                        );
                                        continue;
                                    }
                                }
                            },
                            None => {
                                error!(
                                    "Failed to find intro for user {} on guild {} in channel {}, IntroIndex: {}",
                                    member.user.name, 
                                    channel.guild_id.as_u64(),
                                    channel.name(),
                                    intro.index
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
    }));

    tasks
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    let settings = serde_json::from_str::<Settings>(
        &std::fs::read_to_string("config/settings.json").expect("no config/settings.json"),
    )
    .expect("error parsing settings file");

    let (run_api, run_bot) = (settings.run_api, settings.run_bot);

    info!("{settings:?}");

    let mut tasks = vec![];

    let settings = Arc::new(Mutex::new(settings));
    if run_api {
        tasks.push(spawn_api(settings.clone()));
    }
    if run_bot {
        tasks.append(&mut spawn_bot(settings.clone()).await);
    }

    let tasks = futures::stream::iter(tasks);
    let mut buffered = tasks.buffer_unordered(5);
    while buffered.next().await.is_some() {}

    let _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shuttdown down.");
}
