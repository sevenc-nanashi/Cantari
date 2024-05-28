use crate::error::{Error, Result};
use anyhow::anyhow;
use assets::{open_jtalk_dic, sample_vvm};
use std::sync::Arc;
use tokio::sync::OnceCell;

use axum::{extract::Query, Json};
use duplicate::duplicate_item;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use tracing::info;
use voicevox_core::{tokio::OpenJtalk, InitializeOptions};

static SPEED_SCALE: f32 = 1.0;
static INTONATION_SCALE: f32 = 2.0;

pub static SYNTHESIZER: OnceCell<Arc<voicevox_core::tokio::Synthesizer<OpenJtalk>>> =
    OnceCell::const_new();

#[derive(Debug, Deserialize)]
pub struct AudioQueryParams {
    text: String,
    speaker: u32,
}

#[derive(Debug, Deserialize)]
pub struct AccentPhraseModifyParams {
    speaker: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpAudioQuery {
    #[serde(rename = "accent_phrases")]
    pub accent_phrases: Vec<crate::model::AccentPhraseModel>,
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

impl From<&crate::model::AudioQueryModel> for HttpAudioQuery {
    fn from(value: &crate::model::AudioQueryModel) -> Self {
        Self {
            accent_phrases: value.accent_phrases.clone(),
            speed_scale: value.speed_scale,
            pitch_scale: value.pitch_scale,
            intonation_scale: value.intonation_scale,
            volume_scale: value.volume_scale,
            pre_phoneme_length: value.pre_phoneme_length,
            post_phoneme_length: value.post_phoneme_length,
            output_sampling_rate: value.output_sampling_rate.clone(),
            output_stereo: value.output_stereo,
            kana: value.kana.clone().unwrap_or_default(),
        }
    }
}
impl From<&HttpAudioQuery> for crate::model::AudioQueryModel {
    fn from(value: &HttpAudioQuery) -> Self {
        crate::model::AudioQueryModel {
            accent_phrases: value.accent_phrases.clone(),
            speed_scale: value.speed_scale,
            pitch_scale: value.pitch_scale,
            intonation_scale: value.intonation_scale,
            volume_scale: value.volume_scale,
            pre_phoneme_length: value.pre_phoneme_length,
            post_phoneme_length: value.post_phoneme_length,
            output_sampling_rate: value.output_sampling_rate.clone(),
            output_stereo: value.output_stereo,
            kana: Some(value.kana.clone()),
        }
    }
}

pub async fn post_audio_query(
    Query(query): Query<AudioQueryParams>,
) -> Result<Json<HttpAudioQuery>> {
    let synthesizer = get_or_initialize_synthesizer().await;
    let audio_query = synthesizer
        .audio_query(&query.text, voicevox_core::StyleId::new(0))
        .await
        .map_err(|e| Error::InferenceFailed(anyhow!("Failed to create audio query: {}", e)))?;

    let mut audio_query = crate::model::AudioQueryModel::from(&audio_query)
        .apply_speed_scale(SPEED_SCALE)
        .apply_intonation_scale(INTONATION_SCALE);

    audio_query.pre_phoneme_length = 0.1;
    audio_query.post_phoneme_length = 0.1;

    Ok(Json(HttpAudioQuery::from(&audio_query)))
}

pub async fn post_accent_phrases(
    Query(query): Query<AudioQueryParams>,
) -> Result<Json<Vec<crate::model::AccentPhraseModel>>> {
    let synthesizer = get_or_initialize_synthesizer().await;
    let accent_phrases = synthesizer
        .create_accent_phrases(&query.text, voicevox_core::StyleId::new(0))
        .await
        .map_err(|e| Error::InferenceFailed(anyhow!("Failed to create accent phrases: {}", e)))?;

    let audio_query = crate::model::AudioQueryModel {
        accent_phrases: accent_phrases.iter().map(|x| x.into()).collect(),
        speed_scale: 1.0,
        pitch_scale: 0.0,
        intonation_scale: 1.0,
        volume_scale: 1.0,
        pre_phoneme_length: 0.1,
        post_phoneme_length: 0.1,
        output_sampling_rate: Number::from(24000),
        output_stereo: false,
        kana: None,
    };

    let audio_query = audio_query
        .apply_speed_scale(SPEED_SCALE)
        .apply_intonation_scale(INTONATION_SCALE);

    Ok(Json(audio_query.accent_phrases))
}

#[duplicate_item(
    name               synthesizer_method       should_apply_speed_scale should_apply_intonation_scale;
    [post_mora_data ]  [replace_mora_data]      [true]                   [true];
    [post_mora_pitch]  [replace_mora_pitch]     [false]                  [true];
    [post_mora_length] [replace_phoneme_length] [true]                   [false];
)]
pub async fn name(
    Query(query): Query<AccentPhraseModifyParams>,
    Json(accent_phrases): Json<Vec<crate::model::AccentPhraseModel>>,
) -> Result<Json<Vec<crate::model::AccentPhraseModel>>> {
    let synthesizer = get_or_initialize_synthesizer().await;
    let accent_phrases: Vec<voicevox_core::AccentPhraseModel> =
        accent_phrases.iter().map(|x| x.into()).collect();
    let new_accent_phrases = synthesizer
        .synthesizer_method(&accent_phrases, voicevox_core::StyleId::new(0))
        .await
        .map_err(|e| Error::InferenceFailed(anyhow!("Operation failed: {}", e)))?;

    let audio_query = crate::model::AudioQueryModel {
        accent_phrases: new_accent_phrases.iter().map(|x| x.into()).collect(),
        speed_scale: 1.0,
        pitch_scale: 0.0,
        intonation_scale: 1.0,
        volume_scale: 1.0,
        pre_phoneme_length: 0.1,
        post_phoneme_length: 0.1,
        output_sampling_rate: Number::from(24000),
        output_stereo: false,
        kana: None,
    };

    let audio_query = audio_query
        .apply_speed_scale(if should_apply_speed_scale {
            SPEED_SCALE
        } else {
            1.0
        })
        .apply_intonation_scale(if should_apply_intonation_scale {
            INTONATION_SCALE
        } else {
            1.0
        });

    Ok(Json(audio_query.accent_phrases))
}

