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

impl AccentPhraseModel {
    pub fn apply_speed_scale(&self, speed_scale: f32) -> AccentPhraseModel {
        let mut modified = self.clone();
        for mora in &mut modified.moras {
            mora.vowel_length /= speed_scale;
            if let Some(consonant_length) = &mut mora.consonant_length {
                *consonant_length /= speed_scale;
            }
        }
        if let Some(pause_mora) = &mut modified.pause_mora {
            pause_mora.vowel_length /= speed_scale;
            if let Some(consonant_length) = &mut pause_mora.consonant_length {
                *consonant_length /= speed_scale;
            }
        }

        modified
    }
    pub fn apply_pitch_scale(&self, pitch_scale: f32) -> AccentPhraseModel {
        let pitch_scale = 2.0_f32.powf(pitch_scale);
        let mut modified = self.clone();
        for mora in &mut modified.moras {
            if mora.pitch != 0.0 {
                mora.pitch *= pitch_scale;
            }
        }

        modified
    }
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
impl AudioQueryModel {
    pub fn apply_speed_scale(&self, speed_scale: f32) -> AudioQueryModel {
        let mut modified = self.clone();
        modified.accent_phrases = self
            .accent_phrases
            .iter()
            .map(|accent_phrase| accent_phrase.apply_speed_scale(speed_scale))
            .collect();
        modified.pre_phoneme_length /= speed_scale;
        modified.post_phoneme_length /= speed_scale;

        modified.speed_scale = 1.0;

        modified
    }
    pub fn apply_pitch_scale(&self, pitch_scale: f32) -> AudioQueryModel {
        let mut modified = self.clone();
        modified.accent_phrases = self
            .accent_phrases
            .iter()
            .map(|accent_phrase| accent_phrase.apply_pitch_scale(pitch_scale))
            .collect();

        modified.pitch_scale = 0.0;

        modified
    }
    pub fn apply_intonation_scale(&self, intonation_scale: f32) -> AudioQueryModel {
        let mut modified = self.clone();
        let mut pitches = vec![];
        for accent_phrase in &mut modified.accent_phrases {
            for mora in &accent_phrase.moras {
                pitches.push(mora.pitch);
            }
        }
        let sum = pitches.iter().sum::<f32>();
        let average = sum / pitches.iter().filter(|&&pitch| pitch > 0.0).count() as f32;

        for accent_phrase in &mut modified.accent_phrases {
            for mora in &mut accent_phrase.moras {
                if mora.pitch != 0.0 {
                    mora.pitch += (mora.pitch - average) * (intonation_scale - 1.0);
                }
            }
        }

        modified.intonation_scale = 1.0;

        modified
    }
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
