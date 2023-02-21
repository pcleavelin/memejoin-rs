#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(async_closure)]

use songbird::tracks::TrackQueue;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc;

use serde::Deserialize;
use serenity::async_trait;
use serenity::model::prelude::{Channel, ChannelId, GuildId, Member, Ready};
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::*;

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

#[derive(Debug, Clone, Deserialize)]
struct Settings {
    guilds: HashMap<u64, GuildSettings>,
}
impl TypeMapKey for Settings {
    type Value = Arc<Settings>;
}

#[derive(Debug, Clone, Deserialize)]
struct GuildSettings {
    #[serde(alias = "userEnteredSoundDelay")]
    _sound_delay: u64,
    channels: HashMap<String, ChannelSettings>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChannelSettings {
    #[serde(alias = "enterUsers")]
    users: HashMap<String, UserSettings>,
}

#[derive(Debug, Clone, Deserialize)]
struct UserSettings {
    #[serde(rename = "type")]
    ty: SoundType,

    #[serde(alias = "enterSound")]
    sound: String,
    #[serde(alias = "youtubeVolume")]
    _volume: i32,
}

#[derive(Debug, Clone, Deserialize)]
enum SoundType {
    #[serde(alias = "file")]
    File,
    #[serde(alias = "youtube")]
    Youtube,
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

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("expected DISCORD_TOKEN env var");

    let settings = serde_json::from_str::<Settings>(
        &std::fs::read_to_string("config/settings.json").expect("no config/settings.json"),
    )
    .expect("error parsing settings file");

    info!("{settings:?}");

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

                    let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx.cache) else {
                        error!("Failed to get cached channel from member!");
                        continue;
                    };

                    let Some(user) = settings.guilds.get(channel.guild_id.as_u64())
                        .and_then(|guild| guild.channels.get(channel.name()))
                        .and_then(|c| c.users.get(&member.user.name))
                    else {
                        info!("No sound associated for {} in channel {}", member.user.name, channel.name());
                        continue;
                    };

                    let source = match user.ty {
                        SoundType::Youtube => match songbird::ytdl(&user.sound).await {
                            Ok(source) => source,
                            Err(err) => {
                                error!(
                                    "Error starting youtube source from {}: {err:?}",
                                    user.sound
                                );
                                continue;
                            }
                        },
                        SoundType::File => {
                            match songbird::ffmpeg(format!("sounds/{}", &user.sound)).await {
                                Ok(source) => source,
                                Err(err) => {
                                    error!(
                                        "Error starting file source from {}: {err:?}",
                                        user.sound
                                    );
                                    continue;
                                }
                            }
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

    let _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shuttdown down.");
}
