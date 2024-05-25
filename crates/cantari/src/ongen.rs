use anyhow::anyhow;
use anyhow::Result;
use image::io::Reader as ImageReader;
use once_cell::sync::OnceCell;
use regex_macro::regex;
use std::io::Cursor;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::settings::load_settings;

pub static ONGEN: OnceCell<Arc<RwLock<HashMap<Uuid, Ongen>>>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct Ongen {
    pub uuid: Uuid,
    pub root: PathBuf,
    pub info: HashMap<String, String>,
}

impl Ongen {
    pub async fn new(root: PathBuf) -> Result<Self> {
        let character_txt = root.join("character.txt");
        let character = tokio::fs::read(character_txt).await.unwrap();
        let character = encoding_rs::SHIFT_JIS.decode(&character).0;

        let mut info = HashMap::new();
        let character_pattern = regex!(r"(?P<key>[^:]+)[=：](?P<value>.+)");
        for line in character.lines() {
            if let Some(captures) = character_pattern.captures(line) {
                let key = captures.name("key").unwrap().as_str().to_string();
                let value = captures.name("value").unwrap().as_str().to_string();
                info.insert(key, value);
            }
        }

        let name = info.get("name").ok_or_else(|| anyhow!("name not found"))?;
        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("ongen:{}", name).as_bytes());

        Ok(Self { uuid, root, info })
    }

    pub fn name(&self) -> String {
        self.info.get("name").unwrap().clone()
    }

    pub fn id(&self) -> u32 {
        let uuid_string = self.uuid.to_string();
        let uuid_first_section = uuid_string.split('-').next().unwrap();
        u32::from_str_radix(uuid_first_section, 16).unwrap() & !(0xffu32)
    }

    pub async fn read_image(&self) -> Option<Vec<u8>> {
        let image_path = self.info.get("image")?.replace('\\', "/");
        let path = self.root.join(image_path.trim_start_matches('/'));
        let image_loaded = ImageReader::open(path).unwrap().decode().unwrap();
        let mut image = Vec::new();
        image_loaded
            .write_to(&mut Cursor::new(&mut image), image::ImageFormat::Png)
            .unwrap();

        Some(image)
    }
}

pub async fn setup_ongen() {
    info!("Setting up ongens...");
    let paths = load_settings().await.paths;
    let mut ongens = HashMap::new();
    for path in paths {
        match Ongen::new(PathBuf::from(&path)).await {
            Ok(ongen) => {
                info!(
                    "Loaded ongen: {} ({}, {})",
                    ongen.name(),
                    ongen.uuid,
                    ongen.id()
                );
                ongens.insert(ongen.uuid, ongen);
            }
            Err(e) => {
                warn!("Failed to load ongen at {}: {}", path, e);
            }
        }
    }

    ONGEN.get_or_init(|| Arc::new(RwLock::new(ongens)));
}
