use rasterdrop::conversion::naming::{reserve_destination, resolve_output_directory};
use rasterdrop::models::{CollisionPolicy, OutputFormat, OutputLocation};
use std::sync::{Arc, Barrier};
use tempfile::tempdir;

#[test]
fn replaces_only_the_final_extension_and_preserves_unicode() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("旅行.project.final.v3.png");
    std::fs::write(&source, b"source").unwrap();
    let reserved = reserve_destination(
        &source,
        dir.path(),
        OutputFormat::Jpeg,
        CollisionPolicy::CreateNumberedFile,
    )
    .unwrap();
    assert_eq!(
        reserved.final_path.file_name().unwrap(),
        "旅行.project.final.v3.jpg"
    );
}

#[test]
fn existing_files_get_numbered_names() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("photo.png");
    std::fs::write(&source, b"source").unwrap();
    std::fs::write(dir.path().join("photo.jpg"), b"existing").unwrap();
    std::fs::write(dir.path().join("photo (1).jpg"), b"existing").unwrap();
    let reserved = reserve_destination(
        &source,
        dir.path(),
        OutputFormat::Jpeg,
        CollisionPolicy::CreateNumberedFile,
    )
    .unwrap();
    assert_eq!(reserved.final_path.file_name().unwrap(), "photo (2).jpg");
}

#[test]
fn concurrent_reservations_are_unique() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("photo.png");
    std::fs::write(&source, b"source").unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let barrier = barrier.clone();
            let source = source.clone();
            let output = dir.path().to_path_buf();
            std::thread::spawn(move || {
                barrier.wait();
                reserve_destination(
                    &source,
                    &output,
                    OutputFormat::Jpeg,
                    CollisionPolicy::CreateNumberedFile,
                )
                .unwrap()
            })
        })
        .collect();
    let reservations: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    let mut names: Vec<_> = reservations
        .iter()
        .map(|reservation| reservation.final_path.clone())
        .collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 8);
}

#[test]
fn output_directory_follows_mode() {
    let dir = tempdir().unwrap();
    let source_dir = tempdir().unwrap();
    let source = source_dir.path().join("photo.png");
    assert_eq!(
        resolve_output_directory(&source, &OutputLocation::BesideSource).unwrap(),
        source_dir.path()
    );
    assert_eq!(
        resolve_output_directory(&source, &OutputLocation::FixedDirectory(dir.path().into()))
            .unwrap(),
        dir.path()
    );
}
