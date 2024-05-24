use crate::aivoice::AIVOICE;
use crate::error::{Error, Result};

use anyhow::anyhow;
use async_zip::base::read::seek::ZipFileReader;
use derive_getters::Getters;
use image::imageops;
use once_cell::sync::Lazy;
use std::io::Cursor;
use std::{collections::HashMap, path::Path, sync::Arc};
use tasklist::{get_proc_path, tasklist};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Getters)]
pub struct IconManager {
    icons: HashMap<String, StyleImages>,
    portraits: HashMap<String, StyleImages>,
}
#[derive(Debug, Getters)]
pub struct StyleImages {
    normal: Vec<u8>,
    joy: Vec<u8>,
    anger: Vec<u8>,
    sorrow: Vec<u8>,
}

impl IconManager {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            portraits: HashMap::new(),
        }
    }

    pub async fn setup(&mut self) -> Result<()> {
        let mut tasks = unsafe { tasklist().into_iter() };
        let Some((_, aivoice_process_id)) = tasks.find(|(task_name, _)| task_name.to_lowercase() == "aivoiceeditor.exe") else {
            return Err(Error::ProcessNotFound);
        };

        info!("A.I.Voice process id: {}", aivoice_process_id);
        let aivoice_process_path = unsafe { get_proc_path(aivoice_process_id) };
        let aivoice_process_path = Path::new(&aivoice_process_path);
        info!("Process path: {}", &aivoice_process_path.display());

        let voice_path = aivoice_process_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Voice");

        info!("Voice path: {}", &voice_path.display());
        for speaker in AIVOICE.lock().await.speakers().values() {
            info!("Extracting icon for {}", speaker.internal_name());
            let images_path = voice_path.join(speaker.internal_name()).join("images.dat");
            info!("Images path: {}", &images_path.display());
            let mut zip_reader = ZipFileReader::with_tokio(
                tokio::fs::File::open(&images_path)
                    .await
                    .map_err(|e| Error::ReadImageFailed(e.into()))?,
            )
            .await
            .map_err(|e| Error::ReadImageFailed(e.into()))?;

            let root = {
                let mut images = zip_reader.file().entries().iter().enumerate();
                let (icon, root) = match images.find(|(_, x)| {
                    let filename = x.entry().filename().as_str().unwrap_or("");

                    filename.ends_with("icon.png")
                }) {
                    Some((icon, entry)) => {
                        let filename = entry
                            .entry()
                            .filename()
                            .as_str()
                            .unwrap_or("")
                            .replace('\\', "/");

                        (icon, filename.replace("icon.png", ""))
                    }
                    None => return Err(Error::ReadImageFailed(anyhow!("Icon not found"))),
                };

                let mut icon = zip_reader
                    .reader_with_entry(icon)
                    .await
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;
                let mut icon_buf = Vec::new();
                icon.read_to_end_checked(&mut icon_buf)
                    .await
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;

                let mut normal_bg =
                    image::ImageBuffer::from_fn(48, 48, |_, _| image::Rgba([255, 255, 255, 128]));
                let mut joy_bg =
                    image::ImageBuffer::from_fn(48, 48, |_, _| image::Rgba([255, 255, 200, 128]));
                let mut anger_bg =
                    image::ImageBuffer::from_fn(48, 48, |_, _| image::Rgba([255, 200, 200, 128]));
                let mut sorrow_bg =
                    image::ImageBuffer::from_fn(48, 48, |_, _| image::Rgba([200, 200, 255, 128]));

                let icon = image::load_from_memory(&icon_buf)
                    .map_err(|e| Error::ReadImageFailed(e.into()))?
                    .resize(48, 48, image::imageops::FilterType::Triangle)
                    .to_rgba8();

                imageops::overlay(&mut normal_bg, &icon, 0, 0);
                imageops::overlay(&mut joy_bg, &icon, 0, 0);
                imageops::overlay(&mut anger_bg, &icon, 0, 0);
                imageops::overlay(&mut sorrow_bg, &icon, 0, 0);

                let mut normal_icon_buf = Vec::new();
                let mut joy_icon_buf = Vec::new();
                let mut anger_icon_buf = Vec::new();
                let mut sorrow_icon_buf = Vec::new();

                normal_bg
                    .write_to(
                        &mut Cursor::new(&mut normal_icon_buf),
                        image::ImageOutputFormat::Png,
                    )
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;
                joy_bg
                    .write_to(
                        &mut Cursor::new(&mut joy_icon_buf),
                        image::ImageOutputFormat::Png,
                    )
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;
                anger_bg
                    .write_to(
                        &mut Cursor::new(&mut anger_icon_buf),
                        image::ImageOutputFormat::Png,
                    )
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;
                sorrow_bg
                    .write_to(
                        &mut Cursor::new(&mut sorrow_icon_buf),
                        image::ImageOutputFormat::Png,
                    )
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;

                self.icons.insert(
                    speaker.internal_name().to_string(),
                    StyleImages {
                        normal: normal_icon_buf,
                        joy: joy_icon_buf,
                        anger: anger_icon_buf,
                        sorrow: sorrow_icon_buf,
                    },
                );

                root
            };

            info!("{} root: {:?}", speaker.internal_name(), root);

            let mut portraits = StyleImages {
                normal: Vec::new(),
                joy: Vec::new(),
                anger: Vec::new(),
                sorrow: Vec::new(),
            };
            for emotion in ['A', 'J', 'N', 'S'].iter() {
                let mut images = zip_reader.file().entries().iter().enumerate();
                let image_index = match images.find(|(_, x)| {
                    let name = x
                        .entry()
                        .filename()
                        .as_str()
                        .unwrap_or("")
                        .replace('\\', "/");

                    name.starts_with(&format!("{}{}/OpenEyes", root, emotion))
                        && name.split('/').last().and_then(|n| n.chars().nth(4)) == Some('0')
                }) {
                    Some((i, _)) => i,
                    None => continue,
                };
                let mut image_entry = match zip_reader.reader_with_entry(image_index).await {
                    Ok(image) => image,
                    Err(_) => continue,
                };

                let final_image_buf = match emotion {
                    'A' => &mut portraits.anger,
                    'J' => &mut portraits.joy,
                    'N' => &mut portraits.normal,
                    'S' => &mut portraits.sorrow,
                    _ => unreachable!(),
                };
                let image_buf = &mut Vec::new();
                image_entry
                    .read_to_end_checked(image_buf)
                    .await
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;

                let portrait = image::load_from_memory(image_buf)
                    .map_err(|e| Error::ReadImageFailed(e.into()))?
                    .resize(500, 500, image::imageops::FilterType::Triangle)
                    .to_rgba8();

                let mut final_image_cursor = std::io::Cursor::new(final_image_buf);
                portrait
                    .write_to(&mut final_image_cursor, image::ImageOutputFormat::Png)
                    .map_err(|e| Error::ReadImageFailed(e.into()))?;
            }
            // 一部キャラはN以外の立ち絵がないので全部チェックはしない
            // for (name, portrait) in [
            //     ("Normal", &portraits.normal),
            //     ("Joy", &portraits.joy),
            //     ("Anger", &portraits.anger),
            //     ("Sorrow", &portraits.sorrow),
            // ]
            // .iter()
            // {
            //     if portrait.is_empty() {
            //         return Err(Error::ReadImageFailed(anyhow!(
            //             "{}の{}の立ち絵が見つかりませんでした。",
            //             speaker.internal_name(),
            //             name,
            //         )));
            //     }
            // }

            self.portraits
                .insert(speaker.internal_name().to_string(), portraits);
        }

        Ok(())
    }
}

pub static ICON_MANAGER: Lazy<Arc<Mutex<IconManager>>> =
    Lazy::new(|| Arc::new(Mutex::new(IconManager::new())));
