use image::DynamicImage;
use rasterdrop::conversion::coordinator::{BatchEvent, ConversionCoordinator};
use rasterdrop::conversion::worker::ConversionRequest;
use rasterdrop::models::{CollisionPolicy, JobStatus, OutputFormat, OutputLocation};
use tempfile::tempdir;

#[test]
fn mixed_batch_continues_after_failure_and_reports_monotonic_progress() {
    let dir = tempdir().unwrap();
    let good = dir.path().join("good.png");
    let bad = dir.path().join("bad.png");
    DynamicImage::new_rgb8(4, 4).save(&good).unwrap();
    std::fs::write(&bad, b"broken").unwrap();
    let make_request = |source| ConversionRequest {
        source,
        output_format: OutputFormat::Jpeg,
        output_location: OutputLocation::BesideSource,
        collision_policy: CollisionPolicy::CreateNumberedFile,
        jpeg_quality: 90,
    };
    let batch =
        ConversionCoordinator::start(vec![make_request(good), make_request(bad)], 2).unwrap();
    let mut progress = Vec::new();
    let mut terminal = Vec::new();
    while let Ok(event) = batch.events.recv_blocking() {
        match event {
            BatchEvent::JobUpdated { status, .. } if status.is_terminal() => terminal.push(status),
            BatchEvent::Progress {
                processed,
                total,
                percentage,
                ..
            } => {
                progress.push((processed, total, percentage));
            }
            BatchEvent::Completed { .. } => break,
            _ => {}
        }
    }
    assert_eq!(terminal.len(), 2);
    assert!(
        terminal
            .iter()
            .any(|status| matches!(status, JobStatus::Completed { .. }))
    );
    assert!(
        terminal
            .iter()
            .any(|status| matches!(status, JobStatus::Failed { .. }))
    );
    assert!(progress.windows(2).all(|pair| pair[0].2 <= pair[1].2));
    assert_eq!(progress.last().copied(), Some((2, 2, 100)));
}

#[test]
fn worker_count_is_bounded() {
    assert_eq!(ConversionCoordinator::recommended_worker_count(1), 1);
    assert_eq!(ConversionCoordinator::recommended_worker_count(2), 1);
    assert_eq!(ConversionCoordinator::recommended_worker_count(16), 8);
}
