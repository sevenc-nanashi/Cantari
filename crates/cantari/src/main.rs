mod error;
mod math;
mod model;
mod ongen;
mod ongen_settings;
mod oto;
mod routes;
mod settings;
mod tempdir;

use crate::{
    routes::{audio_query::get_or_initialize_synthesizer, user_dict::get_or_initialize_user_dict},
    settings::{get_settings_path, load_settings, write_settings},
};
use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post, put},
    Router,
};
use clap::Parser;
use ongen::ONGEN;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace};
use tracing::{info, Level};

use crate::{ongen::setup_ongen, tempdir::TEMPDIR};

#[derive(Debug, Parser)]
#[clap(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    ignore_errors = true
)]
struct Cli {
    /// ポート番号。
    #[clap(short, long, default_value = "50202")]
    port: u16,
    /// ホスト名。
    #[clap(long, default_value = "127.0.0.1")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .with_writer(std::io::stderr)
        .with_ansi(cfg!(debug_assertions))
        .init();

    let result = main_impl(args).await;

    info!("Shutting down...");

    result?;

    Ok(())
}

async fn main_impl(args: Cli) -> Result<()> {
    let app =
        Router::new()
            .route("/", get(get_index))
            .route("/version", get(routes::info::get_version))
            .route("/engine_manifest", get(routes::info::get_engine_manifest))
            .route(
                "/supported_devices",
                get(routes::info::get_supported_devices),
            )
            .route("/speakers", get(routes::speakers::get_speakers))
            .route("/speaker_info", get(routes::speakers::get_speaker_info))
            .route(
                "/is_initialized_speaker",
                get(routes::audio_query::get_is_initialized_speaker),
            )
            .route(
                "/initialize_speaker",
                post(routes::audio_query::post_initialize_speaker),
            )
            .route("/mora_data", post(routes::audio_query::post_mora_data))
            .route("/mora_pitch", post(routes::audio_query::post_mora_pitch))
            .route("/mora_length", post(routes::audio_query::post_mora_length))
            .route("/synthesis", post(routes::synthesis::post_synthesis))
            .route("/user_dict", get(routes::user_dict::get_user_dict))
            .route(
                "/import_user_dict",
                post(routes::user_dict::import_user_dict),
            )
            .route(
                "/user_dict_word",
                post(routes::user_dict::post_user_dict_word),
            )
            .route(
                "/user_dict_word/:word_uuid",
                delete(routes::user_dict::delete_user_dict_word),
            )
            .route(
                "/user_dict_word/:word_uuid",
                put(routes::user_dict::put_user_dict_word),
            )
            .route("/audio_query", post(routes::audio_query::post_audio_query))
            .route(
                "/accent_phrases",
                post(routes::audio_query::post_accent_phrases),
            )
            .route(
                "/settings",
                get(routes::settings::get_settings).put(routes::settings::put_settings),
            )
            .route("/icons/:uuid", get(routes::settings::get_icon))
            .layer(CorsLayer::permissive())
            .layer(
                trace::TraceLayer::new_for_http()
                    .make_span_with(
                        |request: &axum::http::Request<axum::body::Body>| tracing::info_span!("request", path = %request.uri().path()),
                    )
                    .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
            )
            .layer(DefaultBodyLimit::disable());

    let settings_path = get_settings_path();
    let has_settings = settings_path.exists();
    if !has_settings {
        info!("Settings file does not exist: {}", settings_path.display());
        write_settings(&load_settings().await).await;
    }

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    setup_ongen().await;
    {
        let ongens = ONGEN.get().unwrap().read().await;
        for ongen in ongens.values() {
            info!("- {} ({}, {})", ongen.name(), ongen.uuid, ongen.id());
        }
    }

    if TEMPDIR.exists() {
        tokio::fs::remove_dir_all(TEMPDIR.as_path()).await?;
    }
    tokio::fs::create_dir_all(TEMPDIR.as_path()).await?;
    info!("Created tempdir: {}", TEMPDIR.as_path().display());

    get_or_initialize_synthesizer().await;
    get_or_initialize_user_dict().await;

    info!("Starting server...");

    info!("Listening on http://{}", addr);

    if !has_settings {
        info!("Launching browser...");
        open::that(format!("http://{}", addr))?;
    }

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C signal handler");
        })
        .await?;

    tokio::fs::remove_dir_all(TEMPDIR.as_path()).await?;

    Ok(())
}

async fn get_index() -> impl IntoResponse {
    Redirect::permanent("/settings")
}
