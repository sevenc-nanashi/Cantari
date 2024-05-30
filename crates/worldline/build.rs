use std::os::unix::process::CommandExt;

static LIB_NAME: &str = if cfg!(target_os = "windows") {
    "worldline.dll"
} else if cfg!(target_os = "macos") {
    "libworldline.dylib"
} else {
    "libworldline.so"
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let cpp_path = format!(
        "{}/OpenUtau/cpp",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
    );
    eprintln!("Building cpp code in {}", cpp_path);
    let output = if std::env::var("TARGET").unwrap().contains("windows") {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg("bazelisk build //worldline")
            .current_dir(cpp_path)
            .output()
            .unwrap()
    } else {
        std::process::Command::new("bazelisk")
            .arg("build")
            .arg("//worldline")
            .current_dir(cpp_path)
            .output()
            .unwrap()
    };

    if !output.status.success() {
        if std::env::var("PROFILE").unwrap() == "release" {
            panic!("Failed to build cpp code: {:?}", output);
        }
        // rust-analyzerだとなぜかエラーが出るので握りつぶす。
        // TODO: ちゃんと直す
        eprintln!("Failed to build cpp code: {:?}", output);
        std::process::exit(0);
    }

    let out_dir = format!("{}/../../../", std::env::var("OUT_DIR").unwrap(),);

    let out_lib_path = format!(
        "{}/OpenUtau/cpp/bazel-bin/worldline/{}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        LIB_NAME
    );
    let target_lib_path = format!("{}/{}", out_dir, LIB_NAME);

    eprintln!("Copying {} to {}", out_lib_path, target_lib_path);

    // メモ：bazel-binの中身をstd::fs::copyでコピーするとPermission deniedエラーが出るので、
    // read -> writeでコピーする
    // TODO: もっといい方法があれば変える
    let binary = std::fs::read(&out_lib_path).unwrap();
    std::fs::write(&target_lib_path, binary).unwrap();
}
