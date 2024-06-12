use crate::{
    auth,
    db::{self, User},
    htmx::{Build, HtmxBuilder, Tag},
    settings::ApiState,
};
use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
};
use iter_tools::Itertools;
use tracing::error;

fn page_header(title: &str) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Html).head(|b| {
        b.title(title)
            .script(
                "https://unpkg.com/htmx.org@1.9.3",
                Some("sha384-lVb3Rd/Ca0AxaoZg5sACe8FJKF0tnUgR2Kd7ehUOG5GCcROv5uBIZsOqovBAcWua"),
            )
            // Not currently using
            // .script("https://unpkg.com/hyperscript.org@0.9.9", None)
            .style_link("https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css")
    })
}

pub(crate) async fn home(
    State(state): State<ApiState>,
    user: Option<User>,
) -> Result<Html<String>, Redirect> {
    if let Some(user) = user {
        let db = state.db.lock().await;

        let needs_setup = db
            .get_guilds()
            .map_err(|err| {
                error!(?err, "failed to get user guilds");
                // TODO: change this to returning a error to the client
                Redirect::to(&format!("{}/error", state.origin))
            })?
            .is_empty();
        let user_guilds = db.get_user_guilds(&user.name).map_err(|err| {
            error!(?err, "failed to get user guilds");
            // TODO: change this to returning a error to the client
            Redirect::to(&format!("{}/login", state.origin))
        })?;
        let user_app_permissions = db.get_user_app_permissions(&user.name).unwrap_or_default();
        let can_add_guild = user_app_permissions.can(auth::AppPermission::AddGuild);

        let client = reqwest::Client::new();
        let discord_guilds: Vec<crate::routes::DiscordUserGuild> = if can_add_guild {
            client
                .get("https://discord.com/api/v10/users/@me/guilds")
                .bearer_auth(&user.discord_token)
                .send()
                .await
                .map_err(|err| {
                    error!(?err, "failed to get guilds");
                    // TODO: change this to returning a error to the client
                    Redirect::to(&format!("{}/error", state.origin))
                })?
                .json()
                .await
                .map_err(|err| {
                    error!(?err, "failed to parse json");
                    // TODO: change this to returning a error to the client
                    Redirect::to(&format!("{}/error", state.origin))
                })?
        } else {
            vec![]
        }
        .into_iter()
        // lol, why does this need to have an explicit type annotation
        .filter(|discord_guild: &crate::routes::DiscordUserGuild| {
            !user_guilds
                .iter()
                .any(|user_guild| discord_guild.id == user_guild.id)
        })
        .collect();

        let guild_list = if needs_setup {
            HtmxBuilder::new(Tag::Empty).builder(Tag::Div, |b| {
                b.attribute("class", "container")
                    .builder_text(Tag::Header2, "Select a Guild to setup")
                    .push_builder(setup_guild_list(&state.origin, &discord_guilds))
            })
        } else {
            HtmxBuilder::new(Tag::Empty).builder(Tag::Div, |b| {
                b.attribute("class", "container")
                    .builder_text(Tag::Header2, "Choose a Guild")
                    .push_builder(guild_list(&state.origin, user_guilds.iter()))
            })
        };

        Ok(Html(
            page_header("MemeJoin - Home")
                .builder(Tag::Div, |b| {
                    let mut b = b.push_builder(guild_list);

                    if !needs_setup && can_add_guild && !discord_guilds.is_empty() {
                        b = b
                            .attribute("class", "container")
                            .builder_text(Tag::Header2, "Add a Guild")
                            .push_builder(setup_guild_list(&state.origin, &discord_guilds));
                    }

                    b
                })
                .build(),
        ))
    } else {
        Err(Redirect::to(&format!("{}/login", state.origin)))
    }
}

fn setup_guild_list(origin: &str, user_guilds: &[crate::routes::DiscordUserGuild]) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).ul(|b| {
        let mut b = b;
        for guild in user_guilds {
            b = b.li(|b| {
                b.link(
                    &guild.name,
                    // TODO: url encode the name
                    &format!("{}/guild/{}/setup?name={}", origin, guild.id, guild.name),
                )
            });
        }

        b
    })
}

fn guild_list<'a>(origin: &str, guilds: impl Iterator<Item = &'a db::Guild>) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).ul(|b| {
        let mut b = b;
        for guild in guilds {
            b = b.li(|b| b.link(&guild.name, &format!("{}/guild/{}", origin, guild.id)));
        }

        b
    })
}

