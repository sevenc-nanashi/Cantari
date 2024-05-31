use axum::{response::IntoResponse, Json};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    pub version: String,
    pub authors: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineManifest {
    pub manifest_version: String,
    pub name: String,
    pub brand_name: String,
    pub uuid: String,
    pub url: String,
    pub icon: String,
    pub default_sampling_rate: i64,
    pub terms_of_service: String,
    pub update_infos: Vec<UpdateInfo>,
    pub dependency_licenses: Vec<DependencyLicense>,
    pub supported_features: SupportedFeatures,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub descriptions: Vec<String>,
    pub contributors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyLicense {
    pub name: String,
    pub version: Option<String>,
    pub license: Option<String>,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportedFeatures {
    pub adjust_mora_pitch: bool,
    pub adjust_phoneme_length: bool,
    pub adjust_speed_scale: bool,
    pub adjust_pitch_scale: bool,
    pub adjust_intonation_scale: bool,
    pub adjust_volume_scale: bool,
    pub interrogative_upspeak: bool,
    pub synthesis_morphing: bool,
    pub manage_library: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupportedDeveices {
    pub cpu: bool,
    pub cuda: bool,
    pub dml: bool,
}

pub async fn get_version() -> impl IntoResponse {
    Json(env!("CARGO_PKG_VERSION"))
}

pub async fn get_engine_manifest() -> Json<EngineManifest> {
    let icon =
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(include_bytes!("../../icon.png"));
    let mut licenses: Vec<License> =
        serde_json::from_slice(include_bytes!("../licenses.json")).unwrap();
    licenses.remove(
        licenses
            .iter()
            .position(|license| license.name == "cantari")
            .unwrap(),
    );
    let mut dependency_licenses = licenses
        .into_iter()
        .map(|license| DependencyLicense {
            name: license.name.clone(),
            version: Some(license.version),
            license: license.license,
            text: if let Some(repository) = license.repository.as_ref() {
                format!("<{}> を参照してください。", repository)
            } else {
                format!(
                    "https://crates.io/crates/{} を参照してください。",
                    &license.name
                )
            },
        })
        .collect::<Vec<_>>();
    let external_licenses: Vec<DependencyLicense> =
        serde_json::from_slice(include_bytes!("../ex_licenses.json")).unwrap();
    dependency_licenses.extend(external_licenses);
    Json(EngineManifest {
        manifest_version: "0.13.1".to_string(),
        name: "Cantari".to_string(),
        brand_name: "Cantari".to_string(),
        uuid: "a6b5fbf0-4561-43b3-83b5-1c0a4a1e32af".to_string(),
        url: "https://github.com/sevenc-nanashi/cantari".to_string(),
        icon,
        default_sampling_rate: 48000,
        terms_of_service: "音源の規約に従って下さい。".to_string(),
        update_infos: vec![UpdateInfo {
            version: "0.1.0".to_string(),
            descriptions: vec!["初期リリース".to_string()],
            contributors: vec!["sevenc-nanashi".to_string()],
        }],
        dependency_licenses,
        supported_features: SupportedFeatures {
            adjust_mora_pitch: true,
            adjust_phoneme_length: true,
            adjust_speed_scale: true,
            adjust_pitch_scale: true,
            adjust_intonation_scale: true,
            adjust_volume_scale: true,
            interrogative_upspeak: false,
            synthesis_morphing: false,
            manage_library: false,
        },
    })
}

pub async fn get_supported_devices() -> Json<SupportedDeveices> {
    Json(SupportedDeveices {
        cpu: true,
        cuda: false,
        dml: false,
    })
}
