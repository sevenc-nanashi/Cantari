use super::sys;
use tracing::info;

pub struct PhraseSynth {
    inner: *mut sys::PhraseSynth,
}

impl Default for PhraseSynth {
    fn default() -> Self {
        Self::new()
    }
}

impl PhraseSynth {
    pub fn new() -> Self {
        Self {
            inner: unsafe { sys::PhraseSynthNew() },
        }
    }

    pub fn add_request(
        &mut self,
        request: &sys::SynthRequest,
        pos_ms: f64,
        skip_ms: f64,
        length_ms: f64,
        fade_in_ms: f64,
        fade_out_ms: f64,
    ) {
        unsafe {
            sys::PhraseSynthAddRequest(
                self.inner,
                request,
                pos_ms,
                skip_ms,
                length_ms,
                fade_in_ms,
                fade_out_ms,
                log_callback,
            );
        }
    }

    pub fn set_curves(
        &mut self,
        f0: &mut [f64],
        gender: &mut [f64],
        tension: &mut [f64],
        breathiness: &mut [f64],
        voicing: &mut [f64],
    ) {
        unsafe {
            sys::PhraseSynthSetCurves(
                self.inner,
                f0.as_mut_ptr(),
                gender.as_mut_ptr(),
                tension.as_mut_ptr(),
                breathiness.as_mut_ptr(),
                voicing.as_mut_ptr(),
                f0.len() as i32,
                log_callback,
            );
        }
    }

    pub fn synth(&mut self) -> Vec<f32> {
        let mut y = std::ptr::null_mut();
        unsafe {
            sys::PhraseSynthSynth(self.inner, &mut y, log_callback);
            let y = std::slice::from_raw_parts(y, 0);
            y.to_vec()
        }
    }
}

impl Drop for PhraseSynth {
    fn drop(&mut self) {
        unsafe {
            sys::PhraseSynthDelete(self.inner);
        }
    }
}

extern "C" fn log_callback(msg: *const std::os::raw::c_char) {
    let msg = unsafe { std::ffi::CStr::from_ptr(msg) };
    info!("{}", msg.to_string_lossy());
}