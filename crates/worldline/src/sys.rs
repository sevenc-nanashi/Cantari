#![allow(non_snake_case, clippy::too_many_arguments)]
use dlopen2::wrapper::WrapperApi;
use std::ffi::{c_char, c_double, c_float, c_int};

#[repr(C)]
pub struct PhraseSynth {
    // Opaque type
    _private: [u8; 0],
}

#[repr(C)]
pub struct SynthRequest {
    pub sample_fs: i32,
    pub sample_length: i32,
    pub sample: *const c_double,
    pub frq_length: i32,
    pub frq: *const c_char,
    pub tone: i32,
    pub con_vel: c_double,
    pub offset: c_double,
    pub required_length: c_double,
    pub consonant: c_double,
    pub cut_off: c_double,
    pub volume: c_double,
    pub modulation: c_double,
    pub tempo: c_double,
    pub pitch_bend_length: i32,
    pub pitch_bend: *const i32,
    pub flag_g: c_int,
    pub flag_o: c_int,
    pub flag_p: c_int,
    pub flag_mt: c_int,
    pub flag_mb: c_int,
    pub flag_mv: c_int,
}

type LogCallback = extern "C" fn(message: *const c_char);

#[derive(WrapperApi)]
pub struct WorldlineSys {
    F0: unsafe extern "C" fn(
        samples: *mut c_float,
        length: c_int,
        fs: c_int,
        frame_period: c_double,
        method: c_int,
        f0: *mut *mut c_double,
    ) -> c_int,

    DecodeMgc: unsafe extern "C" fn(
        f0_length: c_int,
        mgc: *mut c_double,
        mgc_size: c_int,
        fft_size: c_int,
        fs: c_int,
        spectrogram: *mut *mut c_double,
    ) -> c_int,

    DecodeBap: unsafe extern "C" fn(
        f0_length: c_int,
        bap: *mut c_double,
        fft_size: c_int,
        fs: c_int,
        aperiodicity: *mut *mut c_double,
    ) -> c_int,

    WorldSynthesis: unsafe extern "C" fn(
        f0: *const c_double,
        f0_length: c_int,
        mgc_or_sp: *const c_double,
        is_mgc: bool,
        mgc_size: c_int,
        bap_or_ap: *const c_double,
        is_bap: bool,
        fft_size: c_int,
        frame_period: c_double,
        fs: c_int,
        y: *mut *mut c_double,
        gender: *const c_double,
        tension: *const c_double,
        breathiness: *const c_double,
        voicing: *const c_double,
    ) -> c_int,

    Resample: unsafe extern "C" fn(request: *const SynthRequest, y: *mut *mut c_float) -> c_int,

    PhraseSynthNew: unsafe extern "C" fn() -> *mut PhraseSynth,

    PhraseSynthDelete: unsafe extern "C" fn(phrase_synth: *mut PhraseSynth),

    PhraseSynthAddRequest: unsafe extern "C" fn(
        phrase_synth: *mut PhraseSynth,
        request: *const SynthRequest,
        pos_ms: c_double,
        skip_ms: c_double,
        length_ms: c_double,
        fade_in_ms: c_double,
        fade_out_ms: c_double,
        logCallback: LogCallback,
    ),

    PhraseSynthSetCurves: unsafe extern "C" fn(
        phrase_synth: *mut PhraseSynth,
        f0: *const c_double,
        gender: *const c_double,
        tension: *const c_double,
        breathiness: *const c_double,
        voicing: *const c_double,
        length: c_int,
        logCallback: LogCallback,
    ),

    PhraseSynthSynth: unsafe extern "C" fn(
        phrase_synth: *mut PhraseSynth,
        y: *mut *mut c_float,
        logCallback: LogCallback,
    ) -> c_int,
}
