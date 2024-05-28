use anyhow::anyhow;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex_macro::regex;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::warn;

use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub struct OtoData {
    pub header: wav_io::header::WavHeader,
    pub samples: Vec<f64>,
    pub frq: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
enum OtoCache {
    Oto(OtoData),
    Error(String),
}

static CACHE: Lazy<RwLock<HashMap<String, OtoCache>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Serialize)]
pub struct Oto {
    pub path: PathBuf,
    pub frq: PathBuf,
    pub alias: String,
    pub offset: f64,
    pub consonant: f64,
    pub cut_off: f64,
    pub preutter: f64,
    pub overlap: f64,
}

impl Oto {
    pub async fn new(line: &str, root: PathBuf) -> Result<Self> {
        let pattern = regex!("(?P<name>.+)=(?P<alias>.*),(?P<offset>.+),(?P<consonant>.+),(?P<cut_off>.+),(?P<preutter>.+),(?P<overlap>.+)");
        let captures = pattern.captures(line);
        let captures = match captures {
            Some(captures) => captures,
            None => {
                warn!("Failed to parse oto line: {}", line);
                return Err(anyhow!("Failed to parse oto line"));
            }
        };
        let frq_name = captures["name"].replace(".wav", "_wav.frq");
        let mut alias = captures["alias"].to_string();
        if alias.is_empty() {
            alias = captures["name"].to_string().replace(".wav", "");
        }
        let path = root.join(&captures["name"]);
        let frq = root.join(frq_name);
        Ok(Self {
            path,
            frq,
            alias,
            offset: captures["offset"].parse()?,
            consonant: captures["consonant"].parse()?,
            cut_off: captures["cut_off"].parse()?,
            preutter: captures["preutter"].parse()?,
            overlap: captures["overlap"].parse()?,
        })
    }

    pub async fn from_oto_ini(oto_ini: &str, root: PathBuf) -> HashMap<String, Self> {
        let mut otos = HashMap::new();
        for line in oto_ini.lines() {
            let oto = Oto::new(line, root.clone()).await;
            if let Ok(oto) = oto {
                otos.insert(oto.alias.clone(), oto);
            } else {
                warn!("Failed to parse oto line: {}", line);
            }
        }

        otos
    }

    pub fn is_vcv(&self) -> bool {
        self.alias.contains(' ') && !self.alias.starts_with("- ")
    }

    pub fn is_cvvc(&self) -> bool {
        return self.is_vcv()
            && ["あ", "い", "う", "え", "お"]
                .iter()
                .any(|x| self.alias.contains(x));
    }

    pub async fn read(&self) -> Result<OtoData> {
        {
            let cache = CACHE.read().await;
            if let Some(data) = cache.get(&self.alias) {
                return match data {
                    OtoCache::Oto(data) => Ok(data.clone()),
                    OtoCache::Error(e) => Err(anyhow!("Failed to read oto: {}", e)),
                };
            }
        }

        let data = self.read_inner().await;
        match &data {
            Ok(data) => {
                let mut cache = CACHE.write().await;
                cache.insert(self.alias.clone(), OtoCache::Oto(data.clone()));
            }
            Err(err) => {
                let mut cache = CACHE.write().await;
                cache.insert(
                    self.alias.clone(),
                    OtoCache::Error(err.to_string().replace('\n', " ")),
                );
            }
        }
        data
    }

    async fn read_inner(&self) -> Result<OtoData> {
        let file = fs_err::tokio::read(&self.path).await?;
        let mut reader = wav_io::reader::Reader::from_vec(file)
            .map_err(|e| anyhow!("Failed to read wav file: {}", e))?;
        let header = reader
            .read_header()
            .map_err(|e| anyhow!("Failed to read wav header: {}", e))?;
        let mut samples = reader
            .get_samples_f32()
            .map_err(|e| anyhow!("Failed to read wav samples: {}", e))?;
        if header.channels != 1 {
            samples = wav_io::utils::stereo_to_mono(samples);
        }

        let frq = match fs_err::tokio::read(&self.frq).await {
            Ok(frq) => Some(frq),
            Err(e) => {
                warn!("Failed to read frq file: {}", e);

                None
            }
        };

        let data = OtoData {
            header,
            samples: samples.into_iter().map(|x| x as f64).collect(),
            frq,
        };

        Ok(data)
    }
}
