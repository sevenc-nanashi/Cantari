use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code, clippy::enum_variant_names)]
pub enum Error {
    #[error("キャラクターの取得に失敗しました")]
    GetCharacterFailed(#[from] anyhow::Error),
    #[error("キャラクターが見つかりませんでした")]
    CharacterNotFound,
    #[error("Voicevox Coreの初期化に失敗しました")]
    VoicevoxCoreInitializeFailed(#[source] anyhow::Error),
    #[error("推論に失敗しました")]
    InferenceFailed(#[source] anyhow::Error),
    #[error("設定をパースできませんでした")]
    SettingsParseFailed(#[source] anyhow::Error),
    #[error("辞書を書き込めませんでした")]
    WriteDictionaryFailed(#[source] tokio::io::Error),
    #[error("画像を読み込めませんでした")]
    ReadImageFailed(#[source] anyhow::Error),
    #[error("辞書を読み込めませんでした")]
    ReadDictionaryFailed(#[source] anyhow::Error),
    #[error("辞書の操作に失敗しました")]
    DictionaryOperationFailed(#[source] anyhow::Error),
    #[error("解析中にエラーが発生しました")]
    AnalyzeFailed(#[source] anyhow::Error),
    #[error("音声合成中にエラーが発生しました")]
    SynthesisFailed(#[source] anyhow::Error),
    #[error("話者が見つかりませんでした")]
    SpeakerNotFound,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(&ErrorResponse {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}
