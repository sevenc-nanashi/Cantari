use crate::{
    error::Result,
    ongen::{setup_ongen, ONGEN},
    ongen_settings::OngenSettings,
    settings::{load_settings, write_settings},
};
use anyhow::anyhow;
use assets::settings_html;
use axum::{extract::Path, response::Html, Json};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Cursor};
use tracing::{info, info_span};
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
    ongen_settings: HashMap<Uuid, OngenSettings>,
}

pub async fn put_settings(body: Json<PutSettingsBody>) -> Result<String> {
    let mut settings = load_settings().await;
    info!("Updating settings...");

    settings.paths.clone_from(&body.paths);
    settings.ongen_limit = body.ongen_limit;
    let mut ongen_settings = body.ongen_settings.clone();
    for (uuid, ongen_setting) in &mut ongen_settings {
        let span = info_span!("ongen", uuid = %uuid);
        let _guard = span.enter();
        for style_settings in &mut ongen_setting.style_settings {
            let span = info_span!("style", name = %style_settings.name);
            let _guard = span.enter();

            if let Some(icon) = &style_settings.icon {
                info!("Resizing icon...");
                let base_icon = image::load_from_memory(icon).map_err(|e| anyhow!(e))?;
                let small_icon = base_icon.resize(256, 256, image::imageops::FilterType::Lanczos3);
                let mut icon = image::RgbaImage::new(256, 256);
                image::imageops::overlay(
                    &mut icon,
                    &small_icon,
                    ((256 - small_icon.width()) / 2).into(),
                    ((256 - small_icon.height()) / 2).into(),
                );

                let mut icon_buffer = Vec::new();
                icon.write_to(&mut Cursor::new(&mut icon_buffer), image::ImageFormat::Png)
                    .map_err(|e| anyhow!(e))?;
                style_settings.icon = Some(icon_buffer);
                info!(
                    "Resized icon: {}x{} -> {}x{}",
                    base_icon.width(),
                    base_icon.height(),
                    icon.width(),
                    icon.height()
                );
            }
            if let Some(portrait) = &style_settings.portrait {
                info!("Resizing portrait...");
                let base_portrait = image::load_from_memory(portrait).map_err(|e| anyhow!(e))?;
                let portrait =
                    base_portrait.resize(500, 500, image::imageops::FilterType::Lanczos3);
                let mut portrait_buffer = Vec::new();
                portrait
                    .write_to(
                        &mut Cursor::new(&mut portrait_buffer),
                        image::ImageFormat::Png,
                    )
                    .map_err(|e| anyhow!(e))?;
                style_settings.portrait = Some(portrait_buffer);
                info!(
                    "Resized portrait: {}x{} -> {}x{}",
                    base_portrait.width(),
                    base_portrait.height(),
                    portrait.width(),
                    portrait.height()
                );
            }

            info!("Updated style settings");
        }
    }
    settings.ongen_settings = ongen_settings;

    write_settings(&settings).await;

    setup_ongen().await;

    Ok("".to_string())
}

pub async fn get_icon(Path(uuid_png): Path<String>) -> Result<Vec<u8>> {
    let uuid = Uuid::parse_str(
        uuid_png
            .strip_suffix(".png")
            .ok_or(anyhow!("Invalid icon path"))?,
    )
    .map_err(|_| anyhow!("Invalid UUID"))?;
    let ongens = ONGEN.get().unwrap().read().await;
    let ongen = ongens.get(&uuid).ok_or(anyhow!("Ongen not found"))?;

    Ok(ongen.read_image().await.unwrap())
}
