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
        "{}/external/OpenUtau/cpp",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
    );
    eprintln!("Building cpp code in {}", cpp_path);
    if duct::cmd!("bazelisk", "build", "//worldline")
        .dir(cpp_path)
        .run()
        .is_err()
    {
        // rust-analyzerだとなぜかエラーが出るので握りつぶす。
        // TODO: ちゃんと直す
        eprintln!("Failed to build cpp code");
        std::process::exit(0);
    }

    let out_dir = format!(
        "{}/target/{}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        std::env::var("PROFILE").unwrap()
    );

    let out_lib_path = format!(
        "{}/external/OpenUtau/cpp/bazel-bin/worldline/{}",
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

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=dylib=worldline");
}
