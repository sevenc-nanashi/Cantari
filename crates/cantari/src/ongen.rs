use crate::ongen_settings::StyleSettings;
use crate::write_settings;
use anyhow::{anyhow, bail, Result};
use educe::Educe;
use image::io::Reader as ImageReader;
use once_cell::sync::OnceCell;
use regex_macro::regex;
use serde::Serialize;
use std::io::Cursor;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, info_span, instrument, warn};
use uuid::Uuid;

use crate::oto::Oto;
use crate::settings::load_settings;

pub static ONGEN: OnceCell<Arc<RwLock<HashMap<Uuid, Ongen>>>> = OnceCell::new();

#[derive(Educe, Clone, Serialize)]
#[educe(Debug)]
pub struct Ongen {
    pub uuid: Uuid,
    pub root: PathBuf,
    pub info: HashMap<String, String>,
    #[educe(Debug(ignore))]
    pub prefix_suffix_map: HashMap<String, (String, String)>,
    #[educe(Debug(ignore))]
    pub oto: Arc<HashMap<String, Oto>>,
}

impl Ongen {
    #[instrument(skip(existing_uuids))]
    pub async fn new(root: PathBuf, existing_uuids: &[&Uuid]) -> Result<Self> {
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
        if existing_uuids.contains(&&uuid) {
            bail!("Duplicate UUID: {}", uuid);
        }

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
            let span = info_span!("oto.ini", path = %entry.path().display());
            let _guard = span.enter();

            info!("Found oto.ini");
            let oto_ini_file = tokio::fs::read(entry.path()).await?;
            let oto_ini = encoding_rs::SHIFT_JIS.decode(&oto_ini_file).0;
            let oto =
                Oto::from_oto_ini(&oto_ini, entry.path().parent().unwrap().to_path_buf()).await;
            if oto.is_empty() {
                warn!("No oto found");
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
            oto: Arc::new(all_oto),
        })
    }

    pub fn name(&self) -> String {
        self.info.get("name").unwrap().clone()
    }

    pub fn id(&self) -> u32 {
        let uuid_string = self.uuid.to_string();
        let uuid_first_section = uuid_string.split('-').next().unwrap();
        (u32::from_str_radix(uuid_first_section, 16).unwrap() >> 1) & !(0xffu32)
    }

    pub async fn read_image(&self) -> Option<Vec<u8>> {
        let image_path = self.info.get("image")?.replace('\\', "/");
        let path = self.root.join(image_path.trim_start_matches('/'));
        let image = ImageReader::open(path).unwrap().decode().unwrap();
        // 256x256にリサイズされてるので合わせる
        // https://github.com/VOICEVOX/voicevox_resource/blob/main/scripts/resize.sh
        let image = image.resize(256, 256, image::imageops::FilterType::Lanczos3);
        let mut image_buffer = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut image_buffer), image::ImageFormat::Png)
            .unwrap();

        Some(image_buffer)
    }
}

#[instrument]
pub async fn setup_ongen() {
    info!("Setting up ongens...");
    let mut settings = load_settings().await;

    let mut roots = vec![];
    for path in &settings.paths {
        for file in walkdir::WalkDir::new(path)
            .min_depth(1)
            .max_depth(3)
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
        match Ongen::new(
            PathBuf::from(&path),
            ongens.keys().collect::<Vec<&Uuid>>().as_slice(),
        )
        .await
        {
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

        if ongens.len() >= settings.ongen_limit {
            info!("Reached ongen limit of {}", settings.ongen_limit);
            break;
        }
    }
    info!("Loaded {} ongens", ongens.len());

    for (uuid, ongen) in &ongens {
        if !settings.ongen_settings.contains_key(uuid) {
            info!("Adding default settings for {}", ongen.name());
            settings.ongen_settings.insert(*uuid, Default::default());
        }
    }

    write_settings(&settings).await;

    if ONGEN.get().is_some() {
        let mut ongen_lock = ONGEN.get().unwrap().write().await;
        *ongen_lock = ongens;
    } else {
        ONGEN.set(Arc::new(RwLock::new(ongens))).unwrap();
    }
}

pub async fn get_ongen_style_from_id<'a, 'b>(
    ongens: &'a HashMap<Uuid, Ongen>,
    settings: &'b crate::settings::Settings,
    ongen_id: u32,
) -> Option<(&'a Ongen, &'b StyleSettings)> {
    let ongen = ongens
        .values()
        .find(|ongen| ongen.id() == ongen_id & !(0xff))?;
    let ongen_settings = settings.ongen_settings.get(&ongen.uuid)?;

    let style_index = ongen_id & 0xff;
    let style_settings = ongen_settings.style_settings.get(style_index as usize)?;

    Some((ongen, style_settings))
}
