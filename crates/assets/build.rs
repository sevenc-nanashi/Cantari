mod tyc_utau {
    use rc_zip_sync::ReadZip;
    use std::path::PathBuf;

    pub fn download() {
        if std::fs::metadata("assets/tyc-utau.zip").is_ok() {
            return;
        }
        let mut zip = ureq::get("https://tyc.rei-yumesaki.net/files/voice/tyc-utau.zip")
            .call()
            .unwrap()
            .into_reader();
        let mut file = std::fs::File::create("assets/tyc-utau.zip.tmp").unwrap();
        std::io::copy(&mut zip, &mut file).unwrap();
        std::fs::rename("assets/tyc-utau.zip.tmp", "assets/tyc-utau.zip").unwrap();
    }
    pub fn extract() {
        if std::fs::metadata("assets/tyc-utau/.done").is_ok() {
            return;
        }
        std::fs::create_dir_all("assets/tyc-utau").unwrap();
        let file = std::fs::File::open("assets/tyc-utau.zip").unwrap();
        let prefix = PathBuf::from("assets/tyc-utau/");
        let archive = file.read_zip().unwrap();
        // let mut archive = rc::ZipArchive::new(file).unwrap();
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
        std::fs::File::create("assets/tyc-utau/.done").unwrap();
    }
}

mod open_jtalk_dict {
    use std::path::PathBuf;

    use fs_extra::dir::CopyOptions;
    pub fn download() {
        if std::fs::metadata("assets/dict.tgz").is_ok() {
            return;
        }
        let mut zip = ureq::get("https://jaist.dl.sourceforge.net/project/open-jtalk/Dictionary/open_jtalk_dic-1.11/open_jtalk_dic_utf_8-1.11.tar.gz")
        .call()
        .unwrap()
        .into_reader();
        let mut file = std::fs::File::create("assets/dict.tgz.tmp").unwrap();
        std::io::copy(&mut zip, &mut file).unwrap();
        std::fs::rename("assets/dict.tgz.tmp", "assets/dict.tgz").unwrap();
    }
    pub fn extract() {
        if std::fs::metadata("assets/dict/.done").is_ok() {
            return;
        }
        let file = std::fs::File::open("assets/dict.tgz").unwrap();
        let prefix = PathBuf::from("assets/");

        let dict_tar = flate2::read::GzDecoder::new(file);

        let mut dict_archive = tar::Archive::new(dict_tar);
        dict_archive.unpack(prefix).unwrap();
        std::fs::rename("assets/open_jtalk_dic_utf_8-1.11", "assets/dict").unwrap();
        std::fs::File::create("assets/dict/.done").unwrap();
    }
    pub fn move_dict() {
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("../../../");
        std::fs::create_dir_all(out_dir.join("dict")).unwrap();
        fs_extra::dir::copy(
            "assets/dict",
            out_dir.join("dict"),
            &CopyOptions {
                overwrite: true,
                ..Default::default()
            },
        )
        .unwrap();
    }
}
mod sample_vvm {
    use std::io::{Read, Write};
    use std::path::PathBuf;
    pub fn archive() {
        if std::fs::metadata("assets/sample.vvm").is_ok() {
            return;
        }
        let root = PathBuf::from("voicevox_core/model/sample.vvm");
        let file = std::fs::File::create("assets/sample.vvm.tmp").unwrap();
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        let mut archive = zip::ZipWriter::new(file);
        let mut buffer = Vec::new();
        for entry in walkdir::WalkDir::new(&root) {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = path.strip_prefix(&root).unwrap();
            if path.is_file() {
                archive.start_file(name.to_str().unwrap(), options).unwrap();
                let mut file = std::fs::File::open(path).unwrap();
                file.read_to_end(&mut buffer).unwrap();
                archive.write_all(&buffer).unwrap();
                buffer.clear();
            }
        }
        archive.finish().unwrap();
        std::fs::rename("assets/sample.vvm.tmp", "assets/sample.vvm").unwrap();
    }

    pub fn move_vvm() {
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("../../../");
        std::fs::copy("assets/sample.vvm", out_dir.join("sample.vvm")).unwrap();
    }
}

mod frontend {
    pub fn move_build() {
        if std::fs::metadata("frontend/dist/index.html").is_err() {
            if std::env::var("PROFILE").unwrap() == "release" {
                panic!("frontend/dist/index.html not found");
            } else {
                eprintln!("frontend/dist/index.html not found");
                return;
            }
        }
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let out_dir = std::path::PathBuf::from(out_dir);
        let out_dir = out_dir.join("../../../");
        std::fs::copy("frontend/dist/index.html", out_dir.join("settings.html")).unwrap();
    }
}

fn main() {
    open_jtalk_dict::download();
    tyc_utau::download();
    open_jtalk_dict::extract();
    tyc_utau::extract();
    sample_vvm::archive();

    if std::env::var("PROFILE").unwrap() == "release" {
        open_jtalk_dict::move_dict();
        sample_vvm::move_vvm();
        frontend::move_build();
    }
}
