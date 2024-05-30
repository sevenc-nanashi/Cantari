use super::sys;
use dlopen2::wrapper::Container;
use once_cell::sync::Lazy;
use tracing::info;

static LIB: Lazy<Container<sys::WorldlineSys>> = Lazy::new(|| {
    let lib_name = if cfg!(target_os = "windows") {
        "worldline.dll"
    } else if cfg!(target_os = "macos") {
        "libworldline.dylib"
    } else {
        "libworldline.so"
    };
    let exe_path = std::env::current_exe().unwrap();
    let lib_paths = [
        exe_path.parent().unwrap().join(lib_name),
        exe_path.parent().unwrap().parent().unwrap().join(lib_name),
    ];
    let lib_path = lib_paths
        .iter()
        .find(|p| p.exists())
        .expect("Failed to find libworldline.so");

    unsafe {
        let lib = Container::load(lib_path);
        lib.unwrap()
    }
});

pub struct PhraseSynth {
    inner: Inner,
}

#[derive(Clone)]
struct Inner(*mut sys::PhraseSynth);

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

impl Default for PhraseSynth {
    fn default() -> Self {
        Self::new()
    }
}

impl PhraseSynth {
    pub fn new() -> Self {
        Self {
            inner: Inner(unsafe { LIB.PhraseSynthNew() }),
        }
    }

    pub fn add_request(
        &mut self,
        request: &crate::SynthRequest,
        pos_ms: f64,
        skip_ms: f64,
        length_ms: f64,
        fade_in_ms: f64,
        fade_out_ms: f64,
    ) {
        let c_request = request.into_sys();
        unsafe {
            LIB.PhraseSynthAddRequest(
                self.inner.0,
                &c_request,
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
        f0: &[f64],
        gender: &[f64],
        tension: &[f64],
        breathiness: &[f64],
        voicing: &[f64],
    ) {
        unsafe {
            LIB.PhraseSynthSetCurves(
                self.inner.0,
                f0.as_ptr(),
                gender.as_ptr(),
                tension.as_ptr(),
                breathiness.as_ptr(),
                voicing.as_ptr(),
                f0.len() as i32,
                log_callback,
            );
        }
    }

    pub fn synth(&mut self) -> Vec<f32> {
        let mut y = std::ptr::null_mut();
        unsafe {
            let len = LIB.PhraseSynthSynth(self.inner.0, &mut y, log_callback) as usize;
            let y = std::slice::from_raw_parts(y, len);
            y.to_vec()
        }
    }

    pub async fn synth_async(&mut self) -> Vec<f32> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let mut y = std::ptr::null_mut();
            let inner = inner;
            unsafe {
                let len = LIB.PhraseSynthSynth(inner.0, &mut y, log_callback) as usize;
                let y = std::slice::from_raw_parts(y, len);
                y.to_vec()
            }
        })
        .await
        .unwrap()
    }
}

impl Drop for PhraseSynth {
    fn drop(&mut self) {
        unsafe {
            LIB.PhraseSynthDelete(self.inner.0);
        }
    }
}

extern "C" fn log_callback(msg: *const std::os::raw::c_char) {
    let msg = unsafe { std::ffi::CStr::from_ptr(msg) };
    info!("{}", msg.to_string_lossy());
}
