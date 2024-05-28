use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

fn get_path() -> PathBuf {
    let name = if cfg!(feature = "release") {
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct V1Settings {
    pub paths: Vec<String>,
}

async fn load_settings_inner() -> Result<Settings> {
    let path = get_path();

    let settings = tokio::fs::read_to_string(path).await?;

    let settings = serde_json::from_str(&settings)?;

    Ok(settings)
}

pub async fn load_settings() -> V1Settings {
    let path = get_path();

    let settings = load_settings_inner().await;

    let mut settings = settings.unwrap_or_else(|e| {
        error!("Failed to load settings from {}: {}", path.display(), e);
        error!("Using default settings");

        Settings::V1(V1Settings {
            paths: vec![
                "/mnt/e/utau/つくよみちゃんUTAU音源".to_string(),
                "/mnt/e/utau/足立レイver3.1.2(単独音)".to_string(),
                "/mnt/e/utau".to_string(),
                "/mnt/e/OpenUtau/Singers".to_string(),
                "/mnt/e/OpenUtau-dev/OpenUtau/bin/Debug/net6.0-windows/Singers".to_string(),
                "/mnt/e/OpenUtau-dev/OpenUtau/bin/Release/net6.0-windows/Singers".to_string(),
            ],
        })
    });

    // Migration will be added here

    #[allow(irrefutable_let_patterns)]
    if let Settings::V1(settings) = &mut settings {
        settings.clone()
    } else {
        unreachable!()
    }
}

pub async fn write_settings(settings: Settings) {
    let path = get_path();

    let settings = serde_json::to_string_pretty(&settings).unwrap();

    tokio::fs::create_dir_all(path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(path, settings).await.unwrap();
}
