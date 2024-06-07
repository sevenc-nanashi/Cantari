use super::audio_query::HttpAudioQuery;
use crate::{
    error::{Error, Result},
    math::{smooth, MidiNote},
    model::{AccentPhraseModel, AudioQueryModel, MoraModel},
    ongen::{get_ongen_style_from_id, ONGEN},
    oto::{Oto, OtoData},
    settings::load_settings,
    tempdir::TEMPDIR,
};
use anyhow::anyhow;
use async_recursion::async_recursion;
use axum::{extract::Query, Json};
use itertools::izip;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use wav_io::header::WavHeader;
use worldline::{SynthRequest, MS_PER_FRAME};

static CACHES: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));
static PHRASE_PADDING: f64 = 500.0;
static MINIMUM_OVERLAP: f64 = 10.0;

static OTO_FALLBACKS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("お", "を");
    map.insert("を", "お");
    map.insert("ず", "づ");
    map.insert("じ", "ぢ");
    map.insert("づ", "ず");
    map.insert("ぢ", "じ");
    map
});

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
    accent_phrase: &'a AccentPhraseModel,
    prev_mora: Option<&'a MoraModel>,
    next_mora: Option<&'a MoraModel>,
    ongen: &'a crate::ongen::Ongen,
    speaker: u32,
    volume_scale: f32,

    key_shift: i8,
    whisper: bool,
    formant_shift: i8,
    breathiness: u8,
    tension: i8,
    peak_compression: u8,
    voicing: u8,
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
}

async fn get_oto<'a>(
    oto: &'a HashMap<String, Oto>,
    kana: &str,
    prefix: &str,
    suffix: &str,
    prev_vowel: &str,
) -> Option<(String, &'a Oto, OtoData)> {
    get_oto_inner(oto, kana, prefix, suffix, prev_vowel, true).await
}

#[async_recursion]
async fn get_oto_inner<'a>(
    oto: &'a HashMap<String, Oto>,
    kana: &str,
    prefix: &str,
    suffix: &str,
    prev_vowel: &str,
    find_fallback: bool,
) -> Option<(String, &'a Oto, OtoData)> {
    let aliases = [
        // 連続音（音質が安定しないので無効化）
        format!("{}{} {}{}", prefix, prev_vowel, kana, suffix),
        // 単独音2
        format!("{}{}{}", prefix, kana, suffix),
        // 単独音
        format!("{}- {}{}", prefix, kana, suffix),
    ];

    for alias in aliases.into_iter() {
        if let Some(oto) = oto.get(&alias) {
            match oto.read().await {
                Ok(oto_data) => return Some((alias, oto, oto_data.clone())),
                Err(e) => warn!("Failed to read oto data for {:?}: {:?}", alias, e),
            }
        }
    }

    if find_fallback {
        if let Some(fallback) = OTO_FALLBACKS.get(kana) {
            info!(
                "No oto found for {:?} {:?} {:?} {:?}, trying fallback {:?}",
                prefix, prev_vowel, kana, suffix, fallback
            );
            return get_oto_inner(oto, fallback, prefix, suffix, prev_vowel, false).await;
        }
    }

    None
}

fn con_vel_to_factor(con_vel: f64) -> f64 {
    2.0f64.powf((100.0 - con_vel) / 100.0)
}

fn factor_to_con_vel(factor: f64) -> f64 {
    (1.0 - factor.log2()) * 100.0
}

fn text_to_oto(text: &str) -> String {
    if text == "、" {
        "R".to_string()
    } else {
        kana::kata2hira(text)
    }
}

#[derive(Debug)]
struct Prerender<'a> {
    alias: String,
    freq: f32,
    oto: &'a Oto,
    oto_data: OtoData,
    mora: &'a MoraModel,
    note: MidiNote,
}

#[derive(Debug)]
struct AdjustedParams {
    preutter: f64,
    overlap: f64,
    skip: f64,
}

impl AdjustedParams {
    fn fade(&self) -> f64 {
        self.overlap.max(0.0)
    }

