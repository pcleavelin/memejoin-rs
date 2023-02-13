#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(async_closure)]

use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use serde::Deserialize;
use serenity::async_trait;
use serenity::model::prelude::{Channel, GuildId, Message, Ready};
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::prelude::*;
use songbird::SerenityInit;
use tracing::*;

struct Handler;

struct TrackEventHandler {
    ctx: Context,
    guild_id: GuildId,
}

#[async_trait]
impl songbird::EventHandler for TrackEventHandler {
    async fn act<'a, 'b, 'c>(
        &'a self,
        ctx: &'b songbird::EventContext<'c>,
    ) -> Option<songbird::Event> {
        if let songbird::EventContext::Track(track) = ctx {
            if let Some(context) = track.get(0) {
                if context.0.playing == songbird::tracks::PlayMode::End
                    || context.0.playing == songbird::tracks::PlayMode::Stop
                {
                    let manager = songbird::get(&self.ctx).await.expect("should get manager");
                    if let Err(err) = manager.leave(self.guild_id).await {
                        error!("Failed to leave voice channel: {err:?}");
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Settings {
    #[serde(alias = "userEnteredSoundDelay")]
    sound_delay: u64,
    channels: HashMap<String, ChannelSettings>,
}

impl TypeMapKey for Settings {
    type Value = Arc<Settings>;
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
    volume: i32,
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
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is ready", ready.user.name);
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        if old.is_none() {
            if let (Some(member), Some(channel_id)) = (new.member, new.channel_id) {
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

                if member.user.name == "MemeJoin" {
                    return;
                }

                let settings = {
                    let data_read = ctx.data.read().await;

                    data_read
                        .get::<Settings>()
                        .expect("settings should exist")
                        .clone()
                };

                let Some(Channel::Guild(channel)) = channel_id.to_channel_cached(&ctx.cache) else {
                    error!("Failed to get cached channel from member!");
                    return;
                };

                let Some(user) = settings.channels.get(channel.name()).and_then(|c| c.users.get(&member.user.name)) else {
                    info!("No sound associated for {} in channel {}", member.user.name, channel.name());
                    return;
                };

                let Some(manager) = songbird::get(&ctx).await else {
                    error!("Failed to get songbird manager from context");
                    return;
                };

                match manager.join(member.guild_id, channel_id).await {
                    (handler_lock, Ok(())) => {
                        let mut handler = handler_lock.lock().await;

                        let source = match user.ty {
                            SoundType::Youtube => match songbird::ytdl(&user.sound).await {
                                Ok(source) => source,
                                Err(err) => {
                                    error!("Error starting youtube source: {err:?}");
                                    return;
                                }
                            },
                            SoundType::File => {
                                match songbird::ffmpeg(format!("sounds/{}", &user.sound)).await {
                                    Ok(source) => source,
                                    Err(err) => {
                                        error!("Error starting file source: {err:?}");
                                        return;
                                    }
                                }
                            }
                        };

                        let track_handle = handler.play_source(source);
                        if let Err(err) = track_handle.add_event(
                            songbird::Event::Track(songbird::TrackEvent::End),
                            TrackEventHandler {
                                ctx,
                                guild_id: member.guild_id,
                            },
                        ) {
                            error!("Failed to add event handler to track handle: {err:?}");
                        };
                    }

                    (_, Err(err)) => {
                        error!("Failed to join voice channel {}: {err:?}", channel.name());
                    }
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

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .register_songbird()
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;

        data.insert::<Settings>(Arc::new(settings));
    }

    info!("Starting bot with token '{token}'");
    tokio::spawn(async move {
        if let Err(err) = client.start().await {
            error!("An error occurred while running the client: {err:?}");
        }
    });

    let _ = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shuttdown down.");
}
