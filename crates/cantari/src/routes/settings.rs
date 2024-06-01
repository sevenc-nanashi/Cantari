use crate::error::Result;
use crate::ongen::setup_ongen;
use crate::settings::{load_settings, write_settings};
use assets::settings_html;
use axum::{response::Html, Json};
use serde::{Deserialize, Serialize};

static SETTINGS_START: &str = r#"<script id="settings" type="application/json">"#;
static SETTINGS_END: &str = r#"</script>"#;

pub async fn get_settings() -> Html<String> {
    let html = tokio::fs::read_to_string(settings_html()).await.unwrap();

    let settings = load_settings().await;

    let settings_json = serde_json::to_string(&settings).unwrap();

    let settings_start_index = html.find(SETTINGS_START).unwrap();
    let settings_end_index = html.find(SETTINGS_END).unwrap();

    let mut new_html = html[..settings_start_index + SETTINGS_START.len()].to_string();
    new_html.push_str(&settings_json);
    new_html.push_str(&html[settings_end_index..]);

    Html(new_html)
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
