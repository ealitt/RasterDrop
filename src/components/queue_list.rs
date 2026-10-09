use crate::models::{ConversionJob, JobStatus};
use dioxus::prelude::*;

fn status_label(status: &JobStatus) -> (&'static str, String, &'static str) {
    match status {
        JobStatus::Queued => ("○", "Queued".into(), "queued"),
        JobStatus::Inspecting => ("⌕", "Inspecting…".into(), "working"),
        JobStatus::Converting => ("↻", "Converting…".into(), "working"),
        JobStatus::Completed { .. } => ("✓", "Completed".into(), "success"),
        JobStatus::Skipped { reason } => ("↷", format!("Skipped — {reason}"), "skipped"),
        JobStatus::Failed { message, .. } => ("!", format!("Failed — {message}"), "failed"),
        JobStatus::Cancelled => ("×", "Cancelled".into(), "cancelled"),
    }
}

#[component]
pub fn QueueList(jobs: Vec<ConversionJob>) -> Element {
    if jobs.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "queue", aria_label: "Conversion queue",
            h2 { "Recent batch" }
            div { class: "queue-list",
                for job in jobs {
                    {
                        let (icon, text, class) = status_label(&job.status);
                        let name = job.source.file_name().map(|value| value.to_string_lossy().into_owned()).unwrap_or_else(|| job.source.display().to_string());
                        let details = match &job.status { JobStatus::Failed { details, .. } => Some(details.clone()), _ => None };
                        rsx! {
                            div { class: "queue-row", key: "{job.id}",
                                span { class: "file-name", title: "{job.source.display()}", "{name}" }
                                span { class: "status {class}", span { aria_hidden: "true", "{icon}" } " {text}" }
                                if let Some(details) = details {
                                    details { summary { "Technical details" } pre { "{details}" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