    fn shift(&self) -> f64 {
        self.overlap.min(0.0)
    }
}

async fn synthesis_phrase(source: &PhraseSource<'_>) -> SynthesisResult {
    let mut synthesizer = worldline::PhraseSynth::new();

    let mut prev_vowel = source
        .prev_mora
        .map_or("-".to_string(), |mora| mora.vowel.to_lowercase());
    let mut sum_length = 0.0;

    let mut f0 = Vec::new();

    let mut moras = source.accent_phrase.moras.clone();
    if let Some(pause_mora) = source.accent_phrase.pause_mora.clone() {
        moras.push(pause_mora);
    }

    let next_mora_or_empty = match source.next_mora {
        Some(next_mora) => vec![next_mora],
        None => vec![],
    };

    let mut otos: Vec<Prerender> = vec![];
    for mora in moras.iter().chain(next_mora_or_empty.into_iter()) {
        let pitch = if mora.pitch == 0.0 {
            5.5f32
        } else {
            mora.pitch
        };
        let freq = if source.whisper { pitch } else { pitch.exp() };
        let kana = text_to_oto(&mora.text);
        let freq_midi = MidiNote::from_frequency(freq);
        let freq_midi_number = freq_midi.0 as i32;
        let freq_midi_number = (freq_midi_number + source.key_shift as i32).clamp(
            MidiNote::from_str("C1").unwrap().0 as i32,
            MidiNote::from_str("B7").unwrap().0 as i32,
        ) as u8;
        let freq_midi = MidiNote(freq_midi_number);
        let (prefix, suffix) = source
            .ongen
            .prefix_suffix_map
            .get(freq_midi.to_string().as_str())
            .map_or(("", ""), |x| (&x.0, &x.1));
        match get_oto(&source.ongen.oto, &kana, prefix, suffix, &prev_vowel).await {
            Some((alias, oto, oto_data)) => {
                prev_vowel = mora.vowel.to_lowercase();
                otos.push(Prerender {
                    freq,
                    alias,
                    oto,
                    oto_data,
                    mora,
                    note: freq_midi,
                });
            }
            None => {
                warn!("No oto found for {:?}", kana);
                continue;
            }
        }
    }

    let aliases = otos
        .iter()
        .map(|prerender| prerender.alias.clone())
        .collect::<Vec<String>>();

    if aliases.is_empty() {
        warn!("No aliases found for {:?}", source.accent_phrase);
        SynthesisResult {
            wav: vec![],
            sum_length: 0.0,
        }
    } else {
        info!("Calculating {:?}", aliases);
        let mut sum_length = 0.0;
        let con_vels: Vec<f64> = otos
            .iter()
            .map(|current| {
                // if let Some(consonant_length) = current.mora.consonant_length {
                //     let consonant_length = (consonant_length * 1000.0) as f64;
                //     let oto_consonant_length = (current.oto.preutter - current.oto.overlap)
                //         + (current.oto.consonant - current.oto.preutter) / 2.0;
                //     let vel = factor_to_con_vel((consonant_length) / oto_consonant_length)
                //         .clamp(0.0, 200.0);
                //     if vel.is_nan() {
                //         100.0
                //     } else {
                //         vel
                //     }
                // } else {
                //     100.0
                // }
                100.0
            })
            .collect();

        let adjusted_params: Vec<AdjustedParams> = otos
            .iter()
            .zip(con_vels.iter())
            .enumerate()
            .map(|(i, (current, con_vel))| {
                let prev_mora = if i == 0 {
                    if let Some(prev_mora) = source.prev_mora {
                        prev_mora
                    } else {
                        return AdjustedParams {
                            preutter: current.oto.preutter,
                            overlap: current.oto.overlap,
                            skip: 0.0,
                        };
                    }
                } else {
                    &moras[i - 1]
                };
                let prev_length = ((prev_mora.vowel_length
                    + prev_mora.consonant_length.unwrap_or(0.0))
                    * 1000.0) as f64;
                let real_preutter = current.oto.preutter * con_vel_to_factor(*con_vel);
                let real_overlap = current.oto.overlap * con_vel_to_factor(*con_vel);

                if prev_length / 2.0 < real_preutter - real_overlap {
                    let at_preutter = real_preutter / (real_preutter - real_overlap) * prev_length;
                    let at_overlap = real_overlap / (real_preutter - real_overlap) * prev_length;
                    let at_skip = real_preutter - at_preutter;
                    AdjustedParams {
                        preutter: at_preutter,
                        overlap: at_overlap,
                        skip: at_skip,
                    }
                } else {
                    AdjustedParams {
                        preutter: real_preutter,
                        overlap: real_overlap,
                        skip: 0.0,
                    }
                }
            })
            .collect();

        for (i, (current, con_vel, adjusted_param)) in
            izip!(otos.iter(), con_vels.iter(), adjusted_params.iter()).enumerate()
        {
            if source.next_mora.is_some() && i == otos.len() - 1 {
                debug!("Next phrase's mora, breaking loop");
                break;
            }
            debug!("Adjusted params: {:?}", &adjusted_param);
            debug!(
                "Consonant velocity: {:?} (x{:?})",
                con_vel,
                con_vel_to_factor(*con_vel)
            );
            let start =
                sum_length + PHRASE_PADDING + adjusted_param.shift() - adjusted_param.preutter;
            let length = ((current.mora.vowel_length
                + current.mora.consonant_length.unwrap_or(0.0))
                * 1000.0) as f64;
            sum_length += length;

            let skip = adjusted_param.skip.max(0.0) - start.min(0.0);
            let start = start.max(0.0);

            let adjusted_length = length
                + adjusted_param.preutter
                + if i < otos.len() - 1 {
                    let next_adjusted_params = &adjusted_params[i + 1];
                    next_adjusted_params.overlap - next_adjusted_params.preutter
                } else {
                    0.0
                };

            let request = SynthRequest {
                sample_fs: current.oto_data.header.sample_rate as i32,
                sample: current.oto_data.samples.clone(),
                frq: current.oto_data.frq.clone(),
                tone: current.note.0 as i32,
                con_vel: *con_vel,
                offset: current.oto.offset,
                required_length: adjusted_length + skip + 100.0,
                consonant: current.oto.consonant - skip,
                cut_off: current.oto.cut_off - skip * current.oto.cut_off.signum(),
                volume: (100f32 * source.volume_scale) as f64,
                modulation: 0.0,
                tempo: 0.0,
                pitch_bend: vec![0],
                flag_g: source.formant_shift as _,
                flag_o: 0,
                flag_p: source.peak_compression as _,
                flag_mt: source.tension as _,
                flag_mb: source.breathiness as _,
                flag_mv: source.voicing as _,
            };

            let next_fade = if i < otos.len() - 1 {
                let next_adjusted_params = &adjusted_params[i + 1];
                next_adjusted_params.fade()
            } else {
                0.0
            };

            let fade = adjusted_param.fade();

            debug!(
                "Request: alias: {:?}, start: {:?}, skip: {:?}, length: {:?} -> {:?}, fade: {:?}, next fade: {:?}",
                &current.alias, start, skip, length, adjusted_length, fade, next_fade
            );
            synthesizer.add_request(&request, start, skip, adjusted_length, fade, next_fade);

            if i == 0 {
                f0.extend(vec![current.freq; (PHRASE_PADDING / MS_PER_FRAME) as usize]);
            }

            f0.extend(vec![current.freq; (length / MS_PER_FRAME) as usize]);

            if i + 1 == otos.len() {
                f0.extend(vec![current.freq; (PHRASE_PADDING / MS_PER_FRAME) as usize]);
            }

            prev_vowel.clone_from(&current.mora.vowel.to_lowercase());
        }
        info!("Synthesizing {:?}", aliases);

        let smooth_f0 = smooth(&f0, 10);

        synthesizer.set_curves(
            &smooth_f0.iter().map(|x| *x as f64).collect::<Vec<f64>>(),
            &vec![0.5; smooth_f0.len()],
            &vec![0.5; smooth_f0.len()],
            &vec![0.5; smooth_f0.len()],
            &vec![0.5; smooth_f0.len()],
            // &vec![source.formant_shift as f64; smooth_f0.len()],
            // &vec![source.tension as f64; smooth_f0.len()],
            // &vec![source.breathiness as f64; smooth_f0.len()],
            // &vec![source.voicing as f64; smooth_f0.len()],
        );
        let wav = synthesizer.synth_async().await;

        SynthesisResult { wav, sum_length }
    }
}