fn intro_list<'a>(
    intros: impl Iterator<Item = &'a db::Intro>,
    label: &str,
    post: &str,
) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).form(|b| {
        b.attribute("class", "container")
            .hx_post(post)
            .hx_target("closest #channel-intro-selector")
            .attribute("hx-encoding", "multipart/form-data")
            .builder(Tag::FieldSet, |b| {
                let mut b = b
                    .attribute("class", "container")
                    .attribute("style", "height: 256px; overflow: auto");
                for intro in intros {
                    b = b.builder(Tag::Label, |b| {
                        b.builder(Tag::Input, |b| {
                            b.attribute("type", "checkbox")
                                .attribute("name", &intro.id.to_string())
                        })
                        .builder_text(Tag::Paragraph, &intro.name)
                    });
                }

                b
            })
            .button(|b| b.attribute("type", "submit").text(label))
    })
}

pub(crate) async fn guild_dashboard(
    State(state): State<ApiState>,
    user: User,
    Path(guild_id): Path<u64>,
) -> Result<Html<String>, Redirect> {
    let (guild_name, guild_intros, guild_channels, all_user_intros, user_permissions) = {
        let db = state.db.lock().await;

        let guild_name = db.get_guild(guild_id).map_err(|err| {
            error!(?err, %guild_id, "couldn't get guild");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;

        let guild_intros = db.get_guild_intros(guild_id).map_err(|err| {
            error!(?err, %guild_id, "couldn't get guild intros");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;
        let guild_channels = db.get_guild_channels(guild_id).map_err(|err| {
            error!(?err, %guild_id, "couldn't get guild channels");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;
        let all_user_intros = db.get_all_user_intros(guild_id).map_err(|err| {
            error!(?err, %guild_id, "couldn't get user intros");
            // TODO: change to actual error
            Redirect::to(&format!("{}/login", state.origin))
        })?;
        let user_permissions = db
            .get_user_permissions(&user.name, guild_id)
            .unwrap_or_default();

        (
            guild_name,
            guild_intros,
            guild_channels,
            all_user_intros,
            user_permissions,
        )
    };

    let can_upload = user_permissions.can(auth::Permission::UploadSounds);
    let can_add_channel = user_permissions.can(auth::Permission::AddChannel);
    let is_moderator = user_permissions.can(auth::Permission::Moderator);
    let mod_dashboard =
        moderator_dashboard(&state, &state.secrets.bot_token, guild_id, user_permissions).await;

    let user_intros = all_user_intros
        .iter()
        .filter(|intro| intro.username == user.name)
        .group_by(|intro| &intro.channel_name);

    Ok(Html(
        HtmxBuilder::new(Tag::Html)
            .push_builder(page_header("MemeJoin - Dashboard"))
            .builder(Tag::Nav, |b| {
                b.builder(Tag::HeaderGroup, |b| {
                    b.attribute("class", "container")
                        .builder(Tag::Header1, |b| b.text("MemeJoin - A bot for user intros"))
                        .builder_text(Tag::Header6, &format!("{} - {}", user.name, guild_name))
                })
            })
            .builder(Tag::Empty, |b| {
                let mut b = if is_moderator || can_add_channel {
                    b.builder(Tag::Div, |b| {
                        b.attribute("class", "container")
                            .builder(Tag::Article, |b| {
                                b.builder_text(Tag::Header, "Server Settings")
                                    .push_builder(mod_dashboard)
                            })
                    })
                } else {
                    b
                };
                b = if can_upload {
                    b.builder(Tag::Div, |b| {
                        b.attribute("class", "container")
                            .builder(Tag::Article, |b| {
                                b.builder_text(Tag::Header, "Upload New Intro")
                                    .push_builder(upload_form(&state.origin, guild_id))
                            })
                    })
                    .builder(Tag::Div, |b| {
                        b.attribute("class", "container")
                            .builder(Tag::Article, |b| {
                                b.builder_text(Tag::Header, "Upload New Intro from Url")
                                    .push_builder(ytdl_form(&state.origin, guild_id))
                            })
                    })
                } else {
                    b
                };

                b.builder(Tag::Div, |b| {
                    b.attribute("class", "container")
                        .builder(Tag::Article, |b| {
                            let mut b = b.builder_text(Tag::Header, "Guild Intros");

                            let mut user_intros = user_intros.into_iter().peekable();

                            for guild_channel_name in &guild_channels {
                                // Get user intros for this channel
                                let intros = user_intros
                                    .peeking_take_while(|(channel_name, _)| {
                                        channel_name == &guild_channel_name
                                    })
                                    .map(|(_, intros)| intros.map(|intro| &intro.intro))
                                    .flatten();

                                b = b.builder(Tag::Details, |b| {
                                    let mut b = b;
                                    if guild_channels.len() < 2 {
                                        b = b.attribute("open", "");
                                    }
                                    b.builder_text(Tag::Summary, guild_channel_name).builder(
                                        Tag::Div,
                                        |b| {
                                            b.attribute("id", "channel-intro-selector")
                                                .attribute("style", "display: flex; align-items: flex-end; max-height: 50%; overflow: hidden;")
                                                .push_builder(channel_intro_selector(
                                                    &state.origin,
                                                    guild_id,
                                                    guild_channel_name,
                                                    intros,
                                                    guild_intros.iter(),
                                                ))
                                        },
                                    )
                                });
                            }

                            b
                        })
                })
            })
            .build(),
    ))
}

pub fn channel_intro_selector<'a>(
    origin: &str,
    guild_id: u64,
    channel_name: &String,
    intros: impl Iterator<Item = &'a db::Intro>,
    guild_intros: impl Iterator<Item = &'a db::Intro>,
) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty)
        .builder(Tag::Div, |b| {
            b.attribute("style", "display: flex; flex-direction: column; justify-content: space-between; align-items: center; width: 100%; height: 100%; padding: 16px;")
                .builder_text(Tag::Strong, "Your Current Intros")
                .push_builder(intro_list(
                    intros,
                    "Remove Intro",
                    &format!("{}/v2/intros/remove/{}/{}", origin, guild_id, &channel_name),
                ))
        })
        .builder(Tag::Div, |b| {
            b.attribute("style", "display: flex; flex-direction: column; justify-content: space-between; align-items: center; width: 100%; height: 100%; padding: 16px;")
            .builder_text(Tag::Strong, "Select Intros")
                .push_builder(intro_list(
                    guild_intros,
                    "Add Intro",
                    &format!("{}/v2/intros/add/{}/{}", origin, guild_id, channel_name),
                ))
        })
}

fn upload_form(origin: &str, guild_id: u64) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).form(|b| {
        b.attribute("class", "container")
            .hx_post(&format!("{}/v2/intros/{}/upload", origin, guild_id))
            .attribute("hx-encoding", "multipart/form-data")
            .builder(Tag::FieldSet, |b| {
                b.attribute("class", "container")
                    .attribute("role", "group")
                    .input(|b| b.attribute("type", "file").attribute("name", "file"))
                    .input(|b| {
                        b.attribute("name", "name")
                            .attribute("placeholder", "enter intro title")
                    })
                    .button(|b| b.attribute("type", "submit").text("Upload"))
            })
    })
}

fn ytdl_form(origin: &str, guild_id: u64) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).form(|b| {
        b.attribute("class", "container")
            .hx_get(&format!("{}/v2/intros/{}/add", origin, guild_id))
            .builder(Tag::FieldSet, |b| {
                b.attribute("class", "container")
                    .attribute("role", "group")
                    .input(|b| {
                        b.attribute("placeholder", "enter video url")
                            .attribute("name", "url")
                    })
                    .input(|b| {
                        b.attribute("placeholder", "enter intro title")
                            .attribute("name", "name")
                    })
                    .button(|b| b.attribute("type", "submit").text("Upload"))
            })
    })
}

