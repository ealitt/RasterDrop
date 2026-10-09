use crate::components::{DropZone, FormatSelector, ProgressPanel, QueueList, SettingsView};
use crate::conversion::coordinator::{BatchEvent, BatchHandle, ConversionCoordinator};
use crate::conversion::worker::ConversionRequest;
use crate::models::{ConversionJob, JobStatus, OutputLocation};
use crate::platform::{downloads_directory, reveal_path};
use crate::settings::{AppSettings, SettingsStore};
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, warn};

const CSS: &str = include_str!("../assets/main.css");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProgressView {
    processed: usize,
    total: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    cancelled: usize,
    percentage: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CompletionSummary {
    completed: usize,
    skipped: usize,
    failed: usize,
    cancelled: usize,
}

#[component]
pub fn App() -> Element {
    let mut settings = use_signal(SettingsStore::load);
    let mut show_settings = use_signal(|| false);
    let mut drag_active = use_signal(|| false);
    let mut jobs = use_signal(Vec::<ConversionJob>::new);
    let active = use_signal(|| false);
    let progress = use_signal(ProgressView::default);
    let mut completion = use_signal(|| None::<CompletionSummary>);
    let batch_handle = use_signal(|| None::<Arc<BatchHandle>>);
    let latest_output = use_signal(|| None::<PathBuf>);
    let mut pending_paths = use_signal(Vec::<PathBuf>::new);
    let mut notice = use_signal(|| None::<String>);

    let settings_value = settings.read().clone();
    let output_description = match &settings_value.output_location {
        OutputLocation::BesideSource => {
            "Converted images will be saved next to the originals.".into()
        }
        OutputLocation::FixedDirectory(path) => {
            format!("Converted images will be saved to: {}", path.display())
        }
    };
    let settings_error = validate_fixed_directory(&settings_value);

    let mut add_files = move |paths: Vec<PathBuf>| {
        if paths.is_empty() {
            return;
        }
        if *active.read() {
            let count = paths.len();
            pending_paths.write().extend(paths);
            notice.set(Some(format!(
                "{count} image(s) will start after the current batch."
            )));
            return;
        }
        launch_batch(
            paths,
            settings.read().clone(),
            jobs,
            active,
            progress,
            completion,
            batch_handle,
            latest_output,
            pending_paths,
            notice,
        );
    };

    rsx! {
        style { dangerous_inner_html: CSS }
        main { class: "app-shell",
            header { class: "app-header",
                div { class: "brand",
                    div { class: "brand-mark", aria_hidden: "true", "▧" }
                    div { h1 { "RasterDrop" } p { "Fast, private, and entirely on your device" } }
                }
                button {
                    class: "icon-button",
                    aria_label: "Open settings",
                    title: "Settings",
                    onclick: move |_| show_settings.set(true),
                    "⚙"
                }
            }

            if *show_settings.read() {
                SettingsView {
                    settings: settings_value.clone(),
                    validation_error: settings_error,
                    on_update: move |mut next: AppSettings| {
                        next.sanitize();
                        if let Err(error) = SettingsStore::save(&next) { error!(%error, "failed to save settings"); }
                        settings.set(next);
                    },
                    on_choose_folder: move |_| {
                        spawn(async move {
                            if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                                let mut next = settings.read().clone();
                                next.output_location = OutputLocation::FixedDirectory(folder.path().to_path_buf());
                                if let Err(error) = SettingsStore::save(&next) { error!(%error, "failed to save settings"); }
                                settings.set(next);
                            }
                        });
                    },
                    on_use_downloads: move |_| {
                        if let Some(path) = downloads_directory() {
                            let mut next = settings.read().clone();
                            next.output_location = OutputLocation::FixedDirectory(path);
                            if let Err(error) = SettingsStore::save(&next) { error!(%error, "failed to save settings"); }
                            settings.set(next);
                        } else {
                            notice.set(Some("The system Downloads folder could not be located.".into()));
                        }
                    },
                    on_close: move |_| show_settings.set(false),
                }
            } else {
                section { class: "format-bar",
                    FormatSelector {
                        selected: settings_value.output_format,
                        disabled: *active.read(),
                        on_change: move |format| {
                            let mut next = settings.read().clone();
                            next.output_format = format;
                            if let Err(error) = SettingsStore::save(&next) { error!(%error, "failed to save settings"); }
                            settings.set(next);
                        }
                    }
                    p { class: "output-description", "{output_description}" }
                }

                DropZone {
                    active: *drag_active.read(),
                    on_drag_state: move |value| drag_active.set(value),
                    on_files: add_files,
                    on_pick: move |_| {
                        spawn(async move {
                            let files = rfd::AsyncFileDialog::new()
                                .add_filter("Images", &["heic", "heif", "jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "avif"])
                                .pick_files()
                                .await;
                            if let Some(files) = files {
                                add_files(files.into_iter().map(|file| file.path().to_path_buf()).collect());
                            }
                        });
                    }
                }

                if let Some(message) = notice.read().as_ref() {
                    p { class: "notice", role: "status", "{message}" }
                }

                if *active.read() {
                    ProgressPanel {
                        processed: progress.read().processed,
                        total: progress.read().total,
                        completed: progress.read().completed,
                        skipped: progress.read().skipped,
                        failed: progress.read().failed,
                        cancelled: progress.read().cancelled,
                        percentage: progress.read().percentage,
                        on_cancel: move |_| {
                            if let Some(handle) = batch_handle.read().as_ref() { handle.cancel(); }
                        }
                    }
                } else if let Some(summary) = *completion.read() {
                    section { class: "completion", aria_live: "polite",
                        div { h2 { "Conversion complete" } p { "✓ {summary.completed} converted  ·  ↷ {summary.skipped} skipped  ·  ! {summary.failed} failed  ·  × {summary.cancelled} cancelled" } }
                        div { class: "completion-actions",
                            button {
                                disabled: latest_output.read().is_none(),
                                onclick: move |_| {
                                    if let Some(path) = latest_output.read().as_ref()
                                        && let Err(error) = reveal_path(path)
                                    {
                                        warn!(%error, "failed to reveal output");
                                    }
                                },
                                "Reveal Most Recent"
                            }
                            button {
                                disabled: !jobs.read().iter().any(|job| matches!(job.status, JobStatus::Failed { .. })),
                                onclick: move |_| {
                                    let paths = jobs.read().iter().filter(|job| matches!(job.status, JobStatus::Failed { .. })).map(|job| job.source.clone()).collect();
                                    add_files(paths);
                                },
                                "Retry Failed"
                            }
                            button { onclick: move |_| { jobs.write().clear(); completion.set(None); }, "Clear List" }
                        }
                    }
                }
                QueueList { jobs: jobs.read().clone() }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn launch_batch(
    paths: Vec<PathBuf>,
    settings: AppSettings,
    mut jobs: Signal<Vec<ConversionJob>>,
    mut active: Signal<bool>,
    mut progress: Signal<ProgressView>,
    mut completion: Signal<Option<CompletionSummary>>,
    mut batch_handle: Signal<Option<Arc<BatchHandle>>>,
    mut latest_output: Signal<Option<PathBuf>>,
    mut pending_paths: Signal<Vec<PathBuf>>,
    mut notice: Signal<Option<String>>,
) {
    let requests = paths
        .into_iter()
        .map(|source| ConversionRequest {
            source,
            output_format: settings.output_format,
            output_location: settings.output_location.clone(),
            collision_policy: settings.collision_policy,
            jpeg_quality: settings.jpeg_quality,
        })
        .collect();
    let workers = ConversionCoordinator::recommended_worker_count(num_cpus::get());
    let handle = match ConversionCoordinator::start(requests, workers) {
        Ok(handle) => Arc::new(handle),
        Err(error) => {
            notice.set(Some(format!("Could not start conversion: {error}")));
            return;
        }
    };
    let events = handle.events.clone();
    jobs.write().clear();
    progress.set(ProgressView::default());
    completion.set(None);
    active.set(true);
    notice.set(None);
    batch_handle.set(Some(handle));

    spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                BatchEvent::JobAdded(job) => jobs.write().push(job),
                BatchEvent::JobUpdated { id, status } => {
                    if let JobStatus::Completed { output } = &status {
                        latest_output.set(Some(output.clone()));
                    }
                    if let Some(job) = jobs.write().iter_mut().find(|job| job.id == id) {
                        job.status = status;
                    }
                }
                BatchEvent::Progress {
                    processed,
                    total,
                    completed,
                    skipped,
                    failed,
                    cancelled,
                    percentage,
                } => {
                    progress.set(ProgressView {
                        processed,
                        total,
                        completed,
                        skipped,
                        failed,
                        cancelled,
                        percentage,
                    });
                }
                BatchEvent::Completed {
                    completed,
                    skipped,
                    failed,
                    cancelled,
                } => {
                    completion.set(Some(CompletionSummary {
                        completed,
                        skipped,
                        failed,
                        cancelled,
                    }));
                    active.set(false);
                    batch_handle.set(None);
                    let pending = std::mem::take(&mut *pending_paths.write());
                    if !pending.is_empty() {
                        launch_batch(
                            pending,
                            settings.clone(),
                            jobs,
                            active,
                            progress,
                            completion,
                            batch_handle,
                            latest_output,
                            pending_paths,
                            notice,
                        );
                    }
                    break;
                }
            }
        }
    });
}

fn validate_fixed_directory(settings: &AppSettings) -> Option<String> {
    match &settings.output_location {
        OutputLocation::FixedDirectory(path) if !path.is_dir() => Some(format!(
            "The selected folder does not exist or is inaccessible: {}",
            path.display()
        )),
        _ => None,
    }
}
