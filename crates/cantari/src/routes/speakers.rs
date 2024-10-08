use crate::error::{Error, Result};
use crate::ongen::ONGEN;
use crate::settings::load_settings;

use axum::{
    extract::{Host, Path, Query},
    Json,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tracing::warn;
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
    let settings = load_settings().await;

    let mut speakers = Vec::new();

    for speaker in ongens.values() {
        let ongen_settings = settings
            .ongen_settings
            .get(&speaker.uuid)
            .ok_or_else(|| Error::CharacterNotFound)?;
        let speaker = VvSpeaker {
            name: ongen_settings
                .name
                .clone()
                .unwrap_or_else(|| speaker.name()),
            speaker_uuid: speaker.uuid.to_string(),
            styles: ongen_settings
                .style_settings
                .iter()
                .enumerate()
                .map(|(i, style_settings)| VvStyle {
                    name: style_settings.name.clone(),
                    id: speaker.id() + i as u32,
                    r#type: "talk".to_string(),
                })
                .collect(),
            version: "N/A".to_string(),
        };

        speakers.push(speaker);
    }

    Ok(Json(speakers))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceFormat {
    #[default]
    Base64,
    Url,
}

#[derive(Debug, Deserialize)]
pub struct SpeakerInfoQuery {
    pub speaker_uuid: Uuid,
    #[serde(default)]
    pub resource_format: ResourceFormat,
}

pub async fn get_speaker_info(
    Query(query): axum::extract::Query<SpeakerInfoQuery>,
    Host(host): Host,
) -> Result<Json<VvSpeakerInfo>> {
    let ongens = ONGEN.get().unwrap().read().await;
    let settings = load_settings().await;

    let root = format!("http://{}", host);

    let speaker = ongens
        .get(&query.speaker_uuid)
        .ok_or_else(|| Error::CharacterNotFound)?;

    let ongen_settings = settings
        .ongen_settings
        .get(&query.speaker_uuid)
        .ok_or_else(|| Error::CharacterNotFound)?;
    let default_image_data = match query.resource_format {
        ResourceFormat::Base64 => {
            let default_image = match ongen_settings.style_settings[0].icon {
                Some(ref icon) => icon.clone(),
                None => speaker
                    .read_image()
                    .await
                    .unwrap_or_else(|| include_bytes!("../unknown_icon.png").to_vec()),
            };
            base64::engine::general_purpose::STANDARD.encode(&default_image)
        }
        ResourceFormat::Url => {
            format!("{}/speaker_resources/icons/{}/0", &root, speaker.uuid)
        }
    };

    let default_portrait_data = match query.resource_format {
        ResourceFormat::Base64 => {
            let default_portrait = match ongen_settings.style_settings[0].portrait {
                Some(ref portrait) => portrait.clone(),
                None => speaker
                    .read_image()
                    .await
                    .unwrap_or_else(|| include_bytes!("../unknown_portrait.png").to_vec()),
            };
            base64::engine::general_purpose::STANDARD.encode(&default_portrait)
        }
        ResourceFormat::Url => {
            format!("{}/speaker_resources/portraits/{}/0", &root, speaker.uuid)
        }
    };

    let mut style_infos = vec![];

    for (i, style_settings) in ongen_settings.style_settings.iter().enumerate() {
        if i >= 256 {
            warn!("Too many styles for speaker {}", speaker.name());
            break;
        }
        let style_info = VvStyleInfo {
            id: speaker.id() + i as u32,

            icon: match query.resource_format {
                ResourceFormat::Base64 => style_settings.icon.as_ref().map_or_else(
                    || default_image_data.clone(),
                    |icon| base64::engine::general_purpose::STANDARD.encode(icon),
                ),
                ResourceFormat::Url => {
                    format!("{}/speaker_resources/icons/{}/{}", &root, speaker.uuid, i)
                }
            },
            portrait: match query.resource_format {
                ResourceFormat::Base64 => style_settings.portrait.as_ref().map_or_else(
                    || default_portrait_data.clone(),
                    |portrait| base64::engine::general_purpose::STANDARD.encode(portrait),
                ),
                ResourceFormat::Url => {
                    format!(
                        "{}/speaker_resources/portraits/{}/{}",
                        &root, speaker.uuid, i
                    )
                }
            },
            voice_samples: vec![],
        };

        style_infos.push(style_info);
    }

    let info = VvSpeakerInfo {
        policy: "元の音源のライセンスに従ってください。".to_string(),
        portrait: default_portrait_data,
        style_infos,
    };

    Ok(Json(info))
}

pub async fn get_icon(Path((uuid, i)): Path<(Uuid, u32)>) -> Result<Vec<u8>> {
    let ongens = ONGEN.get().unwrap().read().await;
    let speaker = ongens.get(&uuid).ok_or_else(|| Error::CharacterNotFound)?;
    let settings = load_settings().await;
    let ongen_settings = settings
        .ongen_settings
        .get(&uuid)
        .ok_or_else(|| Error::CharacterNotFound)?;
    let style_settings = ongen_settings
        .style_settings
        .get(i as usize)
        .ok_or_else(|| Error::CharacterNotFound)?;
    let icon = match style_settings.icon {
        Some(ref icon) => icon.clone(),
        None => speaker
            .read_image()
            .await
            .unwrap_or_else(|| include_bytes!("../unknown_icon.png").to_vec()),
    };

    Ok(icon)
}

pub async fn get_portrait(Path((uuid, i)): Path<(Uuid, u32)>) -> Result<Vec<u8>> {
    let ongens = ONGEN.get().unwrap().read().await;
    let speaker = ongens.get(&uuid).ok_or_else(|| Error::CharacterNotFound)?;
    let settings = load_settings().await;
    let ongen_settings = settings
        .ongen_settings
        .get(&uuid)
        .ok_or_else(|| Error::CharacterNotFound)?;
    let style_settings = ongen_settings
        .style_settings
        .get(i as usize)
        .ok_or_else(|| Error::CharacterNotFound)?;
    let portrait = match style_settings.portrait {
        Some(ref portrait) => portrait.clone(),
        None => speaker
            .read_image()
            .await
            .unwrap_or_else(|| include_bytes!("../unknown_portrait.png").to_vec()),
    };

    Ok(portrait)
}
