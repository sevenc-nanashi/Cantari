mod phrase_synth;
mod synth_request;
pub mod sys;

pub use phrase_synth::PhraseSynth;
pub use synth_request::SynthRequest;

pub static SAMPLE_RATE: u32 = 44100;
pub static MS_PER_FRAME: f64 = 10.0;
