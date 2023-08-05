use crate::{
    auth::{self, User},
    db,
    htmx::{Build, HtmxBuilder, Tag},
    settings::{ApiState, GuildSettings, Intro, IntroFriendlyName},
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
            .style_link("https://cdn.jsdelivr.net/npm/@picocss/pico@1/css/pico.min.css")
    })
}

pub(crate) async fn home(
    State(state): State<ApiState>,
    user: Option<User>,
) -> Result<Html<String>, Redirect> {
    if let Some(user) = user {
        let db = state.db.lock().await;

        let user_guilds = db.get_user_guilds(&user.name).map_err(|err| {
            error!(?err, "failed to get user guilds");
            // TODO: change this to returning a error to the client
            Redirect::to("/login")
        })?;

        Ok(Html(
            page_header("MemeJoin - Home")
                .builder(Tag::Div, |b| {
                    b.attribute("class", "container")
                        .builder_text(Tag::Header2, "Choose a Guild")
                        .push_builder(guild_list(&state.origin, user_guilds.iter()))
                })
                .build(),
        ))
    } else {
        Err(Redirect::to(&format!("{}/login", state.origin)))
    }
}

fn guild_list<'a>(origin: &str, guilds: impl Iterator<Item = &'a db::Guild>) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).ul(|b| {
        let mut b = b;
        let mut in_any_guilds = false;
        for guild in guilds {
            in_any_guilds = true;

            b = b.li(|b| b.link(&guild.name, &format!("{}/guild/{}", origin, guild.id)));
        }

        if !in_any_guilds {
            b = b.builder_text(Tag::Header4, "Looks like you aren't in any guilds");
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
            .attribute("hx-encoding", "multipart/form-data")
            .builder(Tag::FieldSet, |b| {
                let mut b = b
                    .attribute("class", "container")
                    .attribute("style", "max-height: 50%; overflow-y: scroll");
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
    let db = state.db.lock().await;

    let user_intros = db
        .get_all_user_intros(&user.name, guild_id)
        .map_err(|err| {
            error!(?err, user = %user.name, %guild_id, "couldn't get user's intros");
            // TODO: change to actual error
            Redirect::to("/login")
        })?;
    let user_permissions = db
        .get_user_permissions(&user.name, guild_id)
        .unwrap_or_default();

    let channel_user_intros = user_intros.iter().group_by(|intro| &intro.channel_name);

    let can_upload = user_permissions.can(auth::Permission::UploadSounds);
    let is_moderator = user_permissions.can(auth::Permission::DeleteSounds);

    Ok(Html(
        HtmxBuilder::new(Tag::Html)
            .push_builder(page_header("MemeJoin - Dashboard"))
            .builder(Tag::Nav, |b| {
                b.builder(Tag::HeaderGroup, |b| {
                    b.attribute("class", "container")
                        .builder(Tag::Header1, |b| b.text("MemeJoin - A bot for user intros"))
                        .builder_text(Tag::Header6, &user.name)
                })
            })
            .builder(Tag::Empty, |b| {
                let mut b = if is_moderator {
                    b.builder(Tag::Div, |b| {
                        b.attribute("class", "container")
                            .builder(Tag::Article, |b| {
                                b.builder_text(Tag::Header, "Wow, you're a moderator")
                                    .push_builder(moderator_dashboard(&state))
                                    .builder_text(Tag::Footer, "End of super cool mod section")
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

                            for (channel_name, intros) in &channel_user_intros {
                                b = b.builder(Tag::Article, |b| {
                                    b.builder_text(Tag::Header, &channel_name).builder(
                                        Tag::Div,
                                        |b| {
                                            b.builder_text(Tag::Strong, "Your Current Intros")
                                                .push_builder(intro_list(
                                                    intros,
                                                    "Remove Intro",
                                                    &format!(
                                                        "{}/v2/intros/remove/{}/{}",
                                                        state.origin, guild_id, channel_name
                                                    ),
                                                ))
                                        },
                                    )
                                });
                            }

                            // for (channel_name, channel_settings) in &guild.channels {
                            //     if let Some(channel_user) = channel_settings.users.get(&user.name) {
                            //         let current_intros =
                            //             channel_user.intros.iter().filter_map(|intro_index| {
                            //                 Some((
                            //                     &intro_index.index,
                            //                     guild.intros.get(&intro_index.index)?,
                            //                 ))
                            //             });
                            //         let available_intros =
                            //             guild.intros.iter().filter_map(|intro| {
                            //                 if !channel_user
                            //                     .intros
                            //                     .iter()
                            //                     .any(|intro_index| intro.0 == &intro_index.index)
                            //                 {
                            //                     Some((intro.0, intro.1))
                            //                 } else {
                            //                     None
                            //                 }
                            //             });
                            //         b = b.builder(Tag::Article, |b| {
                            //             b.builder_text(Tag::Header, channel_name).builder(
                            //                 Tag::Div,
                            //                 |b| {
                            //                     b.builder_text(Tag::Strong, "Your Current Intros")
                            //                         .push_builder(intro_list(
                            //                             current_intros,
                            //                             "Remove Intro",
                            //                             &format!(
                            //                                 "{}/v2/intros/remove/{}/{}",
                            //                                 state.origin, guild_id, channel_name
                            //                             ),
                            //                         ))
                            //                         .builder_text(Tag::Strong, "Select Intros")
                            //                         .push_builder(intro_list(
                            //                             available_intros,
                            //                             "Add Intro",
                            //                             &format!(
                            //                                 "{}/v2/intros/add/{}/{}",
                            //                                 state.origin, guild_id, channel_name
                            //                             ),
                            //                         ))
                            //                 },
                            //             )
                            //         });
                            //     }
                            // }

                            b
                        })
                })
            })
            .build(),
    ))
}

fn upload_form(origin: &str, guild_id: u64) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).form(|b| {
        b.attribute("class", "container")
            .hx_post(&format!("{}/v2/intros/{}/upload", origin, guild_id))
            .attribute("hx-encoding", "multipart/form-data")
            .builder(Tag::FieldSet, |b| {
                b.attribute("class", "container")
                    .input(|b| {
                        b.attribute("name", "name")
                            .attribute("placeholder", "enter intro title")
                    })
                    .label(|b| {
                        b.text("Choose File")
                            .input(|b| b.attribute("type", "file").attribute("name", "file"))
                    })
            })
            .button(|b| b.attribute("type", "submit").text("Upload"))
    })
}

fn ytdl_form(origin: &str, guild_id: u64) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).form(|b| {
        b.attribute("class", "container")
            .hx_get(&format!("{}/v2/intros/{}/add", origin, guild_id))
            .builder(Tag::FieldSet, |b| {
                b.attribute("class", "container")
                    .label(|b| {
                        b.text("Video Url").input(|b| {
                            b.attribute("placeholder", "enter video url")
                                .attribute("name", "url")
                        })
                    })
                    .label(|b| {
                        b.text("Intro Title").input(|b| {
                            b.attribute("placeholder", "enter intro title")
                                .attribute("name", "name")
                        })
                    })
            })
            .button(|b| b.attribute("type", "submit").text("Upload"))
    })
}

fn moderator_dashboard(state: &ApiState) -> HtmxBuilder {
    HtmxBuilder::new(Tag::Empty).link("Go back to old UI", &format!("{}/old", state.origin))
}

pub(crate) async fn login(State(state): State<ApiState>) -> Html<String> {
    let authorize_uri = format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}/v2/auth&response_type=code&scope=guilds.members.read%20guilds%20identify", state.secrets.client_id, state.origin);

    Html(
        HtmxBuilder::new(Tag::Html)
            .push_builder(page_header("MemeJoin - Login"))
            .link("Login", &authorize_uri)
            .build(),
    )
}