async fn moderator_dashboard(
    state: &ApiState,
    bot_token: &str,
    guild_id: u64,
    user_permissions: auth::Permissions,
) -> HtmxBuilder {
    let permissions_editor = permissions_editor(state, guild_id).await;
    let channel_editor = channel_editor(state, bot_token, guild_id).await;

    let mut b = HtmxBuilder::new(Tag::Empty);

    if user_permissions.can(auth::Permission::Moderator) {
        b = b.push_builder(permissions_editor);
    }
    if user_permissions.can(auth::Permission::AddChannel) {
        b = b.push_builder(channel_editor);
    }

    b
}

async fn channel_editor(state: &ApiState, bot_token: &str, guild_id: u64) -> HtmxBuilder {
    let db = state.db.lock().await;
    let added_guild_channels = db.get_guild_channels(guild_id).unwrap_or_default();

    let mut got_channels = true;
    let client = reqwest::Client::new();
    let channels: Vec<String> = {
        match client
            .get(format!(
                "https://discord.com/api/v10/guilds/{}/channels",
                guild_id
            ))
            .header("Authorization", format!("Bot {}", bot_token))
            .send()
            .await
        {
            Ok(resp) => match resp.json::<Vec<crate::routes::DiscordChannel>>().await {
                Ok(channels) => channels
                    .into_iter()
                    .filter(|channel| channel.ty == crate::routes::ChannelType::GuildVoice as u32)
                    .filter_map(|channel| channel.name)
                    .filter(|name| !added_guild_channels.contains(name))
                    .collect(),
                Err(err) => {
                    error!(?err, "failed to parse json");
                    got_channels = false;

                    vec![]
                }
            },
            Err(err) => {
                error!(?err, "failed to get channels");
                got_channels = false;

                vec![]
            }
        }
    };

    if got_channels && !channels.is_empty() {
        HtmxBuilder::new(Tag::Details)
            .builder_text(Tag::Summary, "Add Channels")
            .form(|b| {
                b.attribute("class", "container")
                    .hx_post(&format!("{}/guild/{}/add_channel", state.origin, guild_id))
                    .attribute("hx-encoding", "multipart/form-data")
                    .builder(Tag::FieldSet, |b| {
                        let mut b = b
                            .attribute("class", "container")
                            .attribute("style", "max-height: 50%; overflow-y: scroll");
                        for channel_name in channels {
                            b = b.builder(Tag::Label, |b| {
                                b.builder(Tag::Input, |b| {
                                    b.attribute("type", "checkbox")
                                        .attribute("name", &channel_name.to_string())
                                })
                                .builder_text(Tag::Paragraph, &channel_name)
                            });
                        }

                        b
                    })
                    .button(|b| b.attribute("type", "submit").text("Add Channel"))
            })
    } else if channels.is_empty() {
        HtmxBuilder::new(Tag::Empty)
    } else {
        HtmxBuilder::new(Tag::Empty).text("Failed to get channels")
    }
}

