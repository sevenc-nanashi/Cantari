use serde::{Deserialize, Serialize};

// https://github.com/VOICEVOX/voicevox_core/blob/main/crates/voicevox_core/src/engine/model.rs
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MoraModel {
    pub text: String,

    pub consonant: Option<String>,

    pub consonant_length: Option<f32>,

    pub vowel: String,

    pub vowel_length: f32,

    pub pitch: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccentPhraseModel {
    pub moras: Vec<MoraModel>,

    pub accent: usize,

    pub pause_mora: Option<MoraModel>,

    #[serde(default)]
    pub is_interrogative: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AudioQueryModel {
    pub accent_phrases: Vec<AccentPhraseModel>,

    pub speed_scale: f32,

    pub pitch_scale: f32,

    pub intonation_scale: f32,

    pub volume_scale: f32,

    pub pre_phoneme_length: f32,

    pub post_phoneme_length: f32,

    pub output_sampling_rate: serde_json::Number,

    pub output_stereo: bool,

    pub kana: Option<String>,
}

impl From<&voicevox_core::AudioQueryModel> for AudioQueryModel {
    fn from(value: &voicevox_core::AudioQueryModel) -> Self {
        let json = serde_json::to_string(value).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}

impl From<&AudioQueryModel> for voicevox_core::AudioQueryModel {
    fn from(value: &AudioQueryModel) -> Self {
        let mut cloned_value = value.clone();
        cloned_value.output_sampling_rate =
            serde_json::Number::from(value.output_sampling_rate.as_f64().unwrap() as u64);
        let json = serde_json::to_string(&cloned_value).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}

impl From<&voicevox_core::AccentPhraseModel> for AccentPhraseModel {
    fn from(value: &voicevox_core::AccentPhraseModel) -> Self {
        let json = serde_json::to_string(value).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}

impl From<&AccentPhraseModel> for voicevox_core::AccentPhraseModel {
    fn from(value: &AccentPhraseModel) -> Self {
        let json = serde_json::to_string(value).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}