pub async fn get_is_initialized_speaker() -> Json<bool> {
    Json(SYNTHESIZER.get().is_some())
}
pub async fn post_initialize_speaker() {
    if SYNTHESIZER.get().is_some() {
        return;
    }
    initialize_synthesizer().await;
}

pub async fn get_or_initialize_synthesizer() -> Arc<voicevox_core::tokio::Synthesizer<OpenJtalk>> {
    if let Some(synthesizer) = SYNTHESIZER.get() {
        return synthesizer.clone();
    }
    initialize_synthesizer().await;
    SYNTHESIZER.get().unwrap().clone()
}

pub async fn initialize_synthesizer() {
    info!("Initializing Synthesizer...");

    let synthesizer = voicevox_core::tokio::Synthesizer::new(
        OpenJtalk::new(camino::Utf8PathBuf::from_path_buf(open_jtalk_dic()).unwrap())
            .await
            .expect("Failed to initialize OpenJtalk"),
        &InitializeOptions {
            acceleration_mode: voicevox_core::AccelerationMode::Cpu,
            cpu_num_threads: 1,
        },
    )
    .expect("Failed to initialize Synthesizer");

    let model = voicevox_core::tokio::VoiceModel::from_path(sample_vvm())
        .await
        .expect("Failed to load VoiceModel");

    synthesizer
        .load_voice_model(&model)
        .await
        .expect("Failed to load VoiceModel");

    if let Err(e) = SYNTHESIZER.set(Arc::new(synthesizer)) {
        panic!("Failed to set SYNTHESIZER: {}", e);
    }
    info!("Synthesizer initialized");
}
