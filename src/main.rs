#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use dioxus_desktop::{Config, WindowBuilder};
use rasterdrop::app::App;
use std::path::PathBuf;

fn main() {
    let _log_guard = initialize_logging();
    let window = WindowBuilder::new()
        .with_title("RasterDrop")
        .with_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(720.0, 760.0))
        .with_min_inner_size(dioxus_desktop::tao::dpi::LogicalSize::new(540.0, 640.0));
    dioxus_desktop::launch::launch(
        App,
        Vec::new(),
        vec![Box::new(Config::new().with_window(window))],
    );
}

fn initialize_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let directory = directories::ProjectDirs::from("com", "REPLACE_ME", "RasterDrop")
        .map(|dirs| dirs.data_local_dir().join("logs"))
        .unwrap_or_else(|| PathBuf::from("."));
    if std::fs::create_dir_all(&directory).is_err() {
        return None;
    }
    let appender = tracing_appender::rolling::daily(directory, "rasterdrop.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("rasterdrop=info,warn"));
    let subscriber = tracing_subscriber::registry().with(filter).with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(writer),
    );
    if subscriber.try_init().is_ok() {
        Some(guard)
    } else {
        None
    }
}
