use axum::extract::Path;
use axum::response::Json;
use std::collections::HashMap;

use crate::error::Result;

pub async fn get_user_dict() -> Json<HashMap<String, String>> {
    Json(HashMap::new())
}

pub async fn import_user_dict() -> Result<()> {
    Ok(())
}

pub async fn post_user_dict_word() -> Result<String> {
    Ok(uuid::Uuid::new_v4().to_string())
}

pub async fn delete_user_dict_word(Path(_word_uuid): Path<String>) -> Result<()> {
    Ok(())
}

pub async fn put_user_dict_word(Path(_word_uuid): Path<String>) -> Result<()> {
    Ok(())
}
