use crate::{
    auth::{self, User},
    htmx::{Build, HtmxBuilder, SwapMethod, Tag},
    settings::{ApiState, Intro, IntroFriendlyName},
};
use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
};
use tracing::error;

pub(crate) async fn home(user: Option<User>) -> Redirect {
    if user.is_some() {
        Redirect::to("/guild/588149178912473103")
    } else {
        Redirect::to("/login")
    }
}

fn intro_list<'a>(
    intros: impl Iterator<Item = (&'a String, &'a Intro)>,
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
                    .attribute("style", "max-height: 20%; overflow-y: scroll");
                for intro in intros {
                    b = b.builder(Tag::Label, |b| {
                        b.builder(Tag::Input, |b| {
                            b.attribute("type", "checkbox").attribute("name", &intro.0)
                        })
                        .builder_text(Tag::Paragraph, intro.1.friendly_name())
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
    let settings = state.settings.lock().await;

    let Some(guild) = settings.guilds.get(&guild_id) else {
        error!(%guild_id, "no such guild");
        return Err(Redirect::to("/"));
    };
    let Some(guild_user) = guild.users.get(&user.name) else {
        error!(%guild_id, %user.name, "no user in guild");
        return Err(Redirect::to("/"));
    };

    let is_moderator = guild_user.permissions.can(auth::Permission::DeleteSounds);

    Ok(Html(
        HtmxBuilder::new(Tag::Html)
            .head(|b| {
                b.title("MemeJoin - Dashboard")
                .script(
                    "https://unpkg.com/htmx.org@1.9.3",
                    Some("sha384-lVb3Rd/Ca0AxaoZg5sACe8FJKF0tnUgR2Kd7ehUOG5GCcROv5uBIZsOqovBAcWua"),
                )
                .script("https://unpkg.com/hyperscript.org@0.9.9", None)
                .style_link("https://cdn.jsdelivr.net/npm/@picocss/pico@1/css/pico.min.css")
            })
            .builder(Tag::Nav, |b| {
                b.builder(Tag::Header1, |b| b.text("MemeJoin - A bot for user intros"))
                    .builder_text(Tag::Paragraph, &user.name)
            })
            .builder(Tag::Main, |b| {
                if is_moderator {
                    b.builder(Tag::Article, |b| {
                        b.builder_text(Tag::Header, "Wow, you're a moderator")
                    })
                } else {
                    b
                }
                .builder(Tag::Article, |b| {
                    let mut b = b.builder_text(Tag::Header, "Guild Settings");

                    for (channel_name, channel_settings) in &guild.channels {
                        if let Some(channel_user) = channel_settings.users.get(&user.name) {
                            let current_intros =
                                channel_user.intros.iter().filter_map(|intro_index| {
                                    Some((
                                        &intro_index.index,
                                        guild.intros.get(&intro_index.index)?,
                                    ))
                                });
                            let available_intros = guild.intros.iter().filter_map(|intro| {
                                if !channel_user
                                    .intros
                                    .iter()
                                    .any(|intro_index| intro.0 == &intro_index.index)
                                {
                                    Some((intro.0, intro.1))
                                } else {
                                    None
                                }
                            });
                            b = b
                                .builder_text(Tag::Strong, channel_name)
                                .builder(Tag::Div, |b| {
                                    b.builder_text(Tag::Strong, "Your Current Intros")
                                        .push_builder(intro_list(
                                            current_intros,
                                            "Remove Intro",
                                            &format!(
                                                "/v2/intros/remove/{}/{}",
                                                guild_id, channel_name
                                            ),
                                        ))
                                        .builder_text(Tag::Strong, "Select Intros")
                                        .push_builder(intro_list(
                                            available_intros,
                                            "Add Intro",
                                            &format!(
                                                "/v2/intros/add/{}/{}",
                                                guild_id, channel_name
                                            ),
                                        ))
                                });
                        }
                    }

                    b
                })
            })
            .build(),
    ))
}

pub(crate) async fn login(State(state): State<ApiState>) -> Html<String> {
    let authorize_uri = format!("https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}/v2/auth&response_type=code&scope=guilds.members.read%20guilds%20identify", state.secrets.client_id, state.origin);

    Html(
        HtmxBuilder::new(Tag::Html)
            .head(|b| {
                b.title("MemeJoin - Login")
                .script(
                    "https://unpkg.com/htmx.org@1.9.3",
                    Some("sha384-lVb3Rd/Ca0AxaoZg5sACe8FJKF0tnUgR2Kd7ehUOG5GCcROv5uBIZsOqovBAcWua"),
                )
                .script("https://unpkg.com/hyperscript.org@0.9.9", None)
                .style_link("https://cdn.jsdelivr.net/npm/@picocss/pico@1/css/pico.min.css")
            })
            .link("Login", &authorize_uri)
            .build(),
    )
}