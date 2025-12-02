mod db;
pub mod settings;

use memejoin_rs::auth::{AppPermission, AppPermissions};
use memejoin_rs::domain::intro_tool::ports::IntroToolRepository as _;
use memejoin_rs::outbound::sqlite::Sqlite;
use songbird::driver::Bitrate;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;

use serenity::async_trait;
use serenity::model::prelude::{ChannelId, GuildId, Member, Ready};
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::*;

use memejoin_rs::{auth, domain::intro_tool, inbound, outbound};

enum HandlerMessage {
    Ready(Context),
    PlaySound(Context, Member, GuildId, ChannelId),
    TrackEnded(GuildId),

    LeaveVoiceChannel(Context, Member, GuildId),
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
            if let (Some(member), Some(guild_id), Some(channel_id)) =
                (new.member, new.guild_id, new.channel_id)
            {
                if member.user.name == "MemeJoin" {
                    return;
                }

                info!(
                    "{} joined voice channel {:?} in {:?}",
                    member.user.name,
                    ctx.cache
                        .guild(guild_id)
                        .as_ref()
                        .and_then(|guild| guild.channels.get(&channel_id))
                        .map(|channel| channel.name()),
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
                    .send(HandlerMessage::PlaySound(ctx, member, guild_id, channel_id))
                    .await
                {
                    error!("Failed to send play sound message to handler: {err}");
                }
            }
        }
    }
}

