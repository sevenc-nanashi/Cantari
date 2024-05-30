use crate::error::Result;
use crate::ongen::setup_ongen;
use crate::settings::{load_settings, write_settings};
use assets::settings_html;
use axum::{response::Html, Json};
use serde::{Deserialize, Serialize};

pub async fn get_settings() -> Html<String> {
    let html = tokio::fs::read_to_string(settings_html()).await.unwrap();

    let settings = load_settings().await;

    Html(html.replace(
        r#"{"paths": ["/dummy/path"]}"#,
        &serde_json::to_string(&settings).unwrap(),
    ))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PutSettingsBody {
    paths: Vec<String>,
}

pub async fn put_settings(body: Json<PutSettingsBody>) -> Result<String> {
    let mut settings = load_settings().await;

    settings.paths.clone_from(&body.paths);

    write_settings(settings).await;

    setup_ongen().await;

    Ok("".to_string())
}
