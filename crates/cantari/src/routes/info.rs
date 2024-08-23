use axum::{response::IntoResponse, Json};
use base64::Engine as _;
use regex_macro::regex;
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
    pub return_resource_url: bool,
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

    let changelog = include_str!("../../../../CHANGELOG.md");
    let versions = changelog
        .lines()
        .filter(|line| line.starts_with("## "))
        .map(|line| line.trim_start_matches("## ").to_string())
        .collect::<Vec<_>>();

    let mut update_infos = vec![];
    let description_pattern = regex!(
        r"- (?P<description>.+) by \[(?P<contributor>.+)\]\(https://github.com/(?P<contributor_in_url>.+)\)"
    );
    for version in versions {
        let lines = changelog
            .lines()
            .skip_while(|line| line != &format!("## {}", version))
            .skip(1)
            .take_while(|line| !line.starts_with("## "))
            .map(|line| line.to_string())
            .filter(|line| line.starts_with("- "))
            .collect::<Vec<_>>();

        assert!(!lines.is_empty());

        let mut descriptions = vec![];
        let mut contributors = vec![];

        for line in lines {
            let captures = description_pattern
                .captures(&line)
                .unwrap_or_else(|| panic!("assertion failed: {:?} does not match", line));
            assert_eq!(
                captures.name("contributor").unwrap().as_str(),
                captures.name("contributor_in_url").unwrap().as_str()
            );

            descriptions.push(captures.name("description").unwrap().as_str().to_string());
            contributors.push(captures.name("contributor").unwrap().as_str().to_string());
        }

        update_infos.push(UpdateInfo {
            version,
            descriptions,
            contributors,
        });
    }

    Json(EngineManifest {
        manifest_version: "0.13.1".to_string(),
        name: "Cantari".to_string(),
        brand_name: "Cantari".to_string(),
        uuid: "a6b5fbf0-4561-43b3-83b5-1c0a4a1e32af".to_string(),
        url: "https://github.com/sevenc-nanashi/cantari".to_string(),
        icon,
        default_sampling_rate: 48000,
        terms_of_service: "音源の規約に従って下さい。".to_string(),
        update_infos,
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
            return_resource_url: true,
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_get_manifest() {
        let manifest = get_engine_manifest().await.into_response();
        assert_eq!(manifest.status(), 200);
    }
}
