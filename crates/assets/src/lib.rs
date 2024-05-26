use std::path::PathBuf;

#[cfg(debug_assertions)]
pub fn asset(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join(path)
}

#[cfg(not(debug_assertions))]
pub fn asset(path: &str) -> PathBuf {
    PathBuf::from(std::env::current_exe().unwrap().parent().unwrap()).join(path)
}

pub fn open_jtalk_dic() -> PathBuf {
    asset("dict")
}

pub fn tyc_utau() -> PathBuf {
    asset("tyc-utau")
}

pub fn sample_vvm() -> PathBuf {
    asset("sample.vvm")
}
