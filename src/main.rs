use anyhow::Result;
use axum::{extract::Path, routing::get};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use json5;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::env;
use std::fs;
use std::str::FromStr;
use tokio::{signal, sync::mpsc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{debug, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::{self, time::LocalTime};
use types::AEState;

mod models;
mod routes;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config: Value = json5::from_str(&fs::read_to_string("config.json5")?)?;

    let log_level = &config["log_level"].as_str().unwrap_or("debug").to_string();
    let logger = fmt::fmt()
        .with_env_filter(format!(
            "new_ae_server={0},tower_http={0},axum::rejection={0},sqlx={0}",
            log_level
        ))
        .with_timer(LocalTime::rfc_3339());
    if config["env"].as_str().unwrap_or("dev") == "prod" {
        logger
            .with_writer(
                RollingFileAppender::builder()
                    .rotation(Rotation::DAILY)
                    .max_log_files(config["max_log_files"].as_u64().unwrap_or(7) as usize)
                    .filename_prefix(config["log_file"].as_str().unwrap_or("ae.log"))
                    .build(config["log_dir"].as_str().unwrap_or("log"))?,
            )
            .with_ansi(false)
            .init();
    } else {
        logger.with_writer(std::io::stdout).init();
    }

    let tmp_dir = env::temp_dir().join(config["tmp_dir"].as_str().unwrap_or("new_ae_server"));
    debug!("tmp directory path: {}", &tmp_dir.display());
    if tmp_dir.exists() {
        debug!("remove tmp directory");
        fs::remove_dir_all(&tmp_dir)?;
    }
    debug!("recreate tmp directory");
    fs::create_dir(&tmp_dir)?;
    config["settings"]["TMP_DIR"] = tmp_dir.to_str().unwrap().into();

    let tls_config = RustlsConfig::from_pem_file(
        config["pems"]["cert"].as_str().unwrap_or("pems/cert.pem"),
        config["pems"]["key"].as_str().unwrap_or("pems/key.pem"),
    )
    .await?;

    let listen = config["listen"].as_str().unwrap_or("127.0.0.1:5499");
    let db_pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            SqliteConnectOptions::from_str(config["db_url"].as_str().unwrap_or("ae.db"))?
                .with_regexp(),
        )
        .await?;
    let state = AEState {
        db_pool: db_pool.clone(),
        settings: config["settings"].clone(),
    };
    let (tx, mut rx) = mpsc::channel::<u64>(1);
    let public_dir = config["public_dir"]
        .as_str()
        .unwrap_or("public")
        .trim_matches('/');
    let spa_fallback = ServeDir::new(public_dir)
        .not_found_service(ServeFile::new(public_dir.to_string() + "/index.html"));
    let app = routes::router(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                .layer(CorsLayer::very_permissive()), //.layer(DefaultBodyLimit::max(1024)) //默认2MB,够用
        )
        .route(
            "/stop/:sec",
            get(|Path(sec): Path<u64>| async move {
                tx.send(sec).await.unwrap();
                tx.closed().await;
                format!("server will stopped after {sec} seconds")
            }),
        )
        //.nest_service("/public", spa_fallback.clone())
        .fallback_service(spa_fallback);
    let handle = Handle::new();
    let stopper = handle.clone();
    tokio::spawn(async move {
        tokio::select! {
            sec = rx.recv() => {
                db_pool.close().await;
                rx.close();
                let sec = sec.unwrap_or(10);
                info!("server will stopped after {sec} seconds");
                stopper.graceful_shutdown(Some(Duration::from_secs(sec)));
            },
            _ = signal::ctrl_c() => {
                db_pool.close().await;
                info!("Shutdown signal received, shutting down...");
                stopper.graceful_shutdown(Some(Duration::from_secs(10)));
            },
        }
    });

    info!("Starting server at {}", listen);
    axum_server::bind_rustls(listen.parse()?, tls_config)
        .handle(handle.clone())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
