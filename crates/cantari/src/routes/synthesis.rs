use super::audio_query::HttpAudioQuery;
use crate::{error::Result, model::AudioQueryModel};

use axum::{extract::Query, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AudioQueryQuery {
    pub speaker: u32,
}

pub async fn post_synthesis(
    Query(query): Query<AudioQueryQuery>,
    Json(audio_query): Json<HttpAudioQuery>,
) -> Result<Vec<u8>> {
    let audio_query: AudioQueryModel = (&audio_query).into();
    let synthesizer = worldline::PhraseSynth::new();

    todo!()
}