async fn permissions_editor(state: &ApiState, guild_id: u64) -> HtmxBuilder {
    let db = state.db.lock().await;
    let user_permissions = db.get_all_user_permissions(guild_id).unwrap_or_default();

    HtmxBuilder::new(Tag::Details)
        .builder_text(Tag::Summary, "Permissions")
        .form(|b| {
            b.hx_post(&format!(
                "{}/guild/{}/permissions/update",
                state.origin, guild_id
            ))
            .attribute("hx-encoding", "multipart/form-data")
            .builder(Tag::Table, |b| {
                let mut b = b.attribute("role", "grid").builder(Tag::TableHead, |b| {
                    let mut b = b.builder_text(Tag::TableHeader, "User");

                    for perm in enum_iterator::all::<auth::Permission>() {
                        if perm == auth::Permission::Moderator || perm == auth::Permission::None {
                            continue;
                        }

                        b = b.builder_text(Tag::TableHeader, &perm.to_string());
                    }

                    b
                });

                for permission in user_permissions {
                    b = b.builder(Tag::TableRow, |b| {
                        let mut b = b.builder_text(Tag::TableData, permission.0.as_str());

                        for perm in enum_iterator::all::<auth::Permission>() {
                            if perm == auth::Permission::Moderator || perm == auth::Permission::None
                            {
                                continue;
                            }

                            b = b.builder(Tag::TableData, |b| {
                                b.builder(Tag::Input, |b| {
                                    let mut b = b
                                        .attribute("type", "checkbox")
                                        .attribute("name", &format!("{}#{}", permission.0, perm));

                                    if permission.1.can(auth::Permission::Moderator) {
                                        b = b.flag("disabled");
                                    }

                                    if permission.1.can(perm) {
                                        return b.flag("checked");
                                    }

                                    b
                                })
                            });
                        }

                        b
                    });
                }

                b
            })
            .button(|b| b.attribute("type", "submit").text("Update Permissions"))
        })
}

pub(crate) async fn login(
    State(state): State<ApiState>,
    user: Option<User>,
) -> Result<Html<String>, Redirect> {
    if user.is_some() {
        Err(Redirect::to(&format!("{}/", state.origin)))
    } else {
        let authorize_uri = format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}/v2/auth&response_type=code&scope=guilds.members.read+guilds+identify", state.secrets.client_id, state.origin);

        Ok(Html(
            HtmxBuilder::new(Tag::Html)
                .push_builder(page_header("MemeJoin - Dashboard"))
                .builder(Tag::Nav, |b| {
                    b.builder(Tag::HeaderGroup, |b| {
                        b.attribute("class", "container")
                            .builder(Tag::Header1, |b| b.text("MemeJoin - A bot for user intros"))
                            .builder_text(Tag::Header6, "salad")
                    })
                })
                .builder(Tag::Main, |b| {
                    b.attribute("class", "container").builder(Tag::Anchor, |b| {
                        b.attribute("role", "button")
                            .text("Login with Discord")
                            .attribute("href", &authorize_uri)
                    })
                })
                .build(),
        ))
    }

    //Html(
    //    HtmxBuilder::new(Tag::Html)
    //        .push_builder(page_header("MemeJoin - Login"))
    //        .link("Login", &authorize_uri)
    //        .build(),
    //)
}
