use super::audio_query::HttpAudioQuery;
use crate::{
    error::Result,
    math::{smooth, MidiNote},
    model::{AudioQueryModel, MoraModel},
    ongen::{get_ongen_style_from_id, ONGEN},
    oto::{Oto, OtoData},
    settings::load_settings,
};
use async_recursion::async_recursion;
use axum::{extract::Query, Json};
use itertools::izip;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};
use tracing::{debug, info, warn};
use wav_io::header::WavHeader;
use worldline::{SynthRequest, MS_PER_FRAME};

static PHRASE_PADDING: f64 = 500.0;

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
    kana::kata2hira(text)
}

#[derive(Debug)]
struct Prerender<'a> {
    alias: String,
    freq: f32,
    oto: Option<&'a Oto>,
    oto_data: Option<OtoData>,
    mora: &'a MoraModel,
    note: MidiNote,
}

#[derive(Debug)]
struct AdjustedParam {
    preutter: f64,
    overlap: f64,
    skip: f64,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum SynthThreadMessage {
    Request(String, SynthRequest, f64, f64, f64, f64, f64),
    F0(Vec<f32>),
    Do,
}

impl AdjustedParam {
    fn fade(&self) -> f64 {
        self.overlap.max(0.0)
    }

    fn shift(&self) -> f64 {
        self.overlap.min(0.0)
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

    let mut synthesizer = worldline::PhraseSynth::new();

    let mut prev_vowel = "-".to_string();

    let mut f0 = Vec::new();

    let moras = audio_query
        .accent_phrases
        .iter()
        .flat_map(|x| {
            let mut moras = x.moras.iter().collect::<Vec<&MoraModel>>();
            if let Some(pause_mora) = x.pause_mora.as_ref() {
                moras.push(pause_mora);
            }
            moras
        })
        .collect::<Vec<&MoraModel>>();

    let mut otos: Vec<Prerender> = vec![];
    for mora in moras.iter() {
        let pitch = if mora.pitch == 0.0 {
            5.5f32
        } else {
            mora.pitch
        };
        let freq = if style_settings.whisper {
            pitch
        } else {
            pitch.exp()
        };
        let kana = text_to_oto(&mora.text);
        let freq_midi = MidiNote::from_frequency(freq);
        let freq_midi_number = freq_midi.0 as i32;
        let freq_midi_number = (freq_midi_number + style_settings.key_shift as i32).clamp(
            MidiNote::from_str("C1").unwrap().0 as i32,
            MidiNote::from_str("B7").unwrap().0 as i32,
        ) as u8;
        let freq_midi = MidiNote(freq_midi_number);
        let (prefix, suffix) = ongen
            .prefix_suffix_map
            .get(freq_midi.to_string().as_str())
            .map_or(("", ""), |x| (&x.0, &x.1));
        match get_oto(&ongen.oto, &kana, prefix, suffix, &prev_vowel).await {
            Some((alias, oto, oto_data)) => {
                prev_vowel = mora.vowel.to_lowercase();
                otos.push(Prerender {
                    freq,
                    alias,
                    oto: Some(oto),
                    oto_data: Some(oto_data),
                    mora,
                    note: freq_midi,
                });
            }
            None => {
                if kana != "、" {
                    warn!("No oto found for {:?}", kana);
                }
                otos.push(Prerender {
                    freq,
                    alias: "".to_string(),
                    oto: None,
                    oto_data: None,
                    mora,
                    note: freq_midi,
                });
            }
        }
    }

    let aliases = otos
        .iter()
        .map(|prerender| prerender.alias.clone())
        .collect::<Vec<String>>();

    let mut sum_length = 0.0;
    let wav = if aliases.is_empty() {
        warn!("No aliases found");
        vec![]
    } else {
        info!("Calculating {:?}", aliases);
        let con_vels: Vec<f64> = otos
            .iter()
            .map(|current| {
                let Some(oto) = &current.oto else {
                    return 100.0;
                };
                if let Some(consonant_length) = current.mora.consonant_length {
                    let consonant_length = (consonant_length * 1000.0) as f64;
                    let oto_consonant_length =
                        (oto.preutter - oto.overlap) / 2.0 + (oto.consonant - oto.preutter) / 2.0;
                    let vel = factor_to_con_vel((consonant_length) / oto_consonant_length)
                        .clamp(100.0, 275.0);
                    if vel.is_nan() {
                        100.0
                    } else {
                        vel
                    }
                } else {
                    100.0
                }
            })
            .collect();

        sum_length = otos.first().map_or(0.0, |x| {
            ((x.mora.consonant_length.unwrap_or(0.0)) * 1000.0) as f64
        });

        let adjusted_params: Vec<AdjustedParam> = otos
            .iter()
            .zip(con_vels.iter())
            .enumerate()
            .map(|(i, (current, con_vel))| {
                let Some(oto) = &current.oto else {
                    return AdjustedParam {
                        preutter: 0.0,
                        overlap: 0.0,
                        skip: 0.0,
                    };
                };
                let prev_mora = if i == 0 {
                    return AdjustedParam {
                        preutter: oto.preutter,
                        overlap: 0.0,
                        skip: 0.0,
                    };
                } else {
                    &moras[i - 1]
                };
                let prev_length = ((prev_mora.vowel_length
                    + current.mora.consonant_length.unwrap_or(0.0))
                    * 1000.0) as f64;
                let real_preutter = oto.preutter * con_vel_to_factor(*con_vel);
                let real_overlap = oto.overlap * con_vel_to_factor(*con_vel);

                if prev_length / 2.0 < real_preutter - real_overlap {
                    let at_preutter = real_preutter / (real_preutter - real_overlap) * prev_length;
                    let at_overlap = real_overlap / (real_preutter - real_overlap) * prev_length;
                    let at_skip = real_preutter - at_preutter;
                    AdjustedParam {
                        preutter: at_preutter,
                        overlap: at_overlap,
                        skip: at_skip,
                    }
                } else {
                    AdjustedParam {
                        preutter: real_preutter,
                        overlap: real_overlap,
                        skip: 0.0,
                    }
                }
            })
            .collect();

        debug!("Consonant velocities: {:?}", &con_vels);
        debug!("Adjusted params: {:?}", &adjusted_params);

        let (message_sender, message_receiver) = std::sync::mpsc::channel::<SynthThreadMessage>();

        let wav_task = tokio::task::spawn_blocking(move || {
            for message in message_receiver.iter() {
                match message {
                    SynthThreadMessage::Request(
                        alias,
                        request,
                        start,
                        skip,
                        length,
                        fade,
                        next_fade,
                    ) => {
                        debug!(
                            "Adding request: {:?} {:?} {:?} {:?} {:?} {:?}",
                            alias, start, skip, length, fade, next_fade
                        );
                        synthesizer.add_request(&request, start, skip, length, fade, next_fade);
                    }
                    SynthThreadMessage::F0(f0) => {
                        debug!("Setting f0");
                        synthesizer.set_curves(
                            &f0.iter().map(|x| *x as f64).collect::<Vec<f64>>(),
                            &vec![0.5; f0.len()],
                            &vec![0.5; f0.len()],
                            &vec![0.5; f0.len()],
                            &vec![0.5; f0.len()],
                        );
                    }
                    SynthThreadMessage::Do => break,
                }
            }

            info!("Synthesizing...");

            synthesizer.synth()
        });

        for (i, (current, con_vel, adjusted_param)) in
            izip!(otos.iter(), con_vels.iter(), adjusted_params.iter()).enumerate()
        {
            let span = tracing::debug_span!("mora", oto = %current.alias);
            let _guard = span.enter();

            debug!("Adjusted params: {:?}", &adjusted_param);
            debug!(
                "Consonant velocity: {:?} (x{:?})",
                con_vel,
                con_vel_to_factor(*con_vel)
            );
            let start =
                sum_length + PHRASE_PADDING + adjusted_param.shift() - adjusted_param.preutter;
            let length = ((current.mora.vowel_length
                + otos
                    .get(i + 1)
                    .map_or(0.0, |next| next.mora.consonant_length.unwrap_or(0.0)))
                * 1000.0) as f64;
            sum_length += length;

            let Some(oto) = &current.oto else {
                continue;
            };
            let oto_data = current.oto_data.as_ref().unwrap();

            let skip = adjusted_param.skip.max(0.0) - start.min(0.0);
            let start = start.max(0.0);

            let adjusted_length = length
                + adjusted_param.preutter
                + adjusted_params
                    .get(i + 1)
                    .map_or(0.0, |next_adjusted_params| {
                        next_adjusted_params.overlap - next_adjusted_params.preutter
                    });

            let mut next_fade = if i < otos.len() - 1 {
                let next_adjusted_params = &adjusted_params[i + 1];
                next_adjusted_params.fade()
            } else {
                0.0
            };

            let mut fade = if i == 0 { 0.0 } else { adjusted_param.fade() };

            let volume = if fade + next_fade > adjusted_length {
                warn!(
                    "Fade length exceeds adjusted length: {:?} + {:?} > {:?}",
                    fade, next_fade, adjusted_length
                );

                let volume = ((adjusted_length - next_fade) / fade).clamp(0.0, 1.0);
                fade = (adjusted_length - next_fade).max(0.0);
                next_fade = (adjusted_length - fade).max(0.0);

                volume
            } else {
                1.0
            };

            let request = SynthRequest {
                sample_fs: oto_data.header.sample_rate as i32,
                sample: oto_data.samples.clone(),
                frq: oto_data.frq.clone(),
                tone: current.note.0 as i32,
                con_vel: *con_vel,
                offset: oto.offset,
                required_length: adjusted_length + skip + 100.0,
                consonant: oto.consonant - skip,
                cut_off: oto.cut_off - skip * oto.cut_off.signum(),
                volume: (100f64 * volume) * (audio_query.volume_scale as f64),
                modulation: 0.0,
                tempo: 0.0,
                pitch_bend: vec![0],
                flag_g: style_settings.formant_shift as _,
                flag_o: 0,
                flag_p: style_settings.peak_compression as _,
                flag_mt: style_settings.tension as _,
                flag_mb: style_settings.breathiness as _,
                flag_mv: style_settings.voicing as _,
            };

            message_sender
                .send(SynthThreadMessage::Request(
                    current.alias.clone(),
                    request,
                    start,
                    skip,
                    adjusted_length,
                    fade,
                    next_fade,
                ))
                .expect("Failed to send message");

            if i == 0 {
                f0.extend(vec![current.freq; (PHRASE_PADDING / MS_PER_FRAME) as usize]);
            }

            f0.extend(vec![current.freq; (length / MS_PER_FRAME) as usize]);

            if i == otos.len() - 1 {
                f0.extend(vec![current.freq; (PHRASE_PADDING / MS_PER_FRAME) as usize]);
            }

            prev_vowel.clone_from(&current.mora.vowel.to_lowercase());
        }
        info!("Synthesizing {:?}", aliases);

        let smooth_f0 = smooth(&f0, 10);

        message_sender
            .send(SynthThreadMessage::F0(smooth_f0))
            .expect("Failed to send message");

        message_sender
            .send(SynthThreadMessage::Do)
            .expect("Failed to send message");

        wav_task.await.unwrap()
    };

    let pre_phoneme_length = (audio_query.pre_phoneme_length / audio_query.speed_scale) as f64;
    let post_phoneme_length = (audio_query.post_phoneme_length / audio_query.speed_scale) as f64;

    let duration = pre_phoneme_length + sum_length + post_phoneme_length;

    let mut padded_wav = vec![0.0; (duration * worldline::SAMPLE_RATE as f64) as usize];

    let start = (pre_phoneme_length * worldline::SAMPLE_RATE as f64) as i64
        - (PHRASE_PADDING / 1000.0 * worldline::SAMPLE_RATE as f64) as i64;
    for (i, &sample) in wav.iter().enumerate() {
        let index = start + i as i64;
        if index < 0 || index >= padded_wav.len() as i64 {
            continue;
        }
        padded_wav[index as usize] = sample;
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
