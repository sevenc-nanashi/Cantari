use anyhow::anyhow;
use anyhow::Result;
use regex_macro::regex;
use serde::Serialize;
use std::sync::Arc;
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

#[derive(Debug, Clone, Serialize)]
pub struct Oto {
    pub path: PathBuf,
    pub frq: PathBuf,
    pub names: Vec<String>,
    pub offset: f64,
    pub consonant: f64,
    pub cut_off: f64,
    pub preutter: f64,
    pub overlap: f64,

    #[serde(skip)]
    cache: Arc<RwLock<Option<OtoCache>>>,
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
        let wav_name = captures["name"]
            .strip_suffix(".wav")
            .ok_or_else(|| anyhow!("Failed to strip suffix"))?;
        let alias = captures["alias"].to_string();
        let mut names = vec![wav_name.to_string(), alias.clone()];
        if alias.is_empty() {
            names.remove(1);
        }
        let path = root.join(&captures["name"]);
        let frq = root.join(frq_name);
        Ok(Self {
            path,
            frq,
            names,
            offset: captures["offset"].parse()?,
            consonant: captures["consonant"].parse()?,
            cut_off: captures["cut_off"].parse()?,
            preutter: captures["preutter"].parse()?,
            overlap: captures["overlap"].parse()?,

            cache: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn from_oto_ini(oto_ini: &str, root: PathBuf) -> HashMap<String, Self> {
        let mut otos = HashMap::new();
        for line in oto_ini.lines() {
            let oto = Oto::new(line, root.clone()).await;
            if let Ok(oto) = oto {
                for name in &oto.names {
                    otos.insert(name.clone(), oto.clone());
                }
            } else {
                warn!("Failed to parse oto line: {}", line);
            }
        }

        otos
    }

    pub async fn read(&self) -> Result<OtoData> {
        {
            let cache = self.cache.read().await;
            if cache.is_some() {
                let data = cache.as_ref().unwrap();
                return match data {
                    OtoCache::Oto(data) => Ok(data.clone()),
                    OtoCache::Error(e) => Err(anyhow!("Failed to read oto: {}", e)),
                };
            }
        }

        let data = self.read_inner().await;
        match &data {
            Ok(data) => {
                let mut cache = self.cache.write().await;
                *cache = Some(OtoCache::Oto(data.clone()));
            }
            Err(err) => {
                let mut cache = self.cache.write().await;
                *cache = Some(OtoCache::Error(err.to_string()));
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
