use crate::ongen_settings::OngenSettings;
use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

static SETTINGS: OnceCell<Mutex<Settings>> = OnceCell::new();

pub fn get_settings_path() -> PathBuf {
    let name = if cfg!(not(debug_assertions)) {
        "cantari.json"
    } else {
        "cantari-dev.json"
    };

    let home = dirs::home_dir().unwrap();

    home.join(".config").join(name)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub format_version: u8,
    pub paths: Vec<String>,
    pub ongen_limit: usize,
    pub ongen_settings: HashMap<Uuid, OngenSettings>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format_version: 1,
            paths: vec![],
            ongen_limit: 10,
            ongen_settings: HashMap::new(),
        }
    }
}

async fn load_settings_inner() -> Result<Settings> {
    let path = get_settings_path();

    let settings = tokio::fs::read_to_string(path).await?;

    let settings = serde_json::from_str(&settings)?;

    Ok(settings)
}

pub async fn load_settings() -> Settings {
    if let Some(settings) = SETTINGS.get() {
        let settings = settings.lock().await;
        return settings.clone();
    }
    info!("Loading settings...");
    let path = get_settings_path();

    let settings = load_settings_inner().await;

    settings.unwrap_or_else(|e| {
        error!("Failed to load settings from {}: {}", path.display(), e);
        error!("Using default settings");

        let paths = if cfg!(target_os = "windows") {
            let appdata = PathBuf::from(std::env::var("APPDATA").unwrap());
            let utau_voicebank = appdata.join("Utau").join("voice");

            vec![utau_voicebank.to_string_lossy().to_string()]
        } else {
            vec![]
        };

        Settings {
            paths,
            ..Default::default()
        }
    })
}

pub async fn write_settings(new_settings: &Settings) {
    let path = get_settings_path();

    match SETTINGS.get() {
        Some(settings) => {
            info!("Updating settings...");
            let mut settings = settings.lock().await;
            *settings = new_settings.clone();
        }
        None => {
            info!("Initializing settings...");
            SETTINGS.set(Mutex::new(new_settings.clone())).unwrap();
        }
    }

    let settings = serde_json::to_string_pretty(&new_settings).unwrap();

    tokio::fs::create_dir_all(path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(path, settings).await.unwrap();
}
