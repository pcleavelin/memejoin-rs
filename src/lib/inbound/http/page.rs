use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
};

use crate::{
    htmx::{Build, HtmxBuilder, Tag},
    lib::{
        domain::intro_tool::{
            models::guild::{ChannelName, GuildRef, Intro, User},
            ports::IntroToolService,
        },
        inbound::{http::ApiState, response::ErrorAsRedirect},
    },
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
            .as_redirect(&state.origin, "/login")?;

        // TODO: get user app permissions
        // TODO: check if user can add guilds
        // TODO: fetch guilds from discord

        let can_add_guild = false;
        let discord_guilds: Vec<GuildRef> = vec![];

        let guild_list = if needs_setup {
            // TODO:
            // HtmxBuilder::new(Tag::Empty).builder(Tag::Div, |b| {
            //     b.attribute("class", "container")
            //         .builder_text(Tag::Header2, "Select a Guild to setup")
            //         .push_builder(setup_guild_list(&state.origin, &discord_guilds))
            // })
            todo!()
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
                    b.push_builder(guild_list)

                    // TODO:
                    // let mut b = b.push_builder(guild_list);
                    //
                    // if !needs_setup && can_add_guild && !discord_guilds.is_empty() {
                    //     b = b
                    //         .attribute("class", "container")
                    //         .builder_text(Tag::Header2, "Add a Guild")
                    //         .push_builder(setup_guild_list(&state.origin, &discord_guilds));
                    // }
                    //
                    // b
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
        .as_redirect(&state.origin, "/login")?;
    let user_guilds = state
        .intro_tool_service
        .get_user_guilds(user.name())
        .await
        .as_redirect(&state.origin, "/login")?;
    let guild_intros = state
        .intro_tool_service
        .get_guild_intros(guild_id.into())
        .await
        .as_redirect(&state.origin, "/login")?;

    // does user have access to this guild
    if !user_guilds
        .iter()
        .any(|guild_ref| guild_ref.id() == guild.id())
    {
        return Err(Redirect::to(&format!("{}/error", state.origin)));
    }

    let can_upload = true;

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
                // TODO:
                // let mut b = if is_moderator || can_add_channel {
                //     b.builder(Tag::Div, |b| {
                //         b.attribute("class", "container")
                //             .builder(Tag::Article, |b| {
                //                 b.builder_text(Tag::Header, "Server Settings")
                //                     .push_builder(mod_dashboard)
                //             })
                //     })
                // } else {
                //     b
                // };
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
                                                .attribute("style", "display: flex; align-items: flex-end; max-height: 50%; overflow: hidden;")
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

fn guild_list<'a>(origin: &str, guilds: impl Iterator<Item = &'a GuildRef>) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).ul(|b| {
        let mut b = b;
        for guild in guilds {
            b = b.li(|b| b.link(guild.name(), &format!("{}/guild/{}", origin, guild.id())));
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
            b.attribute("style", "display: flex; flex-direction: column; justify-content: space-between; align-items: center; width: 100%; height: 100%; padding: 16px;")
                .builder_text(Tag::Strong, "Your Current Intros")
                .push_builder(intro_list(
                    intros,
                    "Remove Intro",
                    &format!("{}/v2/intros/remove/{}/{}", origin, guild_id, channel_name.as_ref()),
                ))
        })
        .builder(Tag::Div, |b| {
            b.attribute("style", "display: flex; flex-direction: column; justify-content: space-between; align-items: center; width: 100%; height: 100%; padding: 16px;")
            .builder_text(Tag::Strong, "Select Intros")
                .push_builder(intro_list(
                    guild_intros,
                    "Add Intro",
                    &format!("{}/v2/intros/add/{}/{}", origin, guild_id, channel_name.as_ref()),
                ))
        })
}

fn intro_list<'a>(intros: impl Iterator<Item = &'a Intro>, label: &str, post: &str) -> HtmxBuilder {
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
                                .attribute("name", &intro.id().to_string())
                        })
                        .builder_text(Tag::Paragraph, intro.name())
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
