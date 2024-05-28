use crate::error::{Error, Result};
use crate::ongen::ONGEN;

use axum::{extract::Query, Json};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    let ongens = ONGEN.get().unwrap().read().await;

    let mut speakers = Vec::new();

    for speaker in ongens.values() {
        let speaker = VvSpeaker {
            name: speaker.name().clone(),
            speaker_uuid: speaker.uuid.to_string(),
            styles: vec![VvStyle {
                name: "ノーマル".to_string(),
                id: speaker.id(),
                r#type: "talk".to_string(),
            }],
            version: "N/A".to_string(),
        };

        speakers.push(speaker);
    }

    Ok(Json(speakers))
}

#[derive(Debug, Deserialize)]
pub struct SpeakerInfoQuery {
    pub speaker_uuid: Uuid,
}

pub async fn get_speaker_info(
    Query(query): axum::extract::Query<SpeakerInfoQuery>,
) -> Result<Json<VvSpeakerInfo>> {
    let ongens = ONGEN.get().unwrap().read().await;

    let speaker = ongens
        .get(&query.speaker_uuid)
        .ok_or_else(|| Error::CharacterNotFound)?;

    let image = speaker
        .read_image()
        .await
        .unwrap_or_else(|| include_bytes!("../icon.png").to_vec());

    let info = VvSpeakerInfo {
        policy: "N/A".to_string(),
        portrait: base64::engine::general_purpose::STANDARD.encode(&image),
        style_infos: vec![VvStyleInfo {
            id: speaker.id(),
            icon: base64::engine::general_purpose::STANDARD.encode(&image),
            portrait: base64::engine::general_purpose::STANDARD.encode(&image),
            voice_samples: vec![],
        }],
    };

    Ok(Json(info))
}
