use anyhow::{anyhow, bail, Result};
use image::io::Reader as ImageReader;
use once_cell::sync::OnceCell;
use regex_macro::regex;
use serde::Serialize;
use std::io::Cursor;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::oto::Oto;
use crate::settings::load_settings;

pub static ONGEN: OnceCell<Arc<RwLock<HashMap<Uuid, Ongen>>>> = OnceCell::new();

#[derive(Debug, Clone, Serialize)]
pub struct Ongen {
    pub uuid: Uuid,
    pub root: PathBuf,
    pub info: HashMap<String, String>,
    pub prefix_suffix_map: HashMap<String, (String, String)>,
    pub oto: HashMap<String, Oto>,
}

impl Ongen {
    #[instrument]
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

        let mut all_oto = HashMap::new();

        for entry in walkdir::WalkDir::new(&root)
            .min_depth(1)
            .max_depth(3)
            .into_iter()
            .flatten()
        {
            if entry.file_name() != "oto.ini" {
                continue;
            }
            info!("Found oto.ini at {}", entry.path().display());
            let oto_ini_file = tokio::fs::read(entry.path()).await?;
            let oto_ini = encoding_rs::SHIFT_JIS.decode(&oto_ini_file).0;
            let oto =
                Oto::from_oto_ini(&oto_ini, entry.path().parent().unwrap().to_path_buf()).await;
            if oto.is_empty() {
                warn!("No oto found in oto.ini at {}", entry.path().display());
                continue;
            }
            info!("Loaded {} oto entries", oto.len());
            all_oto.extend(oto);
        }

        if all_oto.is_empty() {
            bail!("No oto.ini found for {}", name);
        }
        info!("Loaded {} oto entries", all_oto.len());

        let prefix_suffix_map = if tokio::fs::metadata(root.join("prefix.map")).await.is_ok() {
            info!("Found prefix.map for {}", name);
            let prefix_map = tokio::fs::read_to_string(root.join("prefix.map")).await?;
            let mut map = HashMap::new();
            for line in prefix_map.lines() {
                let mut split = line.split('\t');
                let key = split.next().unwrap().to_string();
                let prefix = split.next().unwrap().to_string();
                let postfix = split.next().unwrap().to_string();

                map.insert(key, (prefix, postfix));
            }
            map
        } else {
            info!("No prefix.map found for {}", name);
            HashMap::new()
        };

        Ok(Self {
            uuid,
            root,
            info,
            prefix_suffix_map,
            oto: all_oto,
        })
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

    let mut roots = vec![];
    for path in &paths {
        for file in walkdir::WalkDir::new(path)
            .min_depth(1)
            .max_depth(2)
            .into_iter()
            .flatten()
        {
            if file.file_type().is_file() && file.file_name() == "character.txt" {
                roots.push(file.path().parent().unwrap().to_path_buf());
            }
        }
    }

    let mut ongens = HashMap::new();
    for path in roots {
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
                warn!("Failed to load ongen at {:?}: {}", path, e);
            }
        }
    }

    ONGEN.get_or_init(|| Arc::new(RwLock::new(ongens)));
}
