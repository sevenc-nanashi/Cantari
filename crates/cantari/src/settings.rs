use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

pub fn get_settings_path() -> PathBuf {
    let name = if cfg!(not(debug_assertions)) {
        "cantari.json"
    } else {
        "cantari-dev.json"
    };

    let home = dirs::home_dir().unwrap();

    home.join(".config").join(name)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "version")]
pub enum Settings {
    #[serde(rename = "1")]
    V1(V1Settings),
    #[serde(rename = "2")]
    V2(V2Settings),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct V1Settings {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct V2Settings {
    pub paths: Vec<String>,
    pub limit: usize,
}

pub type LatestSettings = V2Settings;

async fn load_settings_inner() -> Result<Settings> {
    let path = get_settings_path();

    let settings = tokio::fs::read_to_string(path).await?;

    let settings = serde_json::from_str(&settings)?;

    Ok(settings)
}

pub async fn load_settings() -> LatestSettings {
    let path = get_settings_path();

    let settings = load_settings_inner().await;

    let settings = settings.unwrap_or_else(|e| {
        error!("Failed to load settings from {}: {}", path.display(), e);
        error!("Using default settings");

        let paths = if cfg!(target_os = "windows") {
            let appdata = PathBuf::from(std::env::var("APPDATA").unwrap());
            let utau_voicebank = appdata.join("Utau").join("voice");

            vec![utau_voicebank.to_string_lossy().to_string()]
        } else {
            vec![]
        };

        Settings::V2(V2Settings { paths, limit: 10 })
    });

    let settings = match settings {
        Settings::V1(v1) => Settings::V2(V2Settings {
            paths: v1.paths,
            limit: 10,
        }),
        other => other,
    };

    #[allow(irrefutable_let_patterns)]
    if let Settings::V2(settings) = &settings {
        settings.clone()
    } else {
        unreachable!()
    }
}

pub async fn write_settings(settings: LatestSettings) {
    let path = get_settings_path();

    let settings = serde_json::to_string_pretty(&Settings::V2(settings)).unwrap();

    tokio::fs::create_dir_all(path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(path, settings).await.unwrap();
}
