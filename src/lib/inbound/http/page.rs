use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use reqwest::Url;
use serde::{Deserialize, Deserializer};

use crate::{
    auth::{self, AppPermission},
    domain::intro_tool::{
        models::guild::{
            ChannelName, ExternalGuild, ExternalGuildId, GuildId, GuildRef, Intro, User,
        },
        ports::IntroToolService,
    },
    htmx::{Build, HtmxBuilder, Tag},
    inbound::{
        http::ApiState,
        response::{ApiError, ErrorAsRedirect, PageError},
    },
    outbound::discord::{DiscordAuthParams, DiscordService},
};

pub async fn home<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    user: Option<User>,
) -> Result<impl axum::response::IntoResponse, Redirect> {
    if let Some(user) = user {
        let needs_setup = state.intro_tool_service.needs_setup().await;
        let user_guilds = state
            .intro_tool_service
            .get_user_guilds(user.name())
            .await
            .as_redirect(&state.origin, "login")?;

        let can_add_guild = user.permissions().can(AppPermission::AddGuild);

        let discord_guilds: Vec<ExternalGuild> = if can_add_guild {
            state
                .intro_tool_service
                .get_user_external_guilds::<DiscordService>(user.external_token().to_string())
                .await
                .inspect_err(|err| tracing::error!(?err, "failed to get user external guilds"))
                .unwrap_or_default()
                .into_iter()
                .filter(|external_guilds| {
                    !user_guilds
                        .iter()
                        .any(|g| g.id() == external_guilds.id.0.into())
                })
                .collect()
        } else {
            vec![]
        };

        let guild_list = if needs_setup {
            if can_add_guild {
                HtmxBuilder::new(Tag::Empty).builder(Tag::Div, |b| {
                    b.attribute("class", "container")
                        .builder_text(Tag::Header2, "Select a Guild to setup")
                        .push_builder(setup_guild_list(&state.origin, &discord_guilds))
                })
            } else {
                HtmxBuilder::new(Tag::Empty).builder(Tag::Div, |b| {
                    b.attribute("class", "container")
                        .builder_text(Tag::Paragraph, "Looks like there aren't any guilds yet.")
                })
            }
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
                    //b.push_builder(guild_list)

                    // TODO:
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

pub async fn login<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    user: Option<User>,
) -> Result<Html<String>, Redirect> {
    if user.is_some() {
        Err(Redirect::to(&format!("{}/", state.origin)))
    } else {
        let authorize_uri = format!(
            "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}/v2/auth&response_type=code&scope=guilds.members.read+guilds+identify",
            state.secrets.client_id, state.origin
        );

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
}

pub async fn guild_dashboard<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    user: User,
    Path(guild_id): Path<u64>,
) -> Result<Html<String>, Redirect> {
    let guild = state
        .intro_tool_service
        .get_guild(guild_id)
        .await
        .as_redirect(&state.origin, "login")?;
    let user_guilds = state
        .intro_tool_service
        .get_user_guilds(user.name())
        .await
        .as_redirect(&state.origin, "login")?;
    let guild_intros = state
        .intro_tool_service
        .get_guild_intros(guild_id.into())
        .await
        .as_redirect(&state.origin, "login")?;

    // does user have access to this guild
    if !user_guilds
        .iter()
        .any(|guild_ref| guild_ref.id() == guild.id())
    {
        return Err(Redirect::to(&format!("{}/error", state.origin)));
    }

    let user_guild_perms = guild
        .users()
        .iter()
        .find(|(guild_user, _)| guild_user.name() == user.name())
        .map(|(_, perms)| *perms)
        .unwrap_or_default();

    let is_moderator = user_guild_perms.can(auth::Permission::Moderator);
    let can_add_channel = user_guild_perms.can(auth::Permission::AddChannel);
    let can_upload = user_guild_perms.can(auth::Permission::UploadSounds);
    let mod_dashboard = moderator_dashboard(
        &state,
        &state.secrets.bot_token,
        guild_id.into(),
        user_guild_perms,
    )
    .await;

    Ok(Html(
        HtmxBuilder::new(Tag::Html)
            .push_builder(page_header("MemeJoin - Dashboard"))
            .builder(Tag::Nav, |b| {
                b.builder(Tag::HeaderGroup, |b| {
                    b.attribute("class", "container")
                        .builder(Tag::Header1, |b| b.text("MemeJoin - A bot for user intros"))
                        .builder_text(Tag::Header6, &format!("{} - {}", user.name(), guild.name()))
                })
            })
            .builder(Tag::Empty, |b| {
                let b = if is_moderator || can_add_channel {
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
                let b = if can_upload {
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

                            for guild_channel in guild.channels() {
                                let intros = user.intros().get(&(guild.id(), guild_channel.name().clone())).map(|intros| intros.iter()).unwrap_or_default();

                                b = b.builder(Tag::Details, |b| {
                                    let mut b = b;
                                    if guild.channels().len() < 2 {
                                        b = b.attribute("open", "");
                                    }
                                    b.builder_text(Tag::Summary, guild_channel.name().as_ref()).builder(
                                        Tag::Div,
                                        |b| {
                                            b.attribute("id", "channel-intro-selector")
                                                //.attribute("class", "grid")
                                                .attribute("style", "display: flex; flex-direction: column; align-items: center; max-height: 50%; overflow: hidden;")
                                                .push_builder(channel_intro_selector(
                                                    &state.origin,
                                                    guild_id,
                                                    guild_channel.name(),
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

async fn moderator_dashboard<S: IntroToolService>(
    state: &ApiState<S>,
    bot_token: &str,
    guild_id: GuildId,
    user_permissions: auth::Permissions,
) -> HtmxBuilder {
    let permissions_editor = permissions_editor(state, guild_id).await;
    //let channel_editor = channel_editor(state, bot_token, guild_id).await;

    let mut b = HtmxBuilder::new(Tag::Empty);

    if user_permissions.can(auth::Permission::Moderator) {
        b = b.push_builder(permissions_editor);
    }
    //if user_permissions.can(auth::Permission::AddChannel) {
    //    b = b.push_builder(channel_editor);
    //}

    b
}

async fn permissions_editor<S: IntroToolService>(
    state: &ApiState<S>,
    guild_id: GuildId,
) -> HtmxBuilder {
    let guild_users = state
        .intro_tool_service
        .get_guild_users(guild_id)
        .await
        .unwrap_or_default();

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

                for (user, permissions) in guild_users {
                    b = b.builder(Tag::TableRow, |b| {
                        let mut b = b.builder_text(Tag::TableData, user.name());

                        for perm in enum_iterator::all::<auth::Permission>() {
                            if perm == auth::Permission::Moderator || perm == auth::Permission::None
                            {
                                continue;
                            }

                            b = b.builder(Tag::TableData, |b| {
                                b.builder(Tag::Input, |b| {
                                    let mut b = b
                                        .attribute("type", "checkbox")
                                        .attribute("name", &format!("{}#{}", user.name(), perm));

                                    if permissions.can(auth::Permission::Moderator) {
                                        b = b.flag("disabled");
                                    }

                                    if permissions.can(perm) {
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

pub async fn auth<S: IntroToolService>(
    State(state): State<ApiState<S>>,
    Query(params): Query<HashMap<String, String>>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), PageError> {
    let Some(code) = params.get("code") else {
        return Err(ApiError::bad_request("no code").into());
    };

    tracing::info!("attempting to get access token with code {}", code);

    let token = state
        .intro_tool_service
        // TODO: decoulple discord from HTTP server
        .authenticate_user::<DiscordService>(DiscordAuthParams {
            origin: state.origin.clone(),
            code: code.clone(),
            client_id: state.secrets.client_id.clone(),
            client_secret: state.secrets.client_secret.clone(),
        })
        .await
        .map_err(ApiError::from)?;
    let uri = Url::parse(&state.origin).expect("should be a valid url");

    let mut cookie = Cookie::new("access_token", token);
    cookie.set_path(uri.path().to_string());
    cookie.set_secure(true);

    Ok((jar.add(cookie), Redirect::to(&format!("{}/", state.origin))))
}

pub fn page_header(title: &str) -> HtmxBuilder {
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

fn guild_list<'a>(origin: &str, guilds: impl Iterator<Item = &'a GuildRef>) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).ul(|b| {
        let mut b = b;
        for guild in guilds {
            b = b.li(|b| b.link(guild.name(), &format!("{}/guild/{}", origin, guild.id())));
        }

        b
    })
}

fn setup_guild_list<'a>(origin: &str, user_guilds: &[ExternalGuild]) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).ul(|b| {
        let mut b = b;
        for guild in user_guilds {
            b = b.li(|b| {
                b.link(
                    &guild.name,
                    // TODO: url encode the name
                    &format!("{}/guild/{}/setup?name={}", origin, guild.id.0, guild.name),
                )
            });
        }

        b
    })
}

pub fn channel_intro_selector<'a>(
    origin: &str,
    guild_id: u64,
    channel_name: &ChannelName,
    intros: impl Iterator<Item = &'a Intro>,
    guild_intros: impl Iterator<Item = &'a Intro>,
) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty)
        .builder(Tag::Div, |b| {
            b.attribute("style", "width: 100%; padding: 16px; text-align: center;")
                .builder(Tag::HeaderGroup, |b| {
                    b.attribute("style", "text-align: center;")
                        .builder_text(Tag::Header4, "Your Current Intro")
                })
            .builder(Tag::Empty, |b| {
                let mut b = b;

                for intro in intros {
                    b = b.builder_text(Tag::Paragraph, intro.name());
                }

                b
            })
            .builder(Tag::HorizontalRule, |b| b)
        })
    .builder(Tag::Div, |b| {
        b.attribute("style", "display: flex; flex-direction: column; justify-content: space-between; align-items: center; width: 100%; height: 100%; padding: 18px;")
            .builder_text(Tag::Strong, "Select Intros")
                .push_builder(intro_list(
                    guild_intros,
                    "Select Intro",
                    &format!("{}/v2/intros/add/{}/{}", origin, guild_id, channel_name.as_ref()),
                ))
        })
}

fn intro_list<'a>(intros: impl Iterator<Item = &'a Intro>, label: &str, post: &str) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).form(|b| {
        b.attribute("class", "container")
            .hx_post(post)
            .hx_target("closest #channel-intro-selector")
            .builder(Tag::FieldSet, |b| {
                let mut b = b
                    .attribute("class", "container")
                    .attribute("style", "height: 256px; overflow: auto");
                for intro in intros {
                    b = b.builder(Tag::Label, |b| {
                        b.attribute("style", "padding: 4px;")
                            .builder(Tag::Input, |b| {
                                b.attribute("type", "radio")
                                    .attribute("name", "intro")
                                    .attribute("value", &intro.id().to_string())
                            })
                            .builder_text(Tag::Empty, intro.name())
                    });
                }

                b
            })
            .button(|b| b.attribute("type", "submit").text(label))
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
