use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OngenSettings {
    pub name: Option<String>,

    pub style_settings: Vec<StyleSettings>,
}

impl Default for OngenSettings {
    fn default() -> Self {
        Self {
            name: None,
            style_settings: vec![StyleSettings::default()],
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StyleSettings {
    pub name: String,
    #[serde_as(as = "Option<Base64>")]
    pub portrait: Option<Vec<u8>>,
    #[serde_as(as = "Option<Base64>")]
    pub icon: Option<Vec<u8>>,

    pub key_shift: i8,
    pub whisper: bool,
    pub formant_shift: i8,
    pub breathiness: u8,
    pub tension: i8,
    pub peak_compression: u8,
    pub voicing: u8,
}

impl Default for StyleSettings {
    fn default() -> Self {
        Self {
            name: "ノーマル".to_string(),
            portrait: None,
            icon: None,
            key_shift: 0,
            whisper: false,
            formant_shift: 0,
            breathiness: 0,
            tension: 0,
            peak_compression: 86,
            voicing: 100,
        }
    }
}
