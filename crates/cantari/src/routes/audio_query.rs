use crate::error::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;

use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use voicevox_core::tokio::OpenJtalk;

pub static SYNTHESIZER: OnceCell<Arc<voicevox_core::tokio::Synthesizer<OpenJtalk>>> =
    OnceCell::const_new();

#[derive(Debug, Deserialize)]
pub struct AudioQueryParams {
    text: String,
    speaker: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioQuery {
    #[serde(rename = "accent_phrases")]
    pub accent_phrases: Vec<voicevox_core::AccentPhraseModel>,
    pub speed_scale: f32,
    pub pitch_scale: f32,
    pub intonation_scale: f32,
    pub volume_scale: f32,
    pub pre_phoneme_length: f32,
    pub post_phoneme_length: f32,
    // Recotte Studioはfloatで渡してくるので、serde_json::Numberで受け取る
    pub output_sampling_rate: Number,
    pub output_stereo: bool,
    pub kana: String,
}

pub async fn post_audio_query(Query(query): Query<AudioQueryParams>) -> Result<Json<AudioQuery>> {
    todo!();
}

pub async fn post_accent_phrases(Query(query): Query<AudioQueryParams>) -> Result<Json<Vec<i32>>> {
    todo!();
}
