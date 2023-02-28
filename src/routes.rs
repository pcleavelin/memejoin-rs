use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::settings::{Intro, Settings, UserSettings};

#[derive(Serialize)]
pub(crate) enum MeResponse<'a> {
    Settings(Vec<&'a UserSettings>),
    NoUserFound,
}

#[derive(Serialize)]
pub(crate) enum IntroResponse<'a> {
    Intros(&'a Vec<Intro>),
    NoGuildFound,
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

    let user_settings = settings
        .guilds
        .values()
        .flat_map(|guild| guild.channels.values().flat_map(|channel| &channel.users))
        .filter(|(name, _)| **name == user)
        .map(|(_, settings)| settings)
        .collect::<Vec<_>>();

    if user_settings.is_empty() {
        Json(json!(MeResponse::NoUserFound))
    } else {
        Json(json!(MeResponse::Settings(user_settings)))
    }
}
