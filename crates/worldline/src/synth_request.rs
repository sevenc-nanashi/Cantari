use super::sys;
#[derive(Debug, Clone)]
pub struct SynthRequest {
    pub sample_fs: i32,
    pub sample: Vec<f64>,
    pub frq: Option<Vec<u8>>,
    pub tone: i32,
    pub con_vel: f64,
    pub offset: f64,
    pub required_length: f64,
    pub consonant: f64,
    pub cut_off: f64,
    pub volume: f64,
    pub modulation: f64,
    pub tempo: f64,
    pub pitch_bend: Vec<i32>,
    pub flag_g: i32,
    pub flag_o: i32,
    pub flag_p: i32,
    pub flag_mt: i32,
    pub flag_mb: i32,
    pub flag_mv: i32,
}

impl SynthRequest {
    pub fn into_sys(&self) -> sys::SynthRequest {
        sys::SynthRequest {
            sample_fs: self.sample_fs,
            sample_length: self.sample.len() as i32,
            sample: self.sample.as_ptr(),
            frq_length: self.frq.as_ref().map_or(0, |frq| frq.len() as i32),
            frq: self
                .frq
                .as_ref()
                .map_or(std::ptr::null(), |frq| frq.as_ptr() as *const i8),
            tone: self.tone,
            con_vel: self.con_vel,
            offset: self.offset,
            required_length: self.required_length,
            consonant: self.consonant,
            cut_off: self.cut_off,
            volume: self.volume,
            modulation: self.modulation,
            tempo: self.tempo,
            pitch_bend_length: self.pitch_bend.len() as i32,
            pitch_bend: self.pitch_bend.as_ptr(),
            flag_g: self.flag_g,
            flag_o: self.flag_o,
            flag_p: self.flag_p,
            flag_mt: self.flag_mt,
            flag_mb: self.flag_mb,
            flag_mv: self.flag_mv,
        }
    }
}
