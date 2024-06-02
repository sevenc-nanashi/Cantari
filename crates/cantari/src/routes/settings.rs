use crate::error::Result;
use crate::ongen::{setup_ongen, ONGEN};
use crate::settings::{load_settings, write_settings};
use anyhow::anyhow;
use assets::settings_html;
use axum::extract::Path;
use axum::{response::Html, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

static DATA_START: &str = r#"<script id="{}" type="application/json">"#;
static DATA_END: &str = r#"</script>"#;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FrontendOngen {
    name: String,
}

fn replace_data(html: &str, id: &str, data: &str) -> String {
    let data_start = DATA_START.replace("{}", id);
    let start_index = html.find(&data_start).unwrap();
    let end_index = html[start_index..].find(DATA_END).unwrap() + start_index;

    let mut new_html = html[..start_index + data_start.len()].to_string();
    new_html.push_str(data);
    new_html.push_str(&html[end_index..]);

    new_html
}

pub async fn get_settings() -> Html<String> {
    let html = tokio::fs::read_to_string(settings_html()).await.unwrap();

    let settings = load_settings().await;
    let settings_json = serde_json::to_string(&settings).unwrap();

    let ongens = ONGEN.get().unwrap().read().await;
    let ongens: HashMap<Uuid, FrontendOngen> = ongens
        .iter()
        .map(|(uuid, ongen)| (*uuid, FrontendOngen { name: ongen.name() }))
        .collect();

    let html = replace_data(&html, "settings", &settings_json);
    let html = replace_data(&html, "ongens", &serde_json::to_string(&ongens).unwrap());
    Html(html)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PutSettingsBody {
    paths: Vec<String>,
    ongen_limit: usize,
}

pub async fn put_settings(body: Json<PutSettingsBody>) -> Result<String> {
    let mut settings = load_settings().await;

    settings.paths.clone_from(&body.paths);
    settings.ongen_limit = body.ongen_limit;

    write_settings(&settings).await;

    setup_ongen().await;

    Ok("".to_string())
}

pub async fn get_icon(Path(uuid): Path<String>) -> Result<Vec<u8>> {
    let uuid = Uuid::parse_str(&uuid).map_err(|_| anyhow!("Invalid UUID"))?;
    let ongens = ONGEN.get().unwrap().read().await;
    let ongen = ongens.get(&uuid).ok_or(anyhow!("Ongen not found"))?;

    Ok(ongen.read_image().await.unwrap())
}
