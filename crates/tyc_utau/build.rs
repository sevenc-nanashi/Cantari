use rc_zip_sync::ReadZip;
use std::path::PathBuf;

fn download_zip() {
    if std::fs::metadata("downloaded/tyc-utau.zip").is_ok() {
        return;
    }
    let mut zip = ureq::get("https://tyc.rei-yumesaki.net/files/voice/tyc-utau.zip")
        .call()
        .unwrap()
        .into_reader();
    let mut file = std::fs::File::create("downloaded/tyc-utau.zip.tmp").unwrap();
    std::io::copy(&mut zip, &mut file).unwrap();
    std::fs::rename("downloaded/tyc-utau.zip.tmp", "downloaded/tyc-utau.zip").unwrap();
}
fn extract_zip() {
    if std::fs::metadata("downloaded/tyc-utau/.done").is_ok() {
        return;
    }
    std::fs::create_dir_all("downloaded/tyc-utau").unwrap();
    let file = std::fs::File::open("downloaded/tyc-utau.zip").unwrap();
    let prefix = PathBuf::from("downloaded/tyc-utau/");
    let archive = file.read_zip().unwrap();
    // let mut archive = rc_zip::ZipArchive::new(file).unwrap();
    for file in archive.entries() {
        let name = file.sanitized_name().unwrap();
        if name.ends_with('/') {
            std::fs::create_dir_all(prefix.join(name)).unwrap();
        } else {
            if let Some(p) = PathBuf::from(name).parent() {
                if !p.exists() {
                    std::fs::create_dir_all(prefix.join(p)).unwrap();
                }
            }
            let mut outfile = std::fs::File::create(prefix.join(name)).unwrap();
            std::io::copy(&mut file.reader(), &mut outfile).unwrap();
        }
    }
    std::fs::File::create("downloaded/tyc-utau/.done").unwrap();
}

fn main() {
    download_zip();
    extract_zip();
}
