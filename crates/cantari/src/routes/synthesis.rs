use super::audio_query::HttpAudioQuery;
use crate::{
    error::{Error, Result},
    math::{smooth, MidiNote},
    model::{AccentPhraseModel, AudioQueryModel},
    ongen::ONGEN,
    oto::{Oto, OtoData},
    tempdir::TEMPDIR,
};
use anyhow::anyhow;
use async_recursion::async_recursion;
use axum::{extract::Query, Json};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use tracing::{info, warn, info_span};
use wav_io::header::WavHeader;
use worldline::{SynthRequest, MS_PER_FRAME};

static CACHES: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));

#[derive(Debug, Deserialize)]
pub struct AudioQueryQuery {
    pub speaker: u32,
}

#[derive(Debug)]
struct PhraseWaves {
    pub data: Vec<f32>,
    pub start_seconds: f64,
}

#[derive(Debug, Serialize)]
struct PhraseSource<'a> {
    prev_freq: f32,
    accent_phrase: &'a AccentPhraseModel,
    ongen: &'a crate::ongen::Ongen,
    speaker: u32,
    volume_scale: f32,
    vcv_connect: f64,
}

impl PhraseSource<'_> {
    fn hash(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        xxhash_rust::xxh3::xxh3_64(json.as_bytes()).to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SynthesisResult {
    wav: Vec<f32>,
    sum_length: f64,
    last_freq: f32,
    last_vowel: String,
}

#[async_recursion]
async fn get_oto<'a>(
    oto: &'a HashMap<String, Oto>,
    kana: &str,
    prefix: &str,
    suffix: &str,
    prev_vowel: &str,
) -> Option<(&'a Oto, OtoData)> {
    let aliases = [
        // // 連続音（音質が安定しないので無効化）
        // format!("{}{} {}{}", prefix, prev_vowel, kana, suffix),
        // 単独音2
        format!("{}{}{}", prefix, kana, suffix),
        // 単独音
        format!("{}- {}{}", prefix, kana, suffix),
    ];

    for alias in aliases.iter() {
        if let Some(oto) = oto.get(alias) {
            match oto.read().await {
                Ok(oto_data) => return Some((oto, oto_data.clone())),
                Err(e) => warn!("Failed to read oto data for {:?}: {:?}", oto.alias, e),
            }
        }
    }

    // 「お」を「を」と登録してる場合があるので、それを考慮
    if kana == "お" {
        return get_oto(oto, "を", prefix, suffix, prev_vowel).await;
    }

    None
}

fn text_to_oto(text: &str) -> String {
    if text == "、" {
        "R".to_string()
    } else {
        kana::kata2hira(text)
    }
}

async fn synthesis_phrase(source: &PhraseSource<'_>) -> SynthesisResult {
    let default_prefix_suffix = ("".to_string(), "".to_string());
    let mut synthesizer = worldline::PhraseSynth::new();
    let mut prev_freq = source.prev_freq;

    let mut prev_vowel = "-".to_string();
    let mut sum_length = 0.0;

    let mut f0 = Vec::new();

    let mut aliases = Vec::new();

    let mut moras = source.accent_phrase.moras.clone();
    if let Some(pause_mora) = source.accent_phrase.pause_mora.clone() {
        moras.push(pause_mora);
    }

    for (i, mora) in moras.iter().enumerate() {
        let span = info_span!("mora", text = mora.text.clone());
        let _guard = span.enter();
        let freq = if mora.pitch == 0.0 {
            // 無声化は前の音高を引き継ぐ
            prev_freq
        } else {
            mora.pitch.exp()
        };
        let kana = text_to_oto(&mora.text);
        let freq_midi = MidiNote::from_frequency(freq);
        let length = ((mora.consonant_length.unwrap_or(0.0) + mora.vowel_length) as f64).max(0.035);
        let (prefix, suffix) = source
            .ongen
            .prefix_suffix_map
            .get(freq_midi.to_string().as_str())
            .unwrap_or(&default_prefix_suffix);
        let oto: Option<(&Oto, OtoData)> =
            get_oto(&source.ongen.oto, &kana, prefix, suffix, &prev_vowel).await;

        let start = sum_length;
        sum_length += length;
        let Some((oto, oto_data)) = oto else {
            if kana == "R" {
                info!("This ongen does not have R");
                sum_length -= length;
            } else if kana == "っ" {
                info!("No oto found for っ");

                continue;
            } else {
                warn!(
                    "No oto found for {:?} {:?} {:?} {:?}",
                    prefix, prev_vowel, kana, suffix
                );
            }
            continue;
        };

        let (next_oto, next_mora) = if i < moras.len() - 1 {
            let next_mora = &moras[i + 1];
            (
                get_oto(
                    &source.ongen.oto,
                    &text_to_oto(&next_mora.text),
                    prefix,
                    suffix,
                    &mora.vowel.to_lowercase(),
                )
                .await,
                Some(next_mora),
            )
        } else {
            (None, None)
        };
        let start = start - (oto.overlap) / 1000.0;
        let skip = if start < 0.0 { -start } else { 0.0 };
        let start = if start < 0.0 { 0.0 } else { start * 1000.0 };
        dbg!(start, skip);
        aliases.push(oto.alias.clone());

        let adjusted_length = length * 1000.0 + 35.0;

        let con_vel = if let Some(consonant_length) = mora.consonant_length {
            let oto_consonant_length = (oto.consonant - oto.preutter) / 1000.0;
            let con_vel = oto_consonant_length / (consonant_length as f64);
            100.0 * con_vel.clamp(0.75, 1.25)
        } else {
            100.0
        };

        dbg!(con_vel);

        let request = SynthRequest {
            sample_fs: oto_data.header.sample_rate as i32,
            sample: oto_data.samples,
            frq: oto_data.frq,
            tone: freq_midi.0 as i32,
            con_vel,
            offset: oto.offset,
            required_length: adjusted_length + 100.0,
            consonant: oto.consonant,
            cut_off: oto.cut_off,
            volume: (100f32 * source.volume_scale) as f64,
            modulation: 0.0,
            tempo: 0.0,
            pitch_bend: vec![0],
            flag_g: 0,
            flag_o: 0,
            flag_p: 86,
            flag_mt: 0,
            flag_mb: 0,
            flag_mv: 100,
        };

        synthesizer.add_request(&request, start, skip, adjusted_length, 5.0, 35.0);
        info!("{} {}..{}", mora.text, start, start + adjusted_length);

        f0.extend(vec![freq; (length * 1000.0 / MS_PER_FRAME) as usize]);

        prev_freq = freq;
        prev_vowel.clone_from(&mora.vowel.to_lowercase());
    }
    let wav = if aliases.is_empty() {
        warn!("No aliases found for {:?}", source.accent_phrase);
        vec![0.0]
    } else {
        info!("Synthesizing {:?}", aliases);

        let smooth_f0 = smooth(&f0, 10);

        synthesizer.set_curves(
            &smooth_f0.iter().map(|x| *x as f64).collect::<Vec<f64>>(),
            &vec![0.5f64; smooth_f0.len()],
            &vec![0.5f64; smooth_f0.len()],
            &vec![0.5f64; smooth_f0.len()],
            &vec![0.5f64; smooth_f0.len()],
        );
        synthesizer.synth()
    };

    SynthesisResult {
        wav,
        sum_length,
        last_freq: prev_freq,
        last_vowel: prev_vowel,
    }
}

