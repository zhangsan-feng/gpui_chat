use gpui::*;
use log::info;
use std::borrow::Cow;

use std::sync::Arc;
mod application;
mod domain;
mod infrastructure;
mod state;
mod ui;
use reqwest_client::ReqwestClient;
use rust_embed::RustEmbed;
use std::path::{Path, PathBuf};

pub fn logger_init(log_dir: impl AsRef<Path>, date_format: &str) {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir).expect("create log directory failed");

    let log_file = log_dir.join(format!(
        "client_{}.log",
        chrono::Local::now().format(date_format)
    ));

    fern::Dispatch::new()
        .format(|out, message, record| {
            let file = record.file().unwrap_or("<unknown>");
            let line = record.line().unwrap_or(0);
            out.finish(format_args!(
                "[{}] [{}] [{}] [{}:{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                file,
                line,
                message
            ))
        })
        // .filter(|metadata| {
        //     metadata.level() == Level::Info && !metadata.target().starts_with("symphonia")
        // })
        .level(log::LevelFilter::Info)
        // .level_for("gstreamer", log::LevelFilter::Debug)
        // .level(log::LevelFilter::Debug)
        // .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .chain(fern::log_file(&log_file).expect("open log file failed"))
        .apply()
        .expect("init logger failed");

    info!("init logger success: {}", log_file.display());
}

#[derive(RustEmbed)]
#[folder = "./src/icon"]
struct AssetFiles;

struct MergedAssets {
    local_directories: Vec<PathBuf>,
    component_assets: gpui_component_assets::Assets,
}

impl AssetSource for MergedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        for dir in &self.local_directories {
            let full_path = dir.join(path);
            if full_path.exists() {
                let bytes = std::fs::read(full_path)?;
                return Ok(Some(Cow::Owned(bytes)));
            }
        }

        let clean_path = path.trim_start_matches("icon/").trim_start_matches("/");
        if let Some(file) = AssetFiles::get(clean_path) {
            return Ok(Some(file.data));
        }

        self.component_assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all_files = std::collections::HashSet::new();

        for dir in &self.local_directories {
            let full_path = dir.join(path);
            if full_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(full_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            all_files.insert(name.to_string());
                        }
                    }
                }
            }
        }

        let clean_path = path.trim_start_matches("icon/").trim_start_matches("/");
        for file_path in AssetFiles::iter() {
            if file_path.starts_with(clean_path) {
                all_files.insert(file_path.to_string());
            }
        }

        for file_path in self.component_assets.list(path)? {
            all_files.insert(file_path.to_string());
        }

        Ok(all_files.into_iter().map(SharedString::from).collect())
    }
}

fn main() {
    let project_directory = project_directory().expect("resolve project directory failed");
    logger_init(project_directory.join("logs").join("client"), "%Y-%m-%d");
    if let Err(error) = infrastructure::session_store::SessionStore::initialize(project_directory) {
        log::error!("failed to initialize session database: {}", error);
    }

    let http_client = ReqwestClient::user_agent("gpui").unwrap();
    let assets = MergedAssets {
        local_directories: vec![PathBuf::from("/"), PathBuf::from("./src/icon")],
        component_assets: gpui_component_assets::Assets,
    };

    let app = gpui_platform::application()
        .with_http_client(Arc::new(http_client))
        .with_assets(assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    app.run(move |cx| {
        gpui_component::init(cx);
        gpui_tokio::init(cx);
        state::new_state(cx);

        let shutdown_state = cx.global::<state::GlobalState>().0.clone();
        let _ = cx.on_app_quit(move |app| {
            let _ = shutdown_state.update(app, |state, _| state.stop_ws());
            async {}
        });

        ui::login::open(cx).expect("failed to open login window");
    });
}

fn project_directory() -> anyhow::Result<PathBuf> {
    let current_directory = std::env::current_dir()?;
    if current_directory.join("server").is_dir() && current_directory.join("gui").is_dir() {
        return Ok(current_directory);
    }

    if current_directory
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("gui"))
    {
        return current_directory
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("gui directory has no project parent"));
    }

    Ok(current_directory)
}
