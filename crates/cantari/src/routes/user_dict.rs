use super::audio_query::{get_or_initialize_open_jtalk, OPEN_JTALK};
use voicevox_core::{tokio::UserDict, UserDictWord, UserDictWordType};

use axum::extract::{Path, Query};
use axum::response::Json;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, OnceCell};
use tracing::{info, warn};

use crate::error::{Error, Result};

pub static USER_DICT: OnceCell<Arc<Mutex<UserDict>>> = OnceCell::const_new();
pub async fn get_or_initialize_user_dict() -> Arc<Mutex<UserDict>> {
    if let Some(user_dict) = USER_DICT.get() {
        return user_dict.clone();
    }
    info!("Initializing user dict...");
    let user_dict = UserDict::new();

    if tokio::fs::metadata(&*USER_DICT_PATH).await.is_ok() {
        info!("Loading user dict from {:?}", &*USER_DICT_PATH);
        user_dict.load(&USER_DICT_PATH).await.unwrap();
    } else {
        info!("User dict not found at {:?}", &*USER_DICT_PATH);
    }

    let open_jtalk = get_or_initialize_open_jtalk().await;
    open_jtalk.use_user_dict(&user_dict).await.unwrap();

    info!("User dict initialized");
    Arc::new(Mutex::new(user_dict))
}

pub static USER_DICT_PATH: Lazy<String> = Lazy::new(|| {
    process_path::get_executable_path()
        .unwrap()
        .parent()
        .unwrap()
        .join("user_dict.json")
        .to_str()
        .unwrap()
        .to_string()
});

#[derive(Debug, Serialize, Deserialize)]
pub struct VvUserDictWord {
    priority: u32,
    accent_type: usize,
    mora_count: usize,
    surface: String,
    pronunciation: String,
    #[serde(skip_deserializing)]
    part_of_speech_detail_1: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VvUserDictWordParam {
    priority: u32,
    accent_type: usize,
    surface: String,
    pronunciation: String,
}

impl From<VvUserDictWord> for UserDictWord {
    fn from(word: VvUserDictWord) -> UserDictWord {
        UserDictWord::new(
            &word.surface[..],
            word.pronunciation,
            word.accent_type,
            match &word.part_of_speech_detail_1[..] {
                "一般名詞" => UserDictWordType::CommonNoun,
                "固有名詞" => UserDictWordType::ProperNoun,
                "動詞" => UserDictWordType::Verb,
                "形容詞" => UserDictWordType::Adjective,
                "語尾" => UserDictWordType::Suffix,
                "" => UserDictWordType::ProperNoun,
                _ => {
                    warn!("Unknown word type: {}", &word.part_of_speech_detail_1);
                    UserDictWordType::CommonNoun
                }
            },
            word.priority,
        )
        .unwrap()
    }
}

impl From<UserDictWord> for VvUserDictWord {
    fn from(word: UserDictWord) -> VvUserDictWord {
        VvUserDictWord {
            priority: *word.priority(),
            accent_type: *word.accent_type(),
            mora_count: *word.mora_count(),
            surface: word.surface().to_string(),
            pronunciation: word.pronunciation().to_string(),
            part_of_speech_detail_1: match word.word_type() {
                UserDictWordType::CommonNoun => "一般名詞",
                UserDictWordType::ProperNoun => "固有名詞",
                UserDictWordType::Verb => "動詞",
                UserDictWordType::Adjective => "形容詞",
                UserDictWordType::Suffix => "語尾",
            }
            .to_string(),
        }
    }
}

impl From<VvUserDictWordParam> for UserDictWord {
    fn from(word: VvUserDictWordParam) -> UserDictWord {
        UserDictWord::new(
            &word.surface[..],
            word.pronunciation,
            word.accent_type,
            UserDictWordType::CommonNoun,
            word.priority,
        )
        .unwrap()
    }
}

pub async fn get_user_dict() -> Json<HashMap<String, VvUserDictWord>> {
    let user_dict = get_or_initialize_user_dict().await;
    let user_dict = user_dict.lock().await;

    let mut result = HashMap::new();
    user_dict.with_words(|words| {
        for (key, value) in words.iter() {
            result.insert(key.to_string(), value.clone().into());
        }
    });

    Json(result)
}

pub async fn import_user_dict(Json(payload): Json<HashMap<String, VvUserDictWord>>) -> Result<()> {
    let user_dict = get_or_initialize_user_dict().await;
    let user_dict = user_dict.lock().await;

    let temp_file =
        tempfile::NamedTempFile::new().map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    let temp_file_writer = std::io::BufWriter::new(temp_file.as_file());

    let converted: HashMap<String, UserDictWord> =
        payload.into_iter().map(|(k, v)| (k, v.into())).collect();
    serde_json::to_writer(temp_file_writer, &converted)
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    let temp_file = temp_file.into_temp_path();

    info!("Importing user dict from {:?}", temp_file);

    user_dict
        .load(temp_file.to_str().unwrap())
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    user_dict
        .save(&USER_DICT_PATH)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    OPEN_JTALK
        .get()
        .unwrap()
        .use_user_dict(&user_dict)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    Ok(())
}

pub async fn post_user_dict_word(Query(param): Query<VvUserDictWordParam>) -> Result<String> {
    let user_dict = get_or_initialize_user_dict().await;
    let user_dict = user_dict.lock().await;

    let word: UserDictWord = param.into();

    let word_uuid = user_dict
        .add_word(word)
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    user_dict
        .save(&USER_DICT_PATH)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    get_or_initialize_open_jtalk()
        .await
        .use_user_dict(&user_dict)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    Ok(word_uuid.hyphenated().to_string())
}

pub async fn delete_user_dict_word(Path(word_uuid): Path<String>) -> Result<()> {
    let user_dict = get_or_initialize_user_dict().await;
    let user_dict = user_dict.lock().await;

    let word_uuid = uuid::Uuid::parse_str(&word_uuid)
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    user_dict
        .remove_word(word_uuid)
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    user_dict
        .save(&USER_DICT_PATH)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    get_or_initialize_open_jtalk()
        .await
        .use_user_dict(&user_dict)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    Ok(())
}

pub async fn put_user_dict_word(
    Path(word_uuid): Path<String>,
    Query(payload): Query<VvUserDictWordParam>,
) -> Result<()> {
    let user_dict = get_or_initialize_user_dict().await;
    let user_dict = user_dict.lock().await;

    let word_uuid = uuid::Uuid::parse_str(&word_uuid)
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    let word: UserDictWord = payload.into();

    user_dict
        .update_word(word_uuid, word)
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    user_dict
        .save(&USER_DICT_PATH)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    get_or_initialize_open_jtalk()
        .await
        .use_user_dict(&user_dict)
        .await
        .map_err(|e| Error::DictionaryOperationFailed(e.into()))?;

    Ok(())
}
