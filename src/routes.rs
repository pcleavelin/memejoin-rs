use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::settings::{GuildSettings, Intro, IntroIndex, Settings, UserSettings};

#[derive(Serialize)]
pub(crate) enum IntroResponse<'a> {
    Intros(&'a Vec<Intro>),
    NoGuildFound,
}

#[derive(Serialize)]
pub(crate) enum MeResponse<'a> {
    Me(Me<'a>),
    NoUserFound,
}

#[derive(Serialize)]
pub(crate) struct Me<'a> {
    pub(crate) username: String,
    pub(crate) guilds: Vec<MeGuild<'a>>,
}

#[derive(Serialize)]
pub(crate) struct MeGuild<'a> {
    pub(crate) name: String,
    pub(crate) channels: Vec<MeChannel<'a>>,
}

#[derive(Serialize)]
pub(crate) struct MeChannel<'a> {
    pub(crate) name: String,
    pub(crate) intros: &'a Vec<IntroIndex>,
}

pub(crate) async fn health(State(state): State<Arc<Mutex<Settings>>>) -> Json<Value> {
    let settings = state.lock().await;

    Json(json!(*settings))
}

pub(crate) async fn intros(
    State(state): State<Arc<Mutex<Settings>>>,
    Path(guild): Path<u64>,
) -> Json<Value> {
    let settings = state.lock().await;
    let Some(guild) = settings.guilds.get(&guild) else { return Json(json!(IntroResponse::NoGuildFound)); };

    Json(json!(IntroResponse::Intros(&guild.intros)))
}

pub(crate) async fn me(
    State(state): State<Arc<Mutex<Settings>>>,
    Path(user): Path<String>,
) -> Json<Value> {
    let settings = state.lock().await;

    let mut me = Me {
        username: user.clone(),
        guilds: Vec::new(),
    };

    for g in &settings.guilds {
        let mut guild = MeGuild {
            name: g.0.to_string(),
            channels: Vec::new(),
        };

        for channel in &g.1.channels {
            let user_settings = channel.1.users.iter().find(|u| *u.0 == user);

            let Some(user) = user_settings else { continue; };

            guild.channels.push(MeChannel {
                name: channel.0.to_owned(),
                intros: &user.1.intros,
            });
        }

        me.guilds.push(guild);
    }

    if me.guilds.is_empty() {
        Json(json!(MeResponse::NoUserFound))
    } else {
        Json(json!(MeResponse::Me(me)))
    }
}