pub async fn post_synthesis(
    Query(query): Query<AudioQueryQuery>,
    Json(audio_query): Json<HttpAudioQuery>,
) -> Result<Vec<u8>> {
    let ongens = ONGEN.get().unwrap().read().await;
    let settings = load_settings().await;
    let audio_query = AudioQueryModel::from(&audio_query)
        .apply_speed_scale(audio_query.speed_scale)
        .apply_pitch_scale(audio_query.pitch_scale)
        .apply_intonation_scale(audio_query.intonation_scale);

    let (ongen, style_settings) = get_ongen_style_from_id(&ongens, &settings, query.speaker)
        .await
        .ok_or_else(|| crate::error::Error::CharacterNotFound)?;

    let results = futures::future::join_all(audio_query.accent_phrases.iter().enumerate().map(
        |(i, accent_phrase)| {
            let next_mora = audio_query
                .accent_phrases
                .get(i + 1)
                .as_ref()
                .map(|p| &p.moras[0]);
            let prev_mora = if i == 0 {
                None
            } else {
                Some(audio_query.accent_phrases[i - 1].moras.last().unwrap())
            };
            async move {
                // let prev_freq =
                //     if i == 0 {
                //     440.0f32
                // } else {
                //     accent_phrases[i - 1].moras.last().unwrap().pitch.exp()
                // };
                let phrase_source = PhraseSource {
                    accent_phrase,
                    next_mora,
                    prev_mora,
                    ongen,
                    speaker: query.speaker,
                    volume_scale: audio_query.volume_scale,

                    key_shift: style_settings.key_shift,
                    whisper: style_settings.whisper,
                    formant_shift: style_settings.formant_shift,
                    breathiness: style_settings.breathiness,
                    tension: style_settings.tension,
                    peak_compression: style_settings.peak_compression,
                    voicing: style_settings.voicing,
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
                    if let Err(e) =
                        fs_err::tokio::write(cache_path, rmp_serde::to_vec(&result).unwrap()).await
                    {
                        error!("Failed to write cache: {}", e);
                    };
                    let mut caches = CACHES.write().await;
                    caches.insert(hash);
                    result
                };

                result
            }
        },
    ))
    .await;
    let mut total_sum_length = 0.0;
    let mut phrase_waves: Vec<PhraseWaves> = vec![];
    for result in results {
        phrase_waves.push(PhraseWaves {
            data: result.wav,
            start_seconds: total_sum_length,
        });

        total_sum_length += result.sum_length / 1000.0;
    }
    let duration = total_sum_length
        + ((audio_query.pre_phoneme_length + audio_query.post_phoneme_length)
            / audio_query.speed_scale) as f64;
    let mut wav = vec![0.0; (duration * worldline::SAMPLE_RATE as f64) as usize];

    for phrase_wave in phrase_waves {
        let start = ((phrase_wave.start_seconds
            + (audio_query.pre_phoneme_length / audio_query.speed_scale) as f64
            - PHRASE_PADDING / 1000.0)
            * worldline::SAMPLE_RATE as f64) as i64;
        let end = start + phrase_wave.data.len() as i64;
        if end > (wav.len() as i64) {
            warn!(
                "Wave length exceeds allocated buffer: {} > {}",
                end,
                wav.len()
            );
            wav.resize(end as usize, 0.0);
        }

        for (i, &sample) in phrase_wave.data.iter().enumerate() {
            let item_index = start + i as i64;
            if item_index < 0 {
                continue;
            }
            wav[item_index as usize] += sample;
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
