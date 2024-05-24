use super::audio_query::AudioQuery;
use crate::error::Result;

use axum::{extract::Query, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AudioQueryQuery {
    pub speaker: u32,
}

pub async fn post_synthesis(
    Query(query): Query<AudioQueryQuery>,
    Json(audio_query): Json<AudioQuery>,
) -> Result<Vec<u8>> {
    todo!()
}
