use crate::conversion::errors::ConversionError;
use crate::conversion::worker::{ConversionOutcome, ConversionRequest, convert_file};
use crate::models::{ConversionJob, JobId, JobStatus};
use async_channel::{Receiver, Sender};
use parking_lot::Mutex;
use rayon::ThreadPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub enum BatchEvent {
    JobAdded(ConversionJob),
    JobUpdated {
        id: JobId,
        status: JobStatus,
    },
    Progress {
        processed: usize,
        total: usize,
        completed: usize,
        skipped: usize,
        failed: usize,
        cancelled: usize,
        percentage: u8,
    },
    Completed {
        completed: usize,
        skipped: usize,
        failed: usize,
        cancelled: usize,
    },
}

#[derive(Default)]
struct ProgressState {
    processed: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    cancelled: usize,
}

pub struct BatchHandle {
    pub events: Receiver<BatchEvent>,
    cancelled: Arc<AtomicBool>,
    _pool: Arc<ThreadPool>,
}

impl BatchHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

pub struct ConversionCoordinator;

impl ConversionCoordinator {
    pub fn recommended_worker_count(logical_cpu_count: usize) -> usize {
        logical_cpu_count.saturating_sub(1).clamp(1, 8)
    }

    pub fn start(
        requests: Vec<ConversionRequest>,
        worker_count: usize,
    ) -> Result<BatchHandle, ConversionError> {
        let pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(worker_count.clamp(1, 8))
                .thread_name(|index| format!("rasterdrop-{index}"))
                .build()
                .map_err(|error| ConversionError::Io(std::io::Error::other(error)))?,
        );
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, events) = async_channel::unbounded();
        let total = requests.len();
        let progress = Arc::new(Mutex::new(ProgressState::default()));

        if total == 0 {
            let _ = sender.try_send(BatchEvent::Completed {
                completed: 0,
                skipped: 0,
                failed: 0,
                cancelled: 0,
            });
        }
        for request in requests {
            let job = ConversionJob::new(request.source.clone(), request.output_format);
            let id = job.id;
            let _ = sender.try_send(BatchEvent::JobAdded(job));
            let sender = sender.clone();
            let cancelled = cancelled.clone();
            let progress = progress.clone();
            pool.spawn(move || {
                send_status(&sender, id, JobStatus::Inspecting);
                let status = if cancelled.load(Ordering::Relaxed) {
                    JobStatus::Cancelled
                } else {
                    send_status(&sender, id, JobStatus::Converting);
                    match convert_file(&request, cancelled.clone()) {
                        Ok(ConversionOutcome::Completed(output)) => JobStatus::Completed { output },
                        Ok(ConversionOutcome::Skipped { format, .. }) => JobStatus::Skipped {
                            reason: format!("already {format}"),
                        },
                        Err(ConversionError::Cancelled) => JobStatus::Cancelled,
                        Err(error) => JobStatus::Failed {
                            message: user_message(&error),
                            details: format!("{error:#}"),
                        },
                    }
                };
                send_status(&sender, id, status.clone());
                record_progress(&sender, &progress, status, total);
            });
        }
        drop(sender);
        Ok(BatchHandle {
            events,
            cancelled,
            _pool: pool,
        })
    }
}

fn send_status(sender: &Sender<BatchEvent>, id: JobId, status: JobStatus) {
    let _ = sender.send_blocking(BatchEvent::JobUpdated { id, status });
}

fn record_progress(
    sender: &Sender<BatchEvent>,
    progress: &Mutex<ProgressState>,
    status: JobStatus,
    total: usize,
) {
    let mut progress = progress.lock();
    progress.processed += 1;
    match status {
        JobStatus::Completed { .. } => progress.completed += 1,
        JobStatus::Skipped { .. } => progress.skipped += 1,
        JobStatus::Failed { .. } => progress.failed += 1,
        JobStatus::Cancelled => progress.cancelled += 1,
        _ => {}
    }
    let percentage = ((progress.processed * 100) / total.max(1)) as u8;
    let _ = sender.send_blocking(BatchEvent::Progress {
        processed: progress.processed,
        total,
        completed: progress.completed,
        skipped: progress.skipped,
        failed: progress.failed,
        cancelled: progress.cancelled,
        percentage,
    });
    if progress.processed == total {
        let _ = sender.send_blocking(BatchEvent::Completed {
            completed: progress.completed,
            skipped: progress.skipped,
            failed: progress.failed,
            cancelled: progress.cancelled,
        });
    }
}

fn user_message(error: &ConversionError) -> String {
    match error {
        ConversionError::UnsupportedFormat => "Unsupported or unrecognized image file".into(),
        ConversionError::AnimatedUnsupported => "Animated images are not supported".into(),
        ConversionError::InvalidOutputDirectory(_) => "The output folder is unavailable".into(),
        ConversionError::WouldOverwriteSource => "The original file would be overwritten".into(),
        ConversionError::Cancelled => "Cancelled".into(),
        ConversionError::Heif(_) => "HEIC/HEIF could not be decoded".into(),
        ConversionError::WebP(_) => "WebP could not be encoded".into(),
        ConversionError::Image(_) => "The image data is corrupted or unsupported".into(),
        ConversionError::Io(error) => match error.kind() {
            std::io::ErrorKind::PermissionDenied => "Permission denied".into(),
            std::io::ErrorKind::NotFound => "The source or output folder no longer exists".into(),
            _ => "A file-system error occurred".into(),
        },
    }
}