async fn spawn_bot(db: Sqlite) {
    let token = env::var("DISCORD_TOKEN").expect("expected DISCORD_TOKEN env var");
    let songbird = songbird::Songbird::serenity();

    let (tx, mut rx) = mpsc::channel(10);
    let tx2 = tx.clone();

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

                    let guilds = match db.get_guilds().await {
                        Ok(guilds) => guilds,
                        Err(err) => {
                            error!(?err, "failed to get guild on bot ready");
                            continue;
                        }
                    };

                    for guild in guilds {
                        let handler_lock =
                            songbird.get_or_insert(GuildId::new(*guild.id().as_ref()));

                        let mut handler = handler_lock.lock().await;

                        handler.add_global_event(
                            songbird::Event::Track(songbird::TrackEvent::End),
                            TrackEventHandler {
                                tx: tx.clone(),
                                guild_id: GuildId::new(*guild.id().as_ref()),
                            },
                        );
                    }
                }
                HandlerMessage::TrackEnded(guild_id) => {
                    info!("Got TrackEnded message");

                    if let Some(call) = songbird.get(guild_id) {
                        let mut call = call.lock().await;
                        let queue = call.queue();

                        if queue.is_empty() {
                            info!("Track Queue is empty, leaving voice channel");
                            if let Err(err) = call.leave().await {
                                error!("Failed to leave channel: {err:?}");
                            }
                        }
                    }
                }
                HandlerMessage::PlaySound(ctx, member, guild_id, channel_id) => {
                    info!("Got PlaySound message");

                    let channel_name = {
                        let guild = ctx.cache.guild(guild_id);

                        let Some(channel) = guild
                            .as_ref()
                            .and_then(|guild| guild.channels.get(&channel_id))
                        else {
                            error!("Failed to get cached channel from member!");
                            continue;
                        };

                        channel.name().to_string()
                    };

                    let guild_channel_intros = match db
                        .get_user_channel_intros(&member.user.name, guild_id.get().into())
                        .await
                    {
                        Ok(intros) => intros,
                        Err(err) => {
                            error!(
                                ?err,
                                "failed to get user channel intros when playing sound through bot"
                            );
                            continue;
                        }
                    };

                    let Some(intros) = guild_channel_intros
                        .get(&(guild_id.get().into(), channel_name.clone().into()))
                    else {
                        error!("couldn't get user intro, none exist");
                        continue;
                    };

                    // TODO: randomly choose a intro to play
                    let Some(intro) = intros.first() else {
                        error!("couldn't get user intro, none exist");
                        continue;
                    };

                    let file = songbird::input::File::new(format!("sounds/{}", intro.filename()));
                    let compressed_file =
                        match songbird::input::cached::Compressed::new(file.into(), Bitrate::Auto)
                            .await
                        {
                            Ok(compressed_file) => compressed_file,
                            Err(err) => {
                                error!("Failed to compress file: {err:?}");
                                continue;
                            }
                        };

                    match songbird.join(guild_id, channel_id).await {
                        Ok(call) => {
                            let mut call = call.lock().await;

                            call.enqueue_input(compressed_file.into()).await;
                        }
                        Err(err) => {
                            error!("Failed to join voice channel {}: {err:?}", channel_name);

                            if let Err(err) = tx2
                                .send(HandlerMessage::LeaveVoiceChannel(ctx, member, guild_id))
                                .await
                            {
                                error!(
                                    "Failed to send leave voice channel message to handler: {err}"
                                );
                            }
                        }
                    }
                }
                HandlerMessage::LeaveVoiceChannel(_context, _member, guild_id) => {
                    info!("Got LeaveVoiceChannel message");

                    if let Err(err) = songbird.leave(guild_id).await {
                        error!("Failed to leave channel: {err:?}");
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

    tracing::info!("tracing initialized");

    let secrets = auth::DiscordSecret {
        client_id: env::var("DISCORD_CLIENT_ID").expect("expected DISCORD_CLIENT_ID env var"),
        client_secret: env::var("DISCORD_CLIENT_SECRET")
            .expect("expected DISCORD_CLIENT_SECRET env var"),
        bot_token: env::var("DISCORD_TOKEN").expect("expected DISCORD_TOKEN env var"),
    };
    let origin = env::var("APP_ORIGIN").expect("expected APP_ORIGIN");

    let run_api = env::var("RUN_API")
        .ok()
        .and_then(|val| {
            val.parse()
                .inspect_err(|err| tracing::error!(?err, "failed to parse RUN_BOT var"))
                .ok()
        })
        .unwrap_or_default();
    let run_bot = env::var("RUN_BOT")
        .ok()
        .and_then(|val| {
            val.parse()
                .inspect_err(|err| tracing::error!(?err, "failed to parse RUN_BOT var"))
                .ok()
        })
        .unwrap_or_default();

    tracing::info!(?run_bot, ?run_api);

    let db = outbound::sqlite::Sqlite::new("./config/db.sqlite").expect("couldn't open sqlite db");
    let local_audio_fetcher = outbound::ffmpeg::Ffmpeg;
    let remote_audio_fetcher = outbound::ytdlp::Ytdlp;

    if run_bot {
        spawn_bot(db.clone()).await;
    }

    if run_api {
        if let Ok(impersonated_username) = env::var("IMPERSONATED_USERNAME") {
            let test_permissions = env::var("TEST_PERMISSIONS")
                .map(|s| {
                    s.split(',').map(AppPermission::from_str).fold(
                        AppPermissions::default(),
                        |mut acc, perm| {
                            acc.add(perm.expect("unknown permission"));
                            acc
                        },
                    )
                })
                .unwrap_or_default();

            let service = intro_tool::service::Service::new(
                db.clone(),
                remote_audio_fetcher,
                local_audio_fetcher,
            );
            let service = intro_tool::debug_service::DebugService::new(
                service,
                impersonated_username,
                test_permissions,
            );

            let http_server = inbound::http::HttpServer::new(service, secrets, origin)
                .expect("couldn't start http server");

            tokio::spawn(async move {
                http_server.run().await;
            });
        } else {
            let service = intro_tool::service::Service::new(
                db.clone(),
                remote_audio_fetcher,
                local_audio_fetcher,
            );

            let http_server = inbound::http::HttpServer::new(service, secrets, origin)
                .expect("couldn't start http server");

            tokio::spawn(async move {
                http_server.run().await;
            });
        }
    }

    info!("spawned background tasks");

    let _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down.");

    Ok(())
}