pub async fn post_synthesis(
    Query(query): Query<AudioQueryQuery>,
    Json(audio_query): Json<HttpAudioQuery>,
) -> Result<Vec<u8>> {
    let audio_query = AudioQueryModel::from(&audio_query)
        .apply_speed_scale(audio_query.speed_scale)
        .apply_pitch_scale(audio_query.pitch_scale)
        .apply_intonation_scale(audio_query.intonation_scale);

    let ongens = ONGEN.get().unwrap().read().await;
    let ongen = ongens
        .values()
        .find(|ongen| ongen.id() == query.speaker)
        .unwrap();

    let mut phrase_waves: Vec<PhraseWaves> = Vec::new();
    let mut total_sum_length = 0.0f64;

    let mut prev_freq = 0.0;

    let vcv_connect = 0.1;

    for accent_phrase in audio_query.accent_phrases {
        let phrase_source = PhraseSource {
            prev_freq,
            accent_phrase: &accent_phrase,
            ongen,
            speaker: query.speaker,
            volume_scale: audio_query.volume_scale,
            vcv_connect,
        };
        let hash = phrase_source.hash();
        let cache_hit = {
            let caches = CACHES.read().await;
            caches.contains(&hash)
        };
        let cache_path = TEMPDIR.join(format!("cache-{}.msgpack", hash));
        let result = if cache_hit {
            info!("Cache hit for {}", hash);
            let data = fs_err::tokio::read(cache_path).await.unwrap();
            rmp_serde::from_read(data.as_slice()).unwrap()
        } else {
            info!("Cache miss for {}", hash);
            let result = synthesis_phrase(&phrase_source).await;
            fs_err::tokio::write(cache_path, rmp_serde::to_vec(&result).unwrap())
                .await
                .map_err(|e| Error::SynthesisFailed(anyhow!("Failed to write cache: {}", e)))?;
            let mut caches = CACHES.write().await;
            caches.insert(hash);
            result
        };

        let phrase_wave = PhraseWaves {
            data: result.wav,
            start_seconds: total_sum_length,
        };
        phrase_waves.push(phrase_wave);

        prev_freq = result.last_freq;

        total_sum_length += result.sum_length;
        if let Some(pause_mora) = accent_phrase.pause_mora {
            let pause_length = pause_mora.vowel_length;
            total_sum_length += pause_length as f64;
        }
    }
    let duration = total_sum_length
        + ((audio_query.pre_phoneme_length + audio_query.post_phoneme_length)
            / audio_query.speed_scale) as f64;
    let mut wav = vec![0.0; (duration * worldline::SAMPLE_RATE as f64) as usize];

    for phrase_wave in phrase_waves {
        let start = ((phrase_wave.start_seconds
            + (audio_query.pre_phoneme_length / audio_query.speed_scale) as f64)
            * worldline::SAMPLE_RATE as f64) as usize;
        let end = start + phrase_wave.data.len();
        if end > wav.len() {
            warn!(
                "Wave length exceeds allocated buffer: {} > {}",
                end,
                wav.len()
            );
            wav.resize(end, 0.0);
        }

        for (i, &sample) in phrase_wave.data.iter().enumerate() {
            wav[start + i] += sample;
        }
    }

    let sample_rate = audio_query
        .output_sampling_rate
        .as_f64()
        .expect("Failed to determine sampling rate") as u32;
    let wav = wav_io::resample::linear(wav, 1, worldline::SAMPLE_RATE, sample_rate);
    let wav = if audio_query.output_stereo {
        wav_io::utils::mono_to_stereo(wav)
    } else {
        wav
    };

    let result = wav_io::write_to_bytes(
        &WavHeader {
            sample_format: wav_io::header::SampleFormat::Float,
            channels: if audio_query.output_stereo { 2 } else { 1 },
            sample_rate,
            bits_per_sample: 32,
            list_chunk: None,
        },
        &wav,
    )
    .unwrap();

    Ok(result)
}
