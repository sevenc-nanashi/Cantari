use crate::error::{Error, ErrorResponse, Result};

use axum::{http::StatusCode, response::IntoResponse, Json, extract::Query};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VvSpeaker {
    pub name: String,
    pub speaker_uuid: String,
    pub styles: Vec<VvStyle>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VvSpeakerInfo {
    pub policy: String,
    pub portrait: String,
    pub style_infos: Vec<VvStyleInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VvStyleInfo {
    pub id: u32,
    pub icon: String,
    pub portrait: String,
    pub voice_samples: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportedFeatures {
    pub permitted_synthesis_morphing: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VvStyle {
    pub name: String,
    pub id: u32,
    pub r#type: String,
}

pub async fn get_speakers() -> Result<Json<Vec<VvSpeaker>>> {
    todo!()
}

#[derive(Debug, Deserialize)]
pub struct SpeakerInfoQuery {
    pub speaker_uuid: String,
}

pub async fn get_speaker_info(Query(query): axum::extract::Query<SpeakerInfoQuery>) -> Result<Json<VvSpeakerInfo>> {
    todo!()
}

pub async fn get_is_initialized_speaker() -> Json<bool> {
    Json(true)
}
