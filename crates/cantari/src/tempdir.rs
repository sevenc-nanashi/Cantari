use std::path::PathBuf;

use once_cell::sync::Lazy;

pub static TEMPDIR: Lazy<PathBuf> = Lazy::new(|| {
    let base = if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("TEMP").unwrap())
    } else {
        PathBuf::from("/tmp")
    };
    base.join(".cantari")
});
